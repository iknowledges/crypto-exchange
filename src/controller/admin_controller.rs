use std::str::FromStr;

use axum::{Json, Router, debug_handler, extract::State, routing::post};
use mongodb::bson::{DateTime, Decimal128};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    controller::ApiResponse,
    errors::ApiResult,
    matching::command::{Command, DepositCommand, PutProductCommand},
    repository::product_repository::{self, ProductEntity},
    server::AppContext,
};

pub fn create_router() -> Router<AppContext> {
    Router::new()
        .route("/deposit", post(deposit))
        .route("/product", post(save_product))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepositParam {
    user_id: String,
    currency: String,
    amount: Decimal,
}

impl Into<DepositCommand> for DepositParam {
    fn into(self) -> DepositCommand {
        DepositCommand {
            user_id: self.user_id,
            currency: self.currency,
            amount: self.amount,
            transaction_id: Uuid::new_v4().to_string(),
        }
    }
}

#[debug_handler]
async fn deposit(
    State(state): State<AppContext>,
    Json(param): Json<DepositParam>,
) -> ApiResult<ApiResponse<()>> {
    let command = param.into();
    state.producer.send(&Command::Deposit(command)).await?;
    Ok(ApiResponse::ok("success", None))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutProductParam {
    base_currency: String,
    quote_currency: String,
}

impl Into<ProductEntity> for PutProductParam {
    fn into(self) -> ProductEntity {
        ProductEntity {
            id: format!("{}-{}", self.base_currency, self.quote_currency),
            base_currency: self.base_currency,
            quote_currency: self.quote_currency,
            base_min_size: Decimal128::from_str("0").unwrap(),
            base_max_size: Decimal128::from_str("100000000").unwrap(),
            quote_min_size: Decimal128::from_str("0").unwrap(),
            quote_max_size: Decimal128::from_str("10000000000").unwrap(),
            base_scale: 6,
            quote_scale: 2,
            quote_increment: 0.,
            taker_fee_rate: 0.,
            maker_fee_rate: 0.,
            diplay_order: 0,
            created_at: DateTime::now(),
        }
    }
}

#[debug_handler]
async fn save_product(
    State(state): State<AppContext>,
    Json(param): Json<PutProductParam>,
) -> ApiResult<ApiResponse<()>> {
    let entity: ProductEntity = param.into();
    let command = PutProductCommand {
        product_id: entity.id.clone(), 
        base_currency: entity.base_currency.clone(), 
        quote_currency: entity.quote_currency.clone(),
    };

    product_repository::save(state.db, entity).await;

    state.producer.send(&Command::PutProduct(command)).await?;
    Ok(ApiResponse::ok("success", None))
}
