use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum OrderSide {
    BUY,
    SELL,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum OrderType {
    LIMIT,
    MARKET,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum OrderStatus {
    RECEIVED,
    OPEN,
    DONE,
    REJECTED,
    FILLED,
    CANCELLED,
}