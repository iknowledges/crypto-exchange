use std::str::FromStr;

use mongodb::{Database, bson::{DateTime, Decimal128, doc}};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::matching::{enums::OrderSide, order::Trade};


const TRADE_ENTITY: &str = "trade_entity";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeEntity {
    #[serde(rename = "_id")]
    pub id: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub sequence: u64,
    pub product_id: String,
    pub taker_order_id: String,
    pub maker_order_id: String,
    pub price: Decimal128,
    pub size: Decimal128,
    pub side: OrderSide,
    pub time: DateTime,
}

impl From<Trade> for TradeEntity {
    fn from(value: Trade) -> Self {
        TradeEntity {
            id: format!("{}-{}", value.product_id, value.sequence),
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
            sequence: value.sequence,
            product_id: value.product_id,
            taker_order_id: value.taker_order_id,
            maker_order_id: value.maker_order_id,
            price: Decimal128::from_str(&value.price.to_string()).unwrap(),
            size: Decimal128::from_str(&value.size.to_string()).unwrap(),
            side: value.side,
            time: DateTime::from_millis(value.time.timestamp_millis()),
        }
    }
}

pub async fn save(db: Database, order_entity: TradeEntity) {
    let collection = db.collection(TRADE_ENTITY);
    let filter = doc! {
        "_id": &order_entity.id,
    };

    match collection.replace_one(filter, order_entity).upsert(true).await {
        Ok(res) => info!("Save trade document: {}", res.modified_count),
        Err(e) => error!("Save trade error: {}", e),
    }
}