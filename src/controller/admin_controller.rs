use axum::{Json, Router, debug_handler, extract::State, routing::post};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::{controller::ApiResponse, errors::ApiResult, matching::command::{Command, DepositCommand}, server::AppContext};

pub fn create_router() -> Router<AppContext> {
    Router::new()
        .route("/deposit", post(deposit))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepositParam {
    user_id: String,
    currency: String,
    amount: Decimal
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
    Json(param): Json<DepositParam>
) -> ApiResult<ApiResponse<()>> {
    let command = param.into();
    state.producer.send(&Command::Deposit(command)).await?;
    Ok(ApiResponse::ok("success", None))
}