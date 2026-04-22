use axum::{Router, response::{IntoResponse, Response}};
use serde::{Deserialize, Serialize};
use crate::{auth::middleware::get_auth_layer, server::AppContext};

mod user_controller;
mod admin_controller;
mod order_controller;

pub fn create_router() -> Router<AppContext> {
    Router::new().nest("/api", Router::new()
        .nest("/admin", admin_controller::create_router())
        .nest("/order", order_controller::create_router())
        .route_layer(get_auth_layer())
        .nest("/user", user_controller::create_router()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn new(success: bool, message: String, data: Option<T>) -> Self {
        Self { success, message, data }
    }

    pub fn ok<U: AsRef<str>>(message: U, data: Option<T>) -> Self {
        Self::new(true, String::from(message.as_ref()), data)
    }

    pub fn err<U: AsRef<str>>(message: U) -> Self {
        Self::new(false, String::from(message.as_ref()), None)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        axum::Json(self).into_response()
    }
}