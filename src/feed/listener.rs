use std::sync::Arc;

use futures_util::StreamExt;
use tracing::warn;

use crate::{cache, feed::session_manager::SessionManager, matching::{account_book::Account, message::OrderMessage}};

pub async fn start(session_manager: Arc<SessionManager>) -> anyhow::Result<()> {
    let client = cache::get_client();
    let mut pubsub = client.get_async_pubsub().await?;

    pubsub.subscribe("order").await?;
    pubsub.subscribe("account").await?;
    pubsub.subscribe("trade").await?;
    pubsub.subscribe("ticker").await?;
    pubsub.subscribe("l2_batch").await?;

    let mut stream = pubsub.into_on_message();

    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            let channel = msg.get_channel_name();
            let payload: String = match msg.get_payload() {
                Ok(p) => p,
                Err(_) => continue,
            };

            match channel {
                "order" => {
                    if let Ok(m) = serde_json::from_str::<OrderMessage>(&payload) {
                        let chan = format!("{}.{}.order", m.order.user_id, m.order.product_id);
                        session_manager.broadcast(&chan, &m).await;
                    }
                },
                "account" => {
                    if let Ok(m) = serde_json::from_str::<Account>(&payload) {
                        let chan = format!("{}.{}.funds", m.user_id, m.currency);
                        session_manager.broadcast(&chan, &m).await;
                    }
                }
                _ => warn!("{} channel not found", channel),
            }
        }
    });
    Ok(())
}
