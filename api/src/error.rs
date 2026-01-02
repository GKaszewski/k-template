//! API error handling
//!
//! Maps domain errors to HTTP responses

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use domain::DomainError;

/// API-level errors
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Domain(#[from] DomainError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal server error")]
    Internal(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

/// Error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_response) = match &self {
            ApiError::Domain(domain_error) => {
                let status = match domain_error {
                    DomainError::UserNotFound(_) => StatusCode::NOT_FOUND,

                    DomainError::UserAlreadyExists(_) => StatusCode::CONFLICT,

                    DomainError::ValidationError(_) => StatusCode::BAD_REQUEST,

                    DomainError::Unauthorized(_) => StatusCode::FORBIDDEN,

                    DomainError::RepositoryError(_) | DomainError::InfrastructureError(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };

                (
                    status,
                    ErrorResponse {
                        error: domain_error.to_string(),
                        details: None,
                    },
                )
            }

            ApiError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: "Validation error".to_string(),
                    details: Some(msg.clone()),
                },
            ),

            ApiError::Internal(msg) => {
                // Log internal errors but don't expose details
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "Internal server error".to_string(),
                        details: None,
                    },
                )
            }

            ApiError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ErrorResponse {
                    error: "Forbidden".to_string(),
                    details: Some(msg.clone()),
                },
            ),

            ApiError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error: "Unauthorized".to_string(),
                    details: Some(msg.clone()),
                },
            ),
        };

        (status, Json(error_response)).into_response()
    }
}

impl ApiError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

/// Result type alias for API handlers
pub type ApiResult<T> = Result<T, ApiError>;
