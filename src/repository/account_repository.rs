use std::str::FromStr;

use mongodb::{
    Database,
    bson::{DateTime, Decimal128, Uuid, doc},
};
use serde::Serialize;
use tracing::{error, info};

use crate::matching::account_book::Account;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountEntity {
    pub id: String,
    pub user_id: String,
    pub currency: String,
    pub hold: Decimal128,
    pub available: Decimal128,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl From<Account> for AccountEntity {
    fn from(value: Account) -> Self {
        AccountEntity {
            id: format!("{}-{}", value.user_id, value.currency),
            user_id: value.user_id,
            currency: value.currency,
            hold: Decimal128::from_str(&value.hold.to_string()).unwrap(),
            available: Decimal128::from_str(&value.available.to_string()).unwrap(),
            created_at: DateTime::now(),
            updated_at: DateTime::now()
        }
    }
}

pub async fn save(db: Database, account: AccountEntity) {
    let collection = db.collection("account_entity");
    let filter = doc! {
        "userId": &account.user_id,
        "currency": &account.currency,
    };

    match collection.replace_one(filter, account).upsert(true).await {
        Ok(res) => info!("Replaced account document: {}", res.modified_count),
        Err(e) => error!("Save account error: {}", e),
    }
}
