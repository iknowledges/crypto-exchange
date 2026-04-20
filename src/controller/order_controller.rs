use std::str::FromStr;

use axum::{Extension, Json, Router, debug_handler, extract::State, routing::post};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::jwt::Principal, controller::ApiResponse, errors::{ApiError, ApiResult}, matching::{command::{Command, PlaceOrderCommand}, enums::{OrderSide, OrderType}}, repository::product_repository, server::AppContext
};

pub fn create_router() -> Router<AppContext> {
    Router::new().route("/place", post(place_order))
}

#[derive(Deserialize)]
struct PlaceOrderParam {
    pub client_id: String,
    pub product_id: String,
    pub size: String,
    pub funds: String,
    pub price: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
}

impl Into<PlaceOrderCommand> for PlaceOrderParam {
    fn into(self) -> PlaceOrderCommand {
        PlaceOrderCommand {
            product_id: self.product_id,
            order_id: Uuid::new_v4().to_string(),
            user_id: String::new(),
            size: Decimal::from_str(&self.size).unwrap(),
            price: Decimal::from_str(&self.price).unwrap(),
            order_type: OrderType::LIMIT,
            order_side: OrderSide::BUY,
            time: Utc::now(),
        }
    }
}

#[debug_handler]
async fn place_order(
    State(state): State<AppContext>,
    Extension(principal): Extension<Principal>,
    Json(param): Json<PlaceOrderParam>,
) -> ApiResult<ApiResponse<()>> {
    // let product_id = Uuid::parse_str(param.product_id.clone())
    //     .map_err(|e| anyhow::anyhow!("Uuid parse error: {}", e))?;
    let product = product_repository::find_by_id(state.db, &param.product_id).await?;

    let mut command: PlaceOrderCommand = param.into();
    command.user_id = principal.id;
    state.producer.send(&Command::PlaceOrder(command)).await?;
    Ok(ApiResponse::ok("ok", None))
}
