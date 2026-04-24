use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::matching::{command::PutProductCommand, message::{Message, MessageType}, message_sender::MessageSender};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    #[serde(rename = "_id")]
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
}

impl From<PutProductCommand> for Product {
    fn from(value: PutProductCommand) -> Self {
        Product {
            id: value.product_id,
            base_currency: value.base_currency,
            quote_currency: value.quote_currency
        }
    }
}

pub struct ProductBook {
    products: HashMap<String, Product>,
    message_sender: Arc<MessageSender>,
    message_sequence: Arc<AtomicU64>,
}

impl ProductBook {
    pub fn new(message_sender: Arc<MessageSender>, message_sequence: Arc<AtomicU64>) -> Self {
        Self {
            products: HashMap::new(),
            message_sender,
            message_sequence,
        }
    }

    pub fn get_all_products(&self) -> Vec<Product> {
        self.products.values().cloned().collect()
    }

    pub fn get_product(&self, product_id: &str) -> Option<&Product> {
        self.products.get(product_id)
    }

    pub fn add_product(&mut self, product: Product) {
        self.products.insert(product.id.clone(), product);
    }

    pub async fn put_product(&mut self, product: Product) {
        // 1. Update internal state
        let product_id = product.id.clone();
        let cloned_product = product.clone();
        self.products.insert(product_id, product);

        // 2. Construct and send message
        let sequence = self.message_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let message = Message {
            sequence,
            message_type: MessageType::Product(cloned_product)
        };
        if let Err(e) = self.message_sender.send(message).await {
            error!("put_product error: {}", e);
        }
    }
}