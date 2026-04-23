use std::collections::HashSet;

use axum::extract::ws::Message;
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::feed::SubRequest;

pub struct SessionManager {
    // Maps channel name (e.g. "BTC-USD.level2") to a set of sender channels for active WS tasks
    pub subscriptions: DashMap<String, HashSet<String>>,
    // Maps SessionID to its mpsc sender to push messages to the specific WS task
    pub sessions: DashMap<String, mpsc::UnboundedSender<Message>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: DashMap::new(),
            sessions: DashMap::new(),
        }
    }

    pub fn sub_or_unsub(&self, sub: SubRequest, session_id: &str, user_id: &str, is_sub: bool) {
        for chan in sub.channels {
            match chan.as_str() {
                "order" => {
                    for pid in &sub.product_ids {
                        let order_chan = format!("{}.{}.{}", user_id, pid, chan);
                        if is_sub {
                            self.subscribe_channel(&order_chan, session_id);
                        } else {
                            self.unsubscribe_channel(&order_chan, session_id);
                        }
                    }
                }
                "funds" => {
                    if let Some(currencies) = &sub.currency_ids {
                        for currency in currencies {
                            let account_chan = format!("{}.{}.{}", user_id, currency, chan);
                            if is_sub {
                                self.subscribe_channel(&account_chan, session_id);
                            } else {
                                self.unsubscribe_channel(&account_chan, session_id);
                            }
                        }
                    }
                }
                _ => {
                    warn!("Subsribe channel {} not found", chan);
                }
            }
        }
    }

    fn subscribe_channel(&self, chan: &str, session_id: &str) {
        info!("subscribe: {}", chan);
        self.subscriptions.entry(chan.to_string()).or_default().insert(session_id.to_string());
    }

    fn unsubscribe_channel(&self, chan: &str, session_id: &str) {
        info!("unsubscribe: {}", chan);
        // Use get_mut to access the HashSet for this channel
        if let Some(mut sessions) = self.subscriptions.get_mut(chan) {
            sessions.remove(session_id);
            
            // Remove the channel from the map if it's now empty
            if sessions.is_empty() {
                self.subscriptions.remove(chan);
            }
        }
    }

    pub async fn broadcast<T: Serialize>(&self, channel: &str, msg: &T) {
        let payload = serde_json::to_string(msg).unwrap();
        if let Some(session_ids) = self.subscriptions.get(channel) {
            for sid in session_ids.value() {
                if let Some(tx) = self.sessions.get(sid) {
                    let _ = tx.send(Message::Text(payload.clone().into()));
                }
            }
        } else {
            warn!("broadcast channel warn: {} not found", channel);
        }
    }
}
