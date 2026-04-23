use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

use dashmap::DashMap;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::{config, matching::{account_book::AccountBook, command::{CancelOrderCommand, Command, DepositCommand, PlaceOrderCommand, PutProductCommand}, message::{Message, MessageType}, message_sender::MessageSender, order::Order, order_book::OrderBook, product_book::{Product, ProductBook}}};

pub struct MatchingEngine {
    order_books: DashMap<String, OrderBook>,
    message_sequence: Arc<AtomicU64>,
    message_sender: Arc<MessageSender>,
    product_book: Arc<RwLock<ProductBook>>,
    account_book: Arc<RwLock<AccountBook>>,
    pub startup_command_offset: Option<i64>,
}

impl MatchingEngine {
    pub fn new() -> anyhow::Result<Self> {
        let app_cfg = config::get();
        let message_sender = Arc::new(MessageSender::new(&app_cfg.bootstrap_server, &app_cfg.message_topic)?);
        let message_sequence = Arc::new(AtomicU64::new(0));
        let account_book = Arc::new(RwLock::new(AccountBook::new(message_sender.clone(), message_sequence.clone())));
        let product_book = Arc::new(RwLock::new(ProductBook::new(message_sender.clone(), message_sequence.clone())));
        Ok(Self {
            order_books: DashMap::new(),
            account_book,
            product_book,
            message_sequence, 
            message_sender, 
            startup_command_offset: None
        })
    }

    // pub fn restore(state_store: &EngineSnapshotManager, sender: MessageSender) -> Result<Self> {
    //     info!("Restoring matching engine from snapshot...");
        
    //     let state = state_store.get_engine_state()
    //         .context("Failed to load engine state")?;

    //     let message_sequence = AtomicU64::new(state.message_sequence.unwrap_or(0));
    //     let mut product_book = ProductBook::new(sender.clone(), &message_sequence);
    //     let mut account_book = AccountBook::new(sender.clone(), &message_sequence);

    //     // Batch load products and accounts
    //     state_store.get_products().into_iter().for_each(|p| product_book.add_product(p));
    //     state_store.get_accounts().into_iter().for_each(|a| account_book.add(a));

    //     let mut order_books = HashMap::new();
    //     for product in product_book.get_all_products() {
    //         let mut ob = OrderBook::new(
    //             product.id.clone(),
    //             state.order_sequences.get(&product.id).copied().unwrap_or(0),
    //             &account_book,
    //             &product_book,
    //             sender.clone(),
    //             &message_sequence,
    //         );
            
    //         state_store.get_orders(&product.id).into_iter().for_each(|o| ob.add_order(o));
    //         order_books.insert(product.id.clone(), ob);
    //     }

    //     Ok(Self {
    //         order_books,
    //         message_sequence,
    //         message_sender: sender,
    //         product_book,
    //         account_book,
    //         startup_command_offset: state.command_offset,
    //     })
    // }

    pub async fn execute_command(&mut self, command: Command, offset: i64) {
        info!("execute_command: {:?}, offset: {}", command, offset);
        self.send_marker(offset, true).await;

        match command {
            Command::PlaceOrder(cmd) => self.execute_place_order(cmd).await,
            Command::CancelOrder(cmd) => self.execute_cancel_order(cmd).await,
            Command::Deposit(cmd) => self.execute_deposit(cmd).await,
            Command::PutProduct(cmd) => self.execute_put_product(cmd).await,
        }

        self.send_marker(offset, false).await;
    }

    async fn execute_deposit(&self, cmd: DepositCommand) {
        let mut acct_boot = self.account_book.write().await;
        acct_boot.deposit(
            &cmd.user_id, 
            &cmd.currency, 
            cmd.amount,
        ).await;
    }

    async fn execute_put_product(&self, cmd: PutProductCommand) {
        let product = Product::from(cmd);
        let product_id = product.id.clone();
        let mut prod_book = self.product_book.write().await;
        prod_book.put_product(product).await;
        self.create_order_book(&product_id);
    }

    async fn execute_place_order(&mut self, cmd: PlaceOrderCommand) {
        if let Some(mut order_book) = self.order_books.get_mut(&cmd.product_id) {
            order_book.place_order(Order::from(cmd)).await;
        } else {
            warn!("no such order book to place: {}", cmd.product_id);
        }
    }

    async fn execute_cancel_order(&mut self, cmd: CancelOrderCommand) {
        if let Some(mut order_book) = self.order_books.get_mut(&cmd.product_id) {
            order_book.cancel_order(&cmd.order_id).await;
        } else {
            warn!("no such order book to cancel: {}", cmd.product_id);
        }
    }

    fn create_order_book(&self, product_id: &str) {
        self.order_books.entry(product_id.to_string()).or_insert_with(|| {
            OrderBook::new(
                product_id,
                0, 0, 0,
                self.account_book.clone(),
                self.product_book.clone(),
                self.message_sender.clone(),
                self.message_sequence.clone()
            )
        });
    }

    async fn send_marker(&self, offset: i64, is_start: bool) {
        let sequence = self.message_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let message = if is_start {
            Message {
                sequence,
                message_type: MessageType::CommandStart(offset),
            }
        } else {
            Message {
                sequence,
                message_type: MessageType::CommandEnd(offset),
            }
        };
        if let Err(e) = self.message_sender.send(message).await {
            error!("send_marker error: {}", e);
        }
    }
}
