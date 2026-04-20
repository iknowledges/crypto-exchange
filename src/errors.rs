use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::controller::ApiResponse;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("page not found")]
    NotFound,
    #[error("request method not allowed")]
    MethodNotAllowed,
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("json error: {0}")]
    Json(#[from] JsonRejection),
    #[error("Unauthenticated: {0}")]
    Unauthenticated(String),
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            ApiError::Json(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Unauthenticated(_) => StatusCode::UNAUTHORIZED
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let body = axum::Json(ApiResponse::<()>::err(self.to_string()));
        (status_code, body).into_response()
    }
}

impl From<ApiError> for Response {
    fn from(error: ApiError) -> Response {
        error.into_response()
    }
}
