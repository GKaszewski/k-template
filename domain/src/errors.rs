//! Domain errors for K-Notes
//!
//! Uses `thiserror` for ergonomic error definitions.
//! These errors represent domain-level failures and will be mapped
//! to HTTP status codes in the API layer.

use thiserror::Error;
use uuid::Uuid;

/// Domain-level errors for K-Notes operations
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DomainError {
    /// The requested user was not found
    #[error("User not found: {0}")]
    UserNotFound(Uuid),

    /// User with this email/subject already exists
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    /// A validation error occurred
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// User is not authenticated (maps to HTTP 401)
    #[error("Unauthenticated: {0}")]
    Unauthenticated(String),

    /// User is not allowed to perform this action (maps to HTTP 403)
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// A repository/infrastructure error occurred
    #[error("Repository error: {0}")]
    RepositoryError(String),

    /// An infrastructure adapter error occurred
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
}

impl DomainError {
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }

    /// Create an unauthenticated error (not logged in → 401)
    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::Unauthenticated(message.into())
    }

    /// Create a forbidden error (not allowed → 403)
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    /// Check if this error indicates a "not found" condition
    pub fn is_not_found(&self) -> bool {
        matches!(self, DomainError::UserNotFound(_))
    }

    /// Check if this error indicates a conflict (already exists)
    pub fn is_conflict(&self) -> bool {
        matches!(self, DomainError::UserAlreadyExists(_))
    }
}

impl From<crate::value_objects::ValidationError> for DomainError {
    fn from(error: crate::value_objects::ValidationError) -> Self {
        DomainError::ValidationError(error.to_string())
    }
}

/// Result type alias for domain operations
pub type DomainResult<T> = Result<T, DomainError>;
