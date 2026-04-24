use crate::{matching::command_producer::MatchingEngineCommandProducer, server::AppContext};

mod logger;
mod config;
mod server;
mod controller;
mod errors;
mod repository;
mod auth;
mod matching;
mod feed;
mod cache;
mod market;
mod snapshot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init();

    let app_cfg = config::get();

    matching::run_engine(&app_cfg.bootstrap_server, &app_cfg.command_topic, "MatchingEngine").await?;

    let command_producer = MatchingEngineCommandProducer::new(&app_cfg.bootstrap_server, &app_cfg.command_topic)?;

    let db = repository::init(&app_cfg.database_url, &app_cfg.database_name).await?;
    market::persistence::start("MarketPersistence", db.clone()).await?;

    snapshot::start("EngineSnapshot").await?;

    let state = AppContext::new(db, command_producer);

    server::start(state, app_cfg.server_port).await?;
    Ok(())
}
