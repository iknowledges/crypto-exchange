use rdkafka::Message as _;
use tracing::error;

use crate::{matching::message::Message, snapshot::consumer::MatchingEngineSnapshotConsumer};

mod manager;
mod consumer;

pub async fn start(group_id: &str) -> anyhow::Result<()> {
    let snapshot_consumer = MatchingEngineSnapshotConsumer::new(group_id).await?;

    tokio::spawn(async move {
        if let Err(e) = run_loop(snapshot_consumer).await {
            error!("Snapshot run loop error: {}", e);
        }
    });
    Ok(())
}

async fn run_loop(mut snapshot_consumer: MatchingEngineSnapshotConsumer) -> anyhow::Result<()> {
    snapshot_consumer.initialize_state().await?;

    loop {
        match snapshot_consumer.consumer.recv().await {
            Err(e) => error!("Kafka error: {}", e),
            Ok(kafka_msg) => {
                let payload = match kafka_msg.payload() {
                    Some(p) => p,
                    None => continue,
                };

                if let Ok(message) = serde_json::from_slice::<Message>(payload) {
                    let offset = kafka_msg.offset();
                    snapshot_consumer.process_message(message, offset).await?;
                }
            }
        }
    }
}