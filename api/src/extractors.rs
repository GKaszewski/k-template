//! Auth extractors for API handlers
//!
//! Provides the `CurrentUser` extractor that validates JWT Bearer tokens.

use axum::{extract::FromRequestParts, http::request::Parts};
use domain::User;

use crate::error::ApiError;
use crate::state::AppState;

/// Extracted current user from the request.
///
/// Validates a JWT Bearer token from the `Authorization` header.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        #[cfg(feature = "auth-jwt")]
        {
            return match try_jwt_auth(parts, state).await {
                Ok(user) => Ok(CurrentUser(user)),
                Err(e) => Err(e),
            };
        }

        #[cfg(not(feature = "auth-jwt"))]
        {
            let _ = (parts, state);
            Err(ApiError::Unauthorized(
                "No authentication backend configured".to_string(),
            ))
        }
    }
}

/// Authenticate using JWT Bearer token
#[cfg(feature = "auth-jwt")]
async fn try_jwt_auth(parts: &mut Parts, state: &AppState) -> Result<User, ApiError> {
    use axum::http::header::AUTHORIZATION;

    let auth_header = parts
        .headers
        .get(AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ApiError::Unauthorized("Invalid Authorization header encoding".to_string()))?;

    let token = auth_str.strip_prefix("Bearer ").ok_or_else(|| {
        ApiError::Unauthorized("Authorization header must use Bearer scheme".to_string())
    })?;

    let validator = state
        .jwt_validator
        .as_ref()
        .ok_or_else(|| ApiError::Internal("JWT validator not configured".to_string()))?;

    let claims = validator.validate_token(token).map_err(|e| {
        tracing::debug!("JWT validation failed: {:?}", e);
        match e {
            infra::auth::jwt::JwtError::Expired => {
                ApiError::Unauthorized("Token expired".to_string())
            }
            infra::auth::jwt::JwtError::InvalidFormat => {
                ApiError::Unauthorized("Invalid token format".to_string())
            }
            _ => ApiError::Unauthorized("Token validation failed".to_string()),
        }
    })?;

    let user_id: uuid::Uuid = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID in token".to_string()))?;

    let user = state
        .user_service
        .find_by_id(user_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to fetch user: {}", e)))?;

    Ok(user)
}
