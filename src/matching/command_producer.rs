use std::time::Duration;

use rdkafka::{
    ClientConfig, admin::{AdminClient, AdminOptions, NewTopic}, client::DefaultClientContext, producer::{FutureProducer, FutureRecord}
};
use tracing::{error, info};

use crate::matching::command::Command;

pub struct MatchingEngineCommandProducer {
    producer: FutureProducer,
    topic: String,
}

impl MatchingEngineCommandProducer {
    pub fn new(bootstrap_server: &str, topic: &str) -> anyhow::Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap_server)
            .set("retries", "2147483647")
            .set("linger.ms", "100")
            .set("batch.size", (16384 * 2).to_string())
            // .set("enable.idempotence", "true")
            .set("max.in.flight.requests.per.connection", "5")
            .set("acks", "all")
            .create()?;
        Ok(Self { producer, topic: topic.to_string() })
    }

    pub async fn send(&self, command: &Command) -> anyhow::Result<()> {
        let payload = serde_json::to_string(command)?;

        let key = match command {
            Command::PlaceOrder(c) => &c.order_id,
            Command::CancelOrder(c) => &c.order_id,
            Command::Deposit(c) => &c.transaction_id,
            Command::PutProduct(c) => &c.product_id,
        };

        let record = FutureRecord::to(&self.topic).payload(&payload).key(key);

        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok(delivery) => info!("Sent: {:?}", delivery),
            Err((e, _)) => error!("Error delivering message: {:?}", e),
        }
        Ok(())
    }
}
