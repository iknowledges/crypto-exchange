use std::collections::HashMap;

use rdkafka::{ClientConfig, consumer::{Consumer, StreamConsumer}};
use tracing::info;

use crate::{config, matching::{account_book::Account, message::{Message, MessageType}, order::Order, product_book::Product}, snapshot::manager::{EngineSnapshotManager, EngineState}};

pub struct MatchingEngineSnapshotConsumer {
    pub consumer: StreamConsumer,
    topic: String,
    snapshot_manager: EngineSnapshotManager,
    accounts: HashMap<String, Account>,
    orders: HashMap<String, Order>,
    products: HashMap<String, Product>,
    engine_state: EngineState,
}

impl MatchingEngineSnapshotConsumer {
    pub async fn new(group_id: &str) -> anyhow::Result<Self> {
        let app_cfg = config::get();

        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", app_cfg.bootstrap_server.clone())
            .set("group.id", group_id)
            .set("enable.auto.commit", "true")
            .create()?;

        let snapshot_manager = EngineSnapshotManager::new(&app_cfg.database_url, &app_cfg.database_name).await?;

        Ok(Self {
            consumer,
            snapshot_manager,
            accounts: HashMap::new(),
            orders: HashMap::new(),
            products: HashMap::new(),
            engine_state: EngineState::default(),
            topic: app_cfg.message_topic.to_string(),
        })
    }

    pub async fn initialize_state(&mut self) -> anyhow::Result<()> {
        self.consumer.subscribe(&[&self.topic])?;

        // Fetch current snapshot state from DB
        let state = self.snapshot_manager.get_engine_state().await;
        
        self.engine_state = state.unwrap_or_default();
        self.clean_buffers();

        // Seek to the saved offset if it exists
        if let Some(offset) = self.engine_state.message_offset {
            let target_offset = rdkafka::Offset::Offset(offset + 1);
            info!("Seeking to offset: {:?}", target_offset);
            // In a real scenario, you'd iterate assigned partitions
            self.consumer.seek(&self.topic, 0, target_offset, std::time::Duration::from_secs(5))?;
        }
        Ok(())
    }

    pub async fn process_message(&mut self, message: Message, msg_offset: i64) -> anyhow::Result<()> {
        // 1. Sequence Validation
        let expected_sequence = self.engine_state.message_sequence.unwrap_or(0) + 1;
        if message.sequence < expected_sequence {
            return Ok(()); 
        } else if message.sequence > expected_sequence {
            anyhow::bail!("Out of sequence: sequence={}, expected={}", message.sequence, expected_sequence);
        }

        // 2. Update Engine State metadata
        self.engine_state.message_offset = Some(msg_offset);
        self.engine_state.message_sequence = Some(message.sequence);

        // 3. Handle Message Variants (Pattern Matching)
        match message.message_type {
            MessageType::Order(order_msg) => {
                let order = order_msg.order;
                self.engine_state.order_sequences.insert(order.product_id.clone(), order.sequence);
                self.engine_state.order_book_sequences.insert(order.product_id.clone(), order_msg.order_book_sequence);
                self.orders.insert(order.id.clone(), order);
            }
            MessageType::Trade(trade) => {
                self.engine_state.trade_sequences.insert(trade.product_id, trade.sequence);
            }
            MessageType::Account(account) => {
                self.accounts.insert(account.id.clone(), account);
            }
            MessageType::Product(product) => {
                self.products.insert(product.id.clone(), product);
            }
            MessageType::CommandStart(cmd_start) => {
                self.engine_state.command_offset = Some(cmd_start);
            }
            MessageType::CommandEnd(cmd_end) => {
                self.engine_state.command_offset = Some(cmd_end);
                self.save_state().await?;
            }
        }
        Ok(())
    }

    async fn save_state(&mut self) -> anyhow::Result<()> {
        let accounts_vec: Vec<Account> = self.accounts.values().cloned().collect();
        let orders_vec: Vec<Order> = self.orders.values().cloned().collect();
        let products_vec: Vec<Product> = self.products.values().cloned().collect();

        self.snapshot_manager.save(
            self.engine_state.clone(),
            accounts_vec,
            orders_vec,
            products_vec
        ).await?;

        self.clean_buffers();
        Ok(())
    }

    pub fn clean_buffers(&mut self) {
        self.accounts.clear();
        self.orders.clear();
        self.products.clear();
    }
}