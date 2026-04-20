use rdkafka::{ClientConfig, Message, admin::{AdminClient, AdminOptions, NewTopic}, client::DefaultClientContext, consumer::{Consumer, StreamConsumer}};
use tracing::error;

use crate::matching::{command::Command, engine::MatchingEngine};

pub mod enums;
pub mod command;
pub mod command_producer;
mod engine;
mod order_book;
mod product_book;
mod account_book;
mod message_sender;

pub async fn run_engine(brokers: &str, group_id: &str, topic: &str) -> anyhow::Result<()> {
    let engine = MatchingEngine::new();
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("auto.offset.reset", "earliest")
        .set("socket.timeout.ms", "4000")
        .create()?;
    consumer.subscribe(&[topic])?;

    tokio::spawn(async move {
        if let Err(e) = engine_loop(consumer, engine).await {
            error!("Engine loop error: {}", e);
        }
    });
    Ok(())
}

async fn engine_loop(consumer: StreamConsumer, mut engine: MatchingEngine) -> anyhow::Result<()> {
    loop {
        let message = consumer.recv().await?;
        let payload = match message.payload() {
            Some(p) => p,
            None => continue,
        };

        if let Ok(command) = serde_json::from_slice::<Command>(payload) {
            let offset = message.offset();

            if let Some(start_offset) = engine.startup_command_offset {
                if offset <= start_offset { continue; }
            }

            engine.execute_command(command, offset);
        }
    }
}