use std::str::FromStr;

use futures_util::TryStreamExt;
use mongodb::{Database, bson::{DateTime, Decimal128, doc}};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{matching::{enums::{OrderSide, OrderStatus, OrderType}, order::Order}, repository::PageInfo};

const ORDER_ENTITY: &str = "order_entity";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderEntity {
    #[serde(rename = "_id")]
    pub id: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub sequence: u64,
    pub product_id: String,
    pub user_id: String,
    pub client_oid: String,
    pub time: DateTime,
    pub size: Decimal128,
    pub funds: Decimal128,
    pub filled_size: Decimal128,
    pub executed_value: Decimal128,
    pub price: Decimal128,
    pub fill_fees: Decimal128,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub side: OrderSide,
    pub status: OrderStatus,
    // pub time_in_force: String,
    // pub settled: bool,
    // pub post_only: bool,
}

impl From<Order> for OrderEntity {
    fn from(value: Order) -> Self {
        let filled_size = value.size - value.remaining_size;
        let executed_value = value.funds - value.remaining_funds;
        OrderEntity {
            id: value.id,
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
            sequence: value.sequence,
            product_id: value.product_id,
            user_id: value.user_id,
            client_oid: value.client_oid.unwrap_or_default(),
            time: DateTime::from_millis(value.time.timestamp_millis()),
            size: Decimal128::from_str(&value.size.to_string()).unwrap(),
            funds: Decimal128::from_str(&value.funds.to_string()).unwrap(),
            filled_size: Decimal128::from_str(&filled_size.to_string()).unwrap(),
            executed_value: Decimal128::from_str(&executed_value.to_string()).unwrap(),
            price: Decimal128::from_str(&value.price.to_string()).unwrap(),
            fill_fees: Decimal128::from_str("0").unwrap(),
            order_type: value.order_type,
            side: value.side,
            status: value.status,
        }
    }
}

pub async fn find_by_order_id(db: Database, id: &str) -> Option<OrderEntity> {
    let collection = db.collection(ORDER_ENTITY);

    match collection.find_one(doc!{"_id": id}).await {
        Ok(entity) => entity,
        Err(e) => {
            error!("Order find_by_order_id error: {}", e);
            None
        }
    }
}

pub async fn save(db: Database, order_entity: OrderEntity) {
    let collection = db.collection(ORDER_ENTITY);
    let filter = doc! {
        "_id": &order_entity.id,
    };

    match collection.replace_one(filter, order_entity).upsert(true).await {
        Ok(res) => info!("Save order document: {}", res.modified_count),
        Err(e) => error!("Save order error: {}", e),
    }
}

pub async fn find_all(
    db: Database,
    page_index: u64,
    page_size: i64,
    user_id: &str,
    product_id: Option<String>,
    status_opt: Option<OrderStatus>,
    side_opt: Option<OrderSide>,
) -> anyhow::Result<PageInfo<OrderEntity>> {
    let collection = db.collection(ORDER_ENTITY);

    let mut filter = doc! {
        "userId": user_id,
    };

    if let Some(pid) = product_id {
        filter.insert("productId", &pid);
    }
    if let Some(status) = status_opt {
        filter.insert("status", status.as_ref());
    }
    if let Some(side) = side_opt {
        filter.insert("side", side.as_ref());
    }

    let total = collection.count_documents(filter.clone()).await?;

    let cursor =  collection.find(filter)
        .sort(doc!{ "sequence": -1 })
        .skip((page_index - 1) * page_size as u64)
        .limit(page_size)
        .await?;
    let items: Vec<OrderEntity> = cursor.try_collect().await?;
    Ok(PageInfo { items, total })
}