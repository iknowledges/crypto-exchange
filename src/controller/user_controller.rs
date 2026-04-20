use axum::{Extension, Json, Router, debug_handler, extract::State, routing::{get, post}};
use serde::{Deserialize, Serialize};

use crate::{auth::{jwt::{Principal, get_jwt}, middleware::get_auth_layer}, controller::ApiResponse, errors::ApiResult, repository::user_repository::{self, UserEntity}, server::AppContext};

pub fn create_router() -> Router<AppContext> {
    Router::new()
        .route("/info", get(user_info))
        .route_layer(get_auth_layer())
        .route("/login", post(login))
}

#[derive(Deserialize)]
struct LoginParam {
    pub email: String,
    pub password: String
}

#[derive(Serialize)]
struct LoginResponse {
    pub token: String
}

#[debug_handler]
async fn login(
    State(state): State<AppContext>,
    Json(params): Json<LoginParam>,
) -> ApiResult<ApiResponse<LoginResponse>> {
    let user_opt = user_repository::find_by_email(state.db, &params.email).await?;
    if let Some(user) = user_opt {
        if user.password.eq(&params.password) {
            let principal = Principal {
                id: user.id.to_string(),
                email: params.email
            };

            let token = get_jwt().encode(principal)?;

            let response = LoginResponse { token };
            return Ok(ApiResponse::ok("login success", Some(response)));
        } else {
            return Ok(ApiResponse::err("Invalid email or password"));
        }
    } else {
        return Ok(ApiResponse::err("User not found"));
    }
}

#[debug_handler]
async fn user_info(
    Extension(principal): Extension<Principal>,
    State(state): State<AppContext>,
) -> ApiResult<ApiResponse<UserEntity>> {
    let user_opt = user_repository::find_by_email(state.db, &principal.email).await?;
    Ok(ApiResponse::ok("sucess", user_opt))
}
