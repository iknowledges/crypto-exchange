use std::collections::HashSet;

use axum::extract::ws::Message;
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{info, warn};

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

    pub fn subscribe(&self, chan: &str, session_id: &str) {
        info!("subscribe: {}", chan);
        self.subscriptions.entry(chan.to_string()).or_default().insert(session_id.to_string());
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