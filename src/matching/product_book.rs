use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use tracing::error;

use crate::matching::{message::{Message, MessageType}, message_sender::MessageSender};

#[derive(Debug, Clone)]
pub struct Product {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
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
            message_type: MessageType::Product
        };
        if let Err(e) = self.message_sender.send(message).await {
            error!("put_product error: {}", e);
        }
    }
}