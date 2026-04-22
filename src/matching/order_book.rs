use std::{collections::{BTreeMap, HashMap}, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::matching::{account_book::AccountBook, command::PlaceOrderCommand, enums::{OrderSide, OrderStatus, OrderType}, message::{Message, MessageType, OrderMessage}, message_sender::MessageSender, product_book::{Product, ProductBook}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub sequence: u64,
    pub user_id: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub remaining_size: Decimal,
    pub price: Decimal,
    pub remaining_funds: Decimal,
    pub size: Decimal,
    pub funds: Decimal,
    pub post_only: bool,
    pub time: DateTime<Utc>,
    pub product_id: String,
    pub status: OrderStatus,
    pub client_oid: Option<String>,
}

impl From<PlaceOrderCommand> for Order {
    fn from(command: PlaceOrderCommand) -> Self {
        let funds = if command.order_type == OrderType::LIMIT {
            command.size * command.price
        } else {
            Decimal::ZERO
        };

        Self {
            id: command.order_id,
            sequence: 0,
            user_id: command.user_id,
            order_type: command.order_type,
            side: command.order_side,
            remaining_size: command.size,
            price: command.price,
            remaining_funds: funds,
            size: command.size,
            funds,
            post_only: false,
            time: command.time,
            product_id: command.product_id,
            status: OrderStatus::RECEIVED,
            client_oid: None,
        }
    }
}

pub struct OrderBook {
    product_id: String,
    account_book: AccountBook,
    product_book: ProductBook,
    // BTreeMap is sorted by key. 
    // Asks (Sells) are natural order (low to high).
    asks: BTreeMap<Decimal, HashMap<String, Order>>,
    // Bids (Buys) need reverse order.
    bids: BTreeMap<Decimal, HashMap<String, Order>>,
    order_by_id: HashMap<String, Order>,
    message_sender: Arc<MessageSender>,
    message_sequence: Arc<AtomicU64>,
    order_sequence: u64,
    trade_sequence: u64,
    order_book_sequence: u64,
}

impl OrderBook {
    pub async fn place_order(&mut self, mut taker_order: Order) {
        let product = match self.product_book.get_product(&self.product_id) {
            Some(p) => p.clone(),
            None => {
                warn!("order rejected, reason: {} not found", self.product_id);
                return;
            }
        };
        self.order_sequence += 1;
        taker_order.sequence = self.order_sequence;

        // 1. Hold funds
        let ok = if taker_order.side == OrderSide::BUY {
            self.account_book.hold(&taker_order.user_id, &product.quote_currency, taker_order.remaining_funds).await
        } else {
            self.account_book.hold(&taker_order.user_id, &product.base_currency, taker_order.remaining_size).await
        };
        if !ok {
            warn!("order rejected, reason: INSUFFICIENT_FUNDS: {:?}", taker_order);
            taker_order.status = OrderStatus::REJECTED;
            self.send_order_msg(taker_order).await;
            return;
        }

        taker_order.status = OrderStatus::RECEIVED;
        self.send_order_msg(taker_order.clone()).await;

        // 2. Matching logic
        if let Err(e) = self.match_order(&mut taker_order, &product).await {
            error!("match_order error: {}", e);
            return;
        }

        // 3. Post-match handling
        if taker_order.order_type == OrderType::LIMIT && taker_order.remaining_size > Decimal::ZERO {
            taker_order.status = OrderStatus::OPEN;
            self.order_book_sequence += 1;
            self.add_order(taker_order.clone());
        } else {
            if taker_order.remaining_size > Decimal::ZERO {
                taker_order.status = OrderStatus::CANCELLED;
            } else {
                taker_order.status = OrderStatus::FILLED;
            }
            self.unhold_order_funds(&taker_order, &product).await;
        }
        self.send_order_msg(taker_order).await;
    }

    pub fn add_order(&mut self, order: Order) {
        let depth = if order.side == OrderSide::BUY { &mut self.bids } else { &mut self.asks };
        depth.entry(order.price).or_default().insert(order.id.clone(), order.clone());
        self.order_by_id.insert(order.id.clone(), order);
    }

    async fn match_order(&mut self, taker_order: &mut Order, product: &Product) -> anyhow::Result<()> {
        let is_buy = taker_order.side == OrderSide::BUY;
        // Loop through the appropriate depth
        // If Buying, match against Asks (low to high). If Selling, match against Bids (high to low).
        while taker_order.remaining_size > Decimal::ZERO {
            let maker_price = if is_buy {
                self.asks.keys().next().cloned()
            } else {
                self.bids.keys().next_back().cloned()
            };

            let price = match maker_price {
                Some(p) if self.is_price_crossed(taker_order, p) => p,
                _ => break, // No more matching prices
            };

            let mut orders_at_level = if is_buy {
                self.asks.remove(&price).unwrap()
            } else {
                self.bids.remove(&price).unwrap()
            };

            // Process orders at this price level
            let mut ids_to_remove = Vec::new();
            for (id, maker_order) in orders_at_level.iter_mut() {
                if let Some(trade) = self.execute_trade(taker_order, maker_order) {
                    // Exchange funds in account book
                    self.account_book.exchange(
                        &taker_order.user_id,
                        &maker_order.user_id,
                        &product.base_currency,
                        &product.quote_currency,
                        taker_order.side,
                        trade.size,
                        trade.funds,
                    ).await?;
                    if maker_order.status == OrderStatus::FILLED {
                        ids_to_remove.push(id.clone());
                        self.order_by_id.remove(id);
                        self.unhold_order_funds(maker_order, product).await;
                    }

                    self.order_book_sequence += 1;
                    self.send_order_msg(maker_order.clone()).await;
                    self.send_trade_msg(trade).await;

                    if taker_order.remaining_size <= Decimal::ZERO {
                        break;
                    }
                } else {
                    break;
                }
            }

            for id in ids_to_remove {
                orders_at_level.remove(&id);
            }

            if !orders_at_level.is_empty() {
                if is_buy {
                    self.asks.insert(price, orders_at_level);
                } else {
                    self.bids.insert(price, orders_at_level);
                }
                break;
            }
        }

        Ok(())
    }

    fn execute_trade(&mut self, taker: &mut Order, maker: &mut Order) -> Option<Trade> {
        let price = maker.price;
        let taker_size = if taker.side == OrderSide::BUY && taker.order_type == OrderType::MARKET {
            taker.remaining_funds.checked_div(price)?.round_dp(4)
        } else {
            taker.remaining_size
        };

        if taker_size <= Decimal::ZERO {
            return None;
        }

        let trade_size = taker_size.min(maker.remaining_size);
        let trade_funds = (trade_size * price).round_dp(8);
        
        taker.remaining_size -= trade_size;
        maker.remaining_size -= trade_size;

        if taker.side == OrderSide::BUY {
            taker.remaining_funds -= trade_funds;
        } else {
            maker.remaining_funds -= trade_funds;
        }

        if maker.remaining_size <= Decimal::ZERO {
            maker.status = OrderStatus::FILLED;
        }

        self.trade_sequence += 1;
        Some(Trade {
            sequence: self.trade_sequence,
            product_id: self.product_id.clone(),
            size: trade_size,
            funds: trade_funds,
            price,
            side: maker.side,
            time: taker.time,
            taker_order_id: taker.id.clone(),
            maker_order_id: maker.id.clone(),
        })
    }

    fn is_price_crossed(&self, taker: &Order, maker_price: Decimal) -> bool {
        if taker.order_type == OrderType::MARKET {
            return true;
        }
        if taker.side == OrderSide::BUY {
            taker.price >= maker_price
        } else {
            taker.price <= maker_price
        }
    }

    async fn unhold_order_funds(&mut self, maker_order: &Order, product: &Product) {
        if maker_order.side == OrderSide::BUY {
            if maker_order.remaining_funds > Decimal::ZERO {
                if let Err(e) = self.account_book.unhold(&maker_order.user_id, &product.quote_currency, maker_order.remaining_funds).await {
                    error!("unhold_order_funds buy error: {}", e);
                }
            }
        } else {
            if maker_order.remaining_size > Decimal::ZERO {
                if let Err(e) = self.account_book.unhold(&maker_order.user_id, &product.base_currency, maker_order.remaining_size).await {
                    error!("unhold_order_funds sell error: {}", e);
                }
            }
        }
    }

    async fn send_order_msg(&self, order: Order) {
        let sequence = self.message_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let order_message = OrderMessage {
            order_book_sequence: self.order_book_sequence,
            order
        };
        let message = Message {
            sequence,
            message_type: MessageType::Order(order_message),
        };
        if let Err(e) = self.message_sender.send(message).await {
            error!("send_order_msg error: {}", e);
        }
    }

    async fn send_trade_msg(&self, trade: Trade) {
        let sequence = self.message_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let message = Message {
            sequence,
            message_type: MessageType::Trade,
        };
        if let Err(e) = self.message_sender.send(message).await {
            error!("send_trade_msg error: {}", e);
        }
    }
}


struct Trade {
    pub product_id: String,
    pub sequence: u64,
    pub size: Decimal,
    pub funds: Decimal,
    pub price: Decimal,
    pub time: DateTime<Utc>,
    pub side: OrderSide,
    pub taker_order_id: String,
    pub maker_order_id: String,
}