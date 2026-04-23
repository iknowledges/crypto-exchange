use std::str::FromStr;

use axum::{Extension, Json, Router, debug_handler, extract::{Path, Query, State}, routing::{get, post}};
use chrono::Utc;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::jwt::Principal, controller::ApiResponse, errors::ApiResult, matching::{command::{CancelOrderCommand, Command, PlaceOrderCommand}, enums::{OrderSide, OrderStatus, OrderType}}, repository::{PageInfo, order_repository::{self, OrderEntity}, product_repository::{self, ProductEntity}}, server::AppContext
};

pub fn create_router() -> Router<AppContext> {
    Router::new()
        .route("/place", post(place_order))
        .route("/cancel/{order_id}", post(cancel_order))
        .route("/list", get(list_orders))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaceOrderParam {
    pub client_oid: String,
    pub product_id: String,
    pub size: Decimal,
    pub funds: Decimal,
    pub price: Decimal,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    // GTC, GTT, IOC, or FOK (default is GTC)
    pub time_in_force: String,
}

impl Into<PlaceOrderCommand> for PlaceOrderParam {
    fn into(self) -> PlaceOrderCommand {
        PlaceOrderCommand {
            product_id: self.product_id,
            order_id: Uuid::new_v4().to_string(),
            user_id: String::new(),
            size: self.size,
            price: self.price,
            funds: self.funds,
            order_type: OrderType::from_str(&self.order_type).unwrap(),
            order_side: OrderSide::from_str(&self.side).unwrap(),
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
    let product = product_repository::find_by_id(state.db, &param.product_id).await;

    let Some(product) = product else {
        return Ok(ApiResponse::err("product not found"));
    };

    let mut command: PlaceOrderCommand = param.into();
    command.user_id = principal.id;
    format_place_order_command(&mut command, &product);
    state.producer.send(&Command::PlaceOrder(command)).await?;

    Ok(ApiResponse::ok("place order success", None))
}

fn format_place_order_command(cmd: &mut PlaceOrderCommand, product: &ProductEntity) {
    // Determine target scales
    let base_scale = product.base_scale as u32;
    let quote_scale = product.quote_scale as u32;

    match cmd.order_type {
        OrderType::LIMIT => {
            // RoundingMode.DOWN in Java is RoundingStrategy::ToZero in rust_decimal
            cmd.size = cmd.size.round_dp_with_strategy(base_scale, RoundingStrategy::ToZero);
            cmd.price = cmd.price.round_dp_with_strategy(quote_scale, RoundingStrategy::ToZero);
            
            cmd.funds = if cmd.order_side == OrderSide::BUY {
                cmd.size * cmd.price
            } else {
                Decimal::ZERO
            };
        }
        OrderType::MARKET => {
            cmd.price = Decimal::ZERO;
            if cmd.order_side == OrderSide::BUY {
                cmd.size = Decimal::ZERO;
                cmd.funds = cmd.funds.round_dp_with_strategy(quote_scale, RoundingStrategy::ToZero);
            } else {
                cmd.size = cmd.size.round_dp_with_strategy(base_scale, RoundingStrategy::ToZero);
                cmd.funds = Decimal::ZERO;
            }
        }
    }
}

#[debug_handler]
async fn cancel_order(
    State(state): State<AppContext>,
    Extension(principal): Extension<Principal>,
    Path(order_id): Path<String>,
) -> ApiResult<ApiResponse<()>> {
    if let Some(order) = order_repository::find_by_order_id(state.db, &order_id).await {
        if order.user_id != principal.id {
            return Ok(ApiResponse::err("Current user id is incorrect"));
        }
        let command = CancelOrderCommand {
            product_id: order.product_id,
            order_id
        };
        state.producer.send(&Command::CancelOrder(command)).await?;
        Ok(ApiResponse::ok("cancel order success", None))
    } else {
        Ok(ApiResponse::err(format!("order not found: {}", order_id)))
    }
}

#[derive(Deserialize)]
struct ListOrdersParam {
    page: u64,
    size: i64,
    product_id: Option<String>,
    status: Option<OrderStatus>,
    side: Option<OrderSide>,
}

#[debug_handler]
async fn list_orders(
    State(state): State<AppContext>,
    Extension(principal): Extension<Principal>,
    Query(param): Query<ListOrdersParam>,
) -> ApiResult<ApiResponse<PageInfo<OrderEntity>>> {
    let page_info = order_repository::find_all(
        state.db,
        param.page,
        param.size,
        &principal.id,
        param.product_id,
        param.status,
        param.side
    ).await?;
    Ok(ApiResponse::ok("success", Some(page_info)))
}