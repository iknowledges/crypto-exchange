use rdkafka::{ClientConfig, consumer::{Consumer, StreamConsumer}};

use crate::config;

pub mod persistence;

fn create_message_consumer(group_id: &str) -> anyhow::Result<StreamConsumer> {
    let app_cfg = config::get();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &app_cfg.bootstrap_server)
        .set("group.id", group_id)
        .set("enable.auto.commit", "false")
        .create()?;
    consumer.subscribe(&[&app_cfg.message_topic])?;
    Ok(consumer)
}
