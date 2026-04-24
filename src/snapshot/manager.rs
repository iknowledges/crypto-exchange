use std::collections::HashMap;

use mongodb::{
    IndexModel,
    bson::doc,
    options::{ClientOptions, IndexOptions},
};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::matching::{
    account_book::Account, enums::OrderStatus, order::Order, product_book::Product,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EngineState {
    #[serde(rename = "_id")]
    pub id: String,
    pub command_offset: Option<i64>,
    pub message_offset: Option<i64>,
    pub message_sequence: Option<u64>,
    pub trade_sequences: HashMap<String, u64>,
    pub order_sequences: HashMap<String, u64>,
    pub order_book_sequences: HashMap<String, u64>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            command_offset: None,
            message_offset: None,
            message_sequence: None,
            trade_sequences: HashMap::new(),
            order_sequences: HashMap::new(),
            order_book_sequences: HashMap::new(),
        }
    }
}

pub struct EngineSnapshotManager {
    client: mongodb::Client,
    engine_state_collection: mongodb::Collection<EngineState>,
    account_collection: mongodb::Collection<Account>,
    product_collection: mongodb::Collection<Product>,
    order_collection: mongodb::Collection<Order>,
}

impl EngineSnapshotManager {
    pub async fn new(database_url: &str, database_name: &str) -> anyhow::Result<Self> {
        let client_options = ClientOptions::parse(database_url).await?;
        let client = mongodb::Client::with_options(client_options)?;

        let db = client.database(database_name);
        let engine_state_collection = db.collection("snapshot_engine");
        let account_collection = db.collection("snapshot_account");
        let product_collection = db.collection("snapshot_product");
        let order_collection = db.collection("snapshot_order");
        let index = IndexModel::builder()
            .keys(doc! {"product_id": -1, "sequence": -1})
            .options(IndexOptions::builder().unique(true).build())
            .build();
        let _ = order_collection.create_index(index);

        Ok(Self {
            client,
            engine_state_collection,
            account_collection,
            product_collection,
            order_collection,
        })
    }

    pub async fn save(
        &self,
        engine_state: EngineState,
        accounts: Vec<Account>,
        orders: Vec<Order>,
        products: Vec<Product>,
    ) -> mongodb::error::Result<()> {
        let mut session = self.client.start_session().await?;
        session.start_transaction().await?;

        self.engine_state_collection
            .replace_one(doc! { "_id": &engine_state.id }, engine_state)
            .upsert(true)
            .session(&mut session)
            .await?;

        if !accounts.is_empty() {
            for acct in accounts {
                self.account_collection
                    .replace_one(doc! { "_id": &acct.id }, acct)
                    .upsert(true)
                    .session(&mut session)
                    .await?;
            }
        }

        if !products.is_empty() {
            for prod in products {
                self.product_collection
                    .replace_one(doc! { "_id": &prod.id }, prod)
                    .upsert(true)
                    .session(&mut session)
                    .await?;
            }
        }

        if !orders.is_empty() {
            for order in orders {
                let filter = doc! { "_id": &order.id };
                if order.status == OrderStatus::OPEN {
                    self.order_collection
                        .replace_one(filter, order)
                        .upsert(true)
                        .session(&mut session)
                        .await?;
                } else {
                    self.order_collection
                        .delete_one(filter)
                        .session(&mut session)
                        .await?;
                }
            }
        }

        session.commit_transaction().await?;
        Ok(())
    }

    pub async fn get_engine_state(&self) -> Option<EngineState> {
        match self.engine_state_collection.find_one(doc! {"_id": "default"}).await {
            Ok(es) => es,
            Err(e) => {
                error!("get_engine_state error: {}", e);
                None
            }
        }
    }
}
