use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use domain::errors::DomainError;
use serde_json::json;

pub struct AppError(DomainError);

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self { Self(e) }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            DomainError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            DomainError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            DomainError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            DomainError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            DomainError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
