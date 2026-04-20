use mongodb::{Database, options::ClientOptions};

pub mod user_repository;
pub mod product_repository;

pub async fn init(database_url: &str, database_name: &str) -> anyhow::Result<Database> {
    let client_options = ClientOptions::parse(database_url).await?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database(database_name);
    Ok(db)
}