use rdkafka::{ClientConfig, Message, admin::{AdminClient, AdminOptions, NewTopic}, client::DefaultClientContext, consumer::{self, Consumer, StreamConsumer}, producer::FutureProducer};
use tracing::error;

use crate::matching::{command::Command, engine::MatchingEngine};

pub mod enums;
pub mod command;
pub mod command_producer;
mod engine;
pub mod order_book;
pub mod order;
mod product_book;
pub mod account_book;
mod message_sender;
pub mod message;

pub async fn run_engine(bootstrap_server: &str, topic: &str, group_id: &str) -> anyhow::Result<()> {
    let engine = MatchingEngine::new()?;
    
    let consumer = create_kafka_consumer(bootstrap_server, group_id)?;
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

            engine.execute_command(command, offset).await;
        }
    }
}

fn create_kafka_producer(bootstrap_server: &str) -> anyhow::Result<FutureProducer> {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", bootstrap_server)
        .set("compression.type", "zstd")
        .set("retries", "2147483647")
        .set("linger.ms", "100")
        .set("batch.size", (16384 * 2).to_string())
        .set("enable.idempotence", "true")
        .set("max.in.flight.requests.per.connection", "5")
        .set("acks", "all")
        .create()?;
    Ok(producer)
}

fn create_kafka_consumer(bootstrap_server: &str, group_id: &str) -> anyhow::Result<StreamConsumer> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", bootstrap_server)
        .set("group.id", group_id)
        .set("auto.offset.reset", "earliest")
        .set("socket.timeout.ms", "4000")
        .create()?;
    Ok(consumer)
}
