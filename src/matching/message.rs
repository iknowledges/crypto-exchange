use serde::{Deserialize, Serialize};

use crate::matching::{account_book::Account, order::Trade, order::Order, product_book::Product};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub sequence: u64,
    pub message_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Account(Account),
    Product(Product),
    Order(OrderMessage),
    Trade(Trade),
    CommandStart(i64),
    CommandEnd(i64),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderMessage {
    pub order_book_sequence: u64,
    pub order: Order,
}