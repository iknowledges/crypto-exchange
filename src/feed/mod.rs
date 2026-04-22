use std::sync::Arc;

use axum::{Extension, extract::{Query, WebSocketUpgrade, ws::{Message, WebSocket}}, http::StatusCode, response::IntoResponse};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, warn};

use crate::{auth::jwt::{Principal, get_jwt}, feed::session_manager::SessionManager};

pub mod session_manager;
pub mod listener;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Request {
    #[serde(rename = "subscribe")]
    Subscribe(SubRequest),
    #[serde(rename = "unsubscribe")]
    Unsubscribe(SubRequest),
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Deserialize)]
struct SubRequest {
    #[serde(rename = "productIds")]
    product_ids: Vec<String>,
    channels: Vec<String>,
    #[serde(rename = "currencyIds")]
    currency_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct AuthParams {
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<AuthParams>,
    cookie: CookieJar,
    Extension(manager): Extension<Arc<SessionManager>>,
) -> impl IntoResponse {
    // 1. Get token from query
    let token = params.access_token
        .or_else(|| cookie.get("accessToken").map(|c| c.value().to_string()));

    let Some(token) = token else {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    };

    // 2. Decode token
    let principal = match get_jwt().decode(&token) {
        Ok(p) => p,
        Err(e) => {
            error!("decode token error: {}", e);
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
        }
    };

    // 3. Upgrade only for authenticated users
    ws.on_upgrade(move |socket| handle_socket(socket, manager, principal))
}

async fn handle_socket(socket: WebSocket, manager: Arc<SessionManager>, pricipal: Principal) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    
    let session_id = uuid::Uuid::new_v4().to_string();
    info!("session start: {}", session_id);
    manager.sessions.insert(session_id.clone(), tx);

    // Task for sending messages to this specific client
    // let sid_for_send = session_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = sender.send(msg).await {
                warn!("send_task error: {}", e);
                break;
            }
        }
    });

    // Task for receiving/processing messages from this client
    let sid_for_recv = session_id.clone();
    let manager_clone = manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(req) = serde_json::from_str::<Request>(&text) {
                    match req {
                        Request::Subscribe(sub) => {
                            for chan in sub.channels {
                                match chan.as_str() {
                                    "order" => {
                                        for pid in &sub.product_ids {
                                            let full_chan = format!("{}.{}", pid, chan);
                                            manager_clone.subscribe(&full_chan, &sid_for_recv);
                                        }
                                    },
                                    "funds" => {
                                        let user_id = pricipal.id.clone();
                                        if let Some(currencies) = &sub.currency_ids {
                                            for currency in currencies {
                                                let account_chan = format!("{}.{}.{}", user_id, currency, chan);
                                                manager_clone.subscribe(&account_chan, &sid_for_recv);
                                            }
                                        }
                                    }
                                    _ => { warn!("Subsribe channel {} not found", chan); }
                                }

                            }
                        },
                        Request::Unsubscribe(unsub) => {
                            // Similar logic to remove from subscriptions
                        },
                        Request::Ping => {
                            if let Some(tx) = manager_clone.sessions.get(&sid_for_recv) {
                                let _ = tx.send(Message::Text(r#"{"type":"pong"}"#.into()));
                            }
                        }
                    }
                } else {
                    warn!("Parse message failed: {}", text);
                }
            }
        }
    });

    // Wait for either task to finish, then cleanup
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Cleanup
    manager.sessions.remove(&session_id);
    manager.subscriptions.alter_all(|_, mut v| { v.remove(&session_id); v });
    info!("session closed: {}", session_id);
}