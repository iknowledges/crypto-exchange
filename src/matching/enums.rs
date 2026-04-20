use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderSide {
    BUY,
    SELL,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderType {
    LIMIT,
    MARKET,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderStatus {
    RECEIVED,
    OPEN,
    DONE,
    REJECTED,
    FILLED,
    CANCELLED,
}