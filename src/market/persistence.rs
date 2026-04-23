use mongodb::Database;
use rdkafka::{Message as _, consumer::{Consumer, StreamConsumer}};
use redis::{Commands, Connection};
use tracing::error;

use crate::{cache, market::create_message_consumer, matching::message::{Message, MessageType}, repository::{account_repository, order_repository, trade_repository}};

pub async fn start(group_id: &str, db: Database) -> anyhow::Result<()> {
    let consumer = create_message_consumer(group_id)?;

    let redis_conn = cache::get_client().get_connection()?;

    tokio::spawn(async move {
        if let Err(e) = run_loop(consumer, redis_conn, db).await {
            error!("market consumer start error: {}", e);
        }
    });

    Ok(())
}

async fn run_loop(cosumer: StreamConsumer, mut redis_conn: Connection, db: Database) -> anyhow::Result<()> {
    loop {
        match cosumer.recv().await {
            Err(e) => error!("Kafka error: {}", e),
            Ok(kafka_msg) => {
                let payload = match kafka_msg.payload() {
                    Some(p) => p,
                    None => continue,
                };

                let message = match serde_json::from_slice::<Message>(payload) {
                    Ok(m) => m,
                    Err(e) => {
                        error!("convert json slice to message error: {}", e);
                        continue
                    }
                };

                match message.message_type {
                    MessageType::Order(order_msg) => {
                        let json_load = serde_json::to_string(&order_msg)?;
                        let _: () = redis_conn.publish("order", json_load)?;

                        order_repository::save(db.clone(), order_msg.order.into()).await;

                        cosumer.commit_message(&kafka_msg, rdkafka::consumer::CommitMode::Async)?;
                    },
                    MessageType::Trade(trade) => {
                        let json_load = serde_json::to_string(&trade)?;
                        let _: () = redis_conn.publish("trade", json_load)?;

                        trade_repository::save(db.clone(), trade.into()).await;

                        cosumer.commit_message(&kafka_msg, rdkafka::consumer::CommitMode::Async)?;
                    },
                    MessageType::Account(account) => {
                        let json_load = serde_json::to_string(&account)?;
                        let _: () = redis_conn.publish("account", json_load)?;

                        account_repository::save(db.clone(), account.into()).await;

                        cosumer.commit_message(&kafka_msg, rdkafka::consumer::CommitMode::Async)?;
                    },
                    _ => {},
                }
            }
        }
    }
    Ok(())
}