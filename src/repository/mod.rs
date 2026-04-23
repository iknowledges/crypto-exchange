use mongodb::{Database, options::ClientOptions};
use serde::{Serialize, Serializer};

pub mod user_repository;
pub mod product_repository;
pub mod account_repository;
pub mod order_repository;
pub mod trade_repository;

pub async fn init(database_url: &str, database_name: &str) -> anyhow::Result<Database> {
    let client_options = ClientOptions::parse(database_url).await?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(database_name);
    Ok(db)
}

#[derive(Serialize)]
pub struct PageInfo<T> {
    pub items: Vec<T>,
    pub total: u64
}