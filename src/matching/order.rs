use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::matching::{command::PlaceOrderCommand, enums::{OrderSide, OrderStatus, OrderType}};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[serde(rename = "_id")]
    pub id: String,
    pub sequence: u64,
    pub user_id: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub remaining_size: Decimal,
    pub price: Decimal,
    pub remaining_funds: Decimal,
    pub size: Decimal,
    pub funds: Decimal,
    pub post_only: bool,
    pub time: DateTime<Utc>,
    pub product_id: String,
    pub status: OrderStatus,
    pub client_oid: Option<String>,
}

impl From<PlaceOrderCommand> for Order {
    fn from(command: PlaceOrderCommand) -> Self {
        let funds = if command.order_type == OrderType::LIMIT {
            command.size * command.price
        } else {
            Decimal::ZERO
        };

        Self {
            id: command.order_id,
            sequence: 0,
            user_id: command.user_id,
            order_type: command.order_type,
            side: command.order_side,
            remaining_size: command.size,
            price: command.price,
            remaining_funds: funds,
            size: command.size,
            funds,
            post_only: false,
            time: command.time,
            product_id: command.product_id,
            status: OrderStatus::RECEIVED,
            client_oid: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub product_id: String,
    pub sequence: u64,
    pub size: Decimal,
    pub funds: Decimal,
    pub price: Decimal,
    pub time: DateTime<Utc>,
    pub side: OrderSide,
    pub taker_order_id: String,
    pub maker_order_id: String,
}