use mongodb::{Database, bson::{DateTime, Decimal128, Uuid, doc}};
use serde::{Deserialize, Serialize};

const PRODUCT_ENTITY: &str = "product_entity";

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductEntity {
    #[serde(rename = "_id")]
    pub id: Uuid,
    pub base_currency: String,
    pub quote_currency: String,
    pub base_min_size: Decimal128,
    pub base_max_size: Decimal128,
    pub quote_min_size: Decimal128,
    pub quote_max_size: Decimal128,
    pub base_scale: i32,
    pub quote_scale: i32,
    pub quote_increment: f64,
    pub taker_fee_rate: f64,
    pub maker_fee_rate: f64,
    pub diplay_order: i32,
    pub created_at: DateTime,
}

pub async fn find_by_id(db: Database, id: &str) -> anyhow::Result<Option<ProductEntity>> {
    let collection = db.collection(PRODUCT_ENTITY);
    let entity = collection.find_one(doc!{"_id": id}).await?;
    Ok(entity)
}