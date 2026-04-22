use serde::{Deserialize, Serialize};

use crate::matching::{account_book::Account, order_book::Order};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub sequence: u64,
    pub message_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Account(Account),
    Product,
    Order(OrderMessage),
    Trade,
    CommandStart,
    CommandEnd,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderMessage {
    pub order_book_sequence: u64,
    pub order: Order,
}