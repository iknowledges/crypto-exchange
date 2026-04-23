use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, EnumString, AsRefStr)]
pub enum OrderSide {
    #[strum(ascii_case_insensitive)]
    BUY,
    #[strum(ascii_case_insensitive)]
    SELL,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, EnumString, AsRefStr)]
pub enum OrderType {
    #[strum(ascii_case_insensitive)]
    LIMIT,
    #[strum(ascii_case_insensitive)]
    MARKET,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, EnumString, AsRefStr)]
pub enum OrderStatus {
    RECEIVED,
    OPEN,
    DONE,
    REJECTED,
    FILLED,
    CANCELLED,
}