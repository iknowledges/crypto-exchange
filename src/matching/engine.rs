use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use tracing::{info, warn};

use crate::{config, matching::{account_book::AccountBook, command::{CancelOrderCommand, Command, DepositCommand, PlaceOrderCommand, PutProductCommand}, message_sender::{self, MessageSender}, order_book::{Order, OrderBook}, product_book::ProductBook}};

pub struct MatchingEngine {
    order_books: HashMap<String, OrderBook>,
    message_sequence: Arc<AtomicU64>,
    message_sender: Arc<MessageSender>,
    product_book: ProductBook,
    account_book: AccountBook,
    pub startup_command_offset: Option<i64>,
}

impl MatchingEngine {
    pub fn new() -> anyhow::Result<Self> {
        let app_cfg = config::get();
        let message_sender = Arc::new(MessageSender::new(&app_cfg.bootstrap_server, &app_cfg.message_topic)?);
        let message_sequence = Arc::new(AtomicU64::new(0));
        Ok(Self {
            order_books: HashMap::new(),
            account_book: AccountBook::new(message_sender.clone(), message_sequence.clone()), 
            product_book: ProductBook::new(message_sender.clone(), message_sequence.clone()), 
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
        self.send_marker(offset, true);

        match command {
            Command::PlaceOrder(cmd) => self.execute_place_order(cmd).await,
            Command::CancelOrder(cmd) => self.execute_cancel_order(cmd),
            Command::Deposit(cmd) => self.execute_deposit(cmd).await,
            Command::PutProduct(cmd) => self.execute_put_product(cmd),
        }

        self.send_marker(offset, false);
    }

    async fn execute_deposit(&mut self, cmd: DepositCommand) {
        self.account_book.deposit(
            &cmd.user_id, 
            &cmd.currency, 
            cmd.amount,
        ).await;
    }

    fn execute_put_product(&mut self, cmd: PutProductCommand) {
        // let product = Product::from(cmd.clone());
        // let product_id = cmd.product_id.clone();
        // self.product_book.put_product(product);
        // self.create_order_book(product_id);
    }

    async fn execute_place_order(&mut self, cmd: PlaceOrderCommand) {
        if let Some(order_book) = self.order_books.get_mut(&cmd.product_id) {
            order_book.place_order(Order::from(cmd)).await;
        } else {
            warn!("no such order book: {}", cmd.product_id);
        }
    }

    fn execute_cancel_order(&mut self, cmd: CancelOrderCommand) {
        // if let Some(order_book) = self.order_books.get_mut(&cmd.product_id) {
        //     order_book.cancel_order(&cmd.order_id);
        // } else {
        //     warn!("no such order book: {}", cmd.product_id);
        // }
    }

    fn create_order_book(&mut self, product_id: String) {
        // self.order_books.entry(product_id.clone()).or_insert_with(|| {
        //     OrderBook::new(
        //         product_id,
        //         0, 0, 0, // Initial sequences
        //         &self.account_book,
        //         &self.product_book,
        //         self.message_sender.clone(),
        //         &self.message_sequence
        //     )
        // });
    }

    fn send_marker(&self, offset: i64, is_start: bool) {
        let seq = self.message_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        if is_start {
            // self.message_sender.send()
        } else {
            // self.message_sender.send()
        }
    }
}
