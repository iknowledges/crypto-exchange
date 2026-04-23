use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::matching::{enums::OrderSide, message::{Message, MessageType}, message_sender::MessageSender};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub currency: String,
    pub available: Decimal,
    pub hold: Decimal,
}

pub struct AccountBook {
    // Nested HashMap: userId -> (currency -> Account)
    accounts: HashMap<String, HashMap<String, Account>>,
    message_sender: Arc<MessageSender>,
    message_sequence: Arc<AtomicU64>,
}

impl AccountBook {
    pub fn new(message_sender: Arc<MessageSender>, message_sequence: Arc<AtomicU64>) -> Self {
        Self {
            accounts: HashMap::new(),
            message_sender,
            message_sequence,
        }
    }

    pub fn add(&mut self, account: Account) {
        self.accounts
            .entry(account.user_id.clone())
            .or_default()
            .insert(account.currency.clone(), account);
    }

    pub fn get_account(&self, user_id: &str, currency: &str) -> Option<&Account> {
        self.accounts.get(user_id)?.get(currency)
    }

    fn get_mut_or_create(&mut self, user_id: &str, currency: &str) -> &mut Account {
        self.accounts
            .entry(user_id.to_string())
            .or_default()
            .entry(currency.to_string())
            .or_insert_with(|| Account {
                 id: format!("{}-{}", user_id, currency), 
                 user_id: user_id.to_string(), 
                 currency: currency.to_string(), 
                 available: Decimal::ZERO, 
                 hold: Decimal::ZERO
            })
    }

    pub async fn deposit(&mut self, user_id: &str, currency: &str, amount: Decimal) {
        let account = self.get_mut_or_create(user_id, currency);
        account.available += amount;

        let account_clone = account.clone();
        self.send_account_message(account_clone).await;
    }

    pub async fn hold(&mut self, user_id: &str, currency: &str, amount: Decimal) -> bool {
        if amount <= Decimal::ZERO {
            error!("Amount should be greater than 0: {}", amount);
            return false;
        }

        if let Some(account) = self.accounts.get_mut(user_id).and_then(|m| m.get_mut(currency)) {
            if account.available < amount {
                warn!("hold available: {}, amount: {}", account.available, amount);
                return false;
            }
            account.available -= amount;
            account.hold += amount;

            let account_clone = account.clone();
            self.send_account_message(account_clone).await;
            return true;
        } else {
            error!("account not found by user_id: {}, currency: {}", user_id, currency);
        }
        false
    }

    pub async fn unhold(&mut self, user_id: &str, currency: &str, amount: Decimal) -> anyhow::Result<()> {
        if let Some(account) = self.accounts.get_mut(user_id)
            .and_then(|m| m.get_mut(currency)) {
            if amount <= Decimal::ZERO || account.hold < amount {
                anyhow::bail!("Invalid unhold: amount <= 0 or insufficient hold");
            }
            account.available += amount;
            account.hold -= amount;

            let account_clone = account.clone();
            self.send_account_message(account_clone).await;
        } else {
            anyhow::bail!("Account not found or insufficient funds");
        }
        Ok(())
    }

    async fn update_balance(&mut self, user_id: &str, currency:&str, available_delta: Decimal, hold_delta: Decimal) -> anyhow::Result<()> {
        let account = self.get_mut_or_create(user_id, currency);
        account.available += available_delta;
        account.hold += hold_delta;

        if account.available < Decimal::ZERO || account.hold < Decimal::ZERO {
            anyhow::bail!("Negative balance on account: {:?}", account);
        }

        let account_clone = account.clone();
        self.send_account_message(account_clone).await;
        Ok(())
    }

    pub async fn exchange(
        &mut self,
        taker_user_id: &str,
        maker_user_id: &str,
        base_currency: &str,
        quote_currency: &str,
        taker_side: OrderSide,
        size: Decimal,
        funds: Decimal,
    ) -> anyhow::Result<()> {
        if taker_side == OrderSide::BUY {
            // Taker buys base
            self.update_balance(taker_user_id, base_currency, size, Decimal::ZERO).await?;
            // Taker pays quote (from hold)
            self.update_balance(taker_user_id, quote_currency, Decimal::ZERO, -funds).await?;
            // Maker gives base (from hold)
            self.update_balance(maker_user_id, base_currency, Decimal::ZERO, -size).await?;
            // Maker gets quote
            self.update_balance(maker_user_id, quote_currency, funds, Decimal::ZERO).await?;
        } else {
            // Taker gives base (from hold)
            self.update_balance(taker_user_id, base_currency, Decimal::ZERO, -size).await?;
            // Taker gets quote
            self.update_balance(taker_user_id, quote_currency, funds, Decimal::ZERO).await?;
            // Maker gets base
            self.update_balance(maker_user_id, base_currency, size, Decimal::ZERO).await?;
            // Maker pays quote (from hold)
            self.update_balance(maker_user_id, quote_currency, Decimal::ZERO, -funds).await?;
        }
        Ok(())
    }

    async fn send_account_message(&self, account: Account) {
        let sequence = self.message_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let message = Message {
            sequence,
            message_type: MessageType::Account(account),
        };
        if let Err(e) = self.message_sender.send(message).await {
            error!("send_account_message error: {}", e);
        }
    }
}
