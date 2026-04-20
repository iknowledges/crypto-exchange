use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::matching::enums::{OrderSide, OrderType};

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    PlaceOrder(PlaceOrderCommand),
    CancelOrder(CancelOrderCommand),
    Deposit(DepositCommand),
    PutProduct(PutProductCommand),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaceOrderCommand {
    pub product_id: String,
    pub order_id: String,
    pub user_id: String,
    pub size: Decimal,
    pub price: Decimal,
    pub order_type: OrderType,
    pub order_side: OrderSide,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelOrderCommand {
    pub product_id: String,
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositCommand {
    pub user_id: String,
    pub currency: String,
    pub amount: Decimal,
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutProductCommand{
    pub product_id: String,
    pub base_currency: String,
    pub quote_currency: String,
}