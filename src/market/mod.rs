use mongodb::Database;
use rdkafka::{ClientConfig, Message as _, consumer::{Consumer, StreamConsumer}};
use redis::{Commands, Connection};
use tracing::error;

use crate::{cache, config, matching::message::{Message, MessageType}, repository::account_repository};


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

pub async fn start(group_id: &str, db: Database) -> anyhow::Result<()> {
    let consumer = create_message_consumer(group_id)?;

    let redis_conn = cache::get_client().get_connection()?;

    tokio::spawn(async move {
        if let Err(e) = run_loop(consumer, redis_conn, db).await {
            error!("market consumer start error: {}", e);
        }
    });

    Ok(())
}

async fn run_loop(cosumer: StreamConsumer, mut redis_conn: Connection, db: Database) -> anyhow::Result<()> {
    loop {
        match cosumer.recv().await {
            Err(e) => error!("Kafka error: {}", e),
            Ok(kafka_msg) => {
                let payload = match kafka_msg.payload() {
                    Some(p) => p,
                    None => continue,
                };

                let message = serde_json::from_slice::<Message>(payload)?;

                match message.message_type {
                    MessageType::Order(order_msg) => {
                        // self.order_manager.save(entity).await;

                        let json_load = serde_json::to_string(&order_msg)?;
                        let _: () = redis_conn.publish("order", json_load)?;

                        cosumer.commit_message(&kafka_msg, rdkafka::consumer::CommitMode::Async)?;
                    }
                    MessageType::Account(account) => {
                        let json_load = serde_json::to_string(&account)?;
                        let _: () = redis_conn.publish("account", json_load)?;

                        account_repository::save(db.clone(), account.into()).await;

                        cosumer.commit_message(&kafka_msg, rdkafka::consumer::CommitMode::Async)?;
                    },
                    MessageType::Product => todo!(),
                    MessageType::Trade => todo!(),
                    MessageType::CommandStart => todo!(),
                    MessageType::CommandEnd => todo!(),
                }
                // if let Ok(Message{ message_type: MessageType::Order(order_msg), .. }) =  {
                
                // }
            }
        }
    }
    Ok(())
}