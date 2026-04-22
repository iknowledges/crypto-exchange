use std::time::Duration;

use anyhow::Ok;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tracing::error;

use crate::matching::{create_kafka_producer, message::Message};

pub struct MessageSender {
    producer: FutureProducer,
    topic: String,
}

impl MessageSender {
    pub fn new(bootstrap_server: &str, topic: &str) -> anyhow::Result<Self> {
        let producer = create_kafka_producer(bootstrap_server)?;
        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn send(&self, message: Message) -> anyhow::Result<()> {
        let payload = serde_json::to_string(&message)?;

        let record = FutureRecord::to(&self.topic)
            .payload(&payload).key("");

        if let Err((e, _)) = self.producer.send(record, Duration::from_secs(0)).await {
            error!("Kafka delivery failed: {:?}", e);
        }
        Ok(())
    }
}
