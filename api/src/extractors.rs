//! Auth extractors for API handlers
//!
//! Provides the `CurrentUser` extractor that works with both session and JWT auth.

use axum::{extract::FromRequestParts, http::request::Parts};
use domain::User;

use crate::config::AuthMode;
use crate::error::ApiError;
use crate::state::AppState;

/// Extracted current user from the request.
///
/// This extractor supports multiple authentication methods based on the configured `AuthMode`:
/// - `Session`: Uses axum-login session cookies
/// - `Jwt`: Uses Bearer token in Authorization header
/// - `Both`: Tries JWT first, then falls back to session
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_mode = state.config.auth_mode;

        // Try JWT first if enabled
        #[cfg(feature = "auth-jwt")]
        if matches!(auth_mode, AuthMode::Jwt | AuthMode::Both) {
            match try_jwt_auth(parts, state).await {
                Ok(Some(user)) => return Ok(CurrentUser(user)),
                Ok(None) => {
                    // No JWT token present, continue to session auth if Both mode
                    if auth_mode == AuthMode::Jwt {
                        return Err(ApiError::Unauthorized(
                            "Missing or invalid Authorization header".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    // JWT was present but invalid
                    tracing::debug!("JWT auth failed: {}", e);
                    if auth_mode == AuthMode::Jwt {
                        return Err(e);
                    }
                    // In Both mode, continue to try session
                }
            }
        }

        // Try session auth if enabled
        #[cfg(feature = "auth-axum-login")]
        if matches!(auth_mode, AuthMode::Session | AuthMode::Both) {
            if let Some(user) = try_session_auth(parts).await? {
                return Ok(CurrentUser(user));
            }
        }

        Err(ApiError::Unauthorized("Not authenticated".to_string()))
    }
}

/// Try to authenticate using JWT Bearer token
#[cfg(feature = "auth-jwt")]
async fn try_jwt_auth(parts: &mut Parts, state: &AppState) -> Result<Option<User>, ApiError> {
    use axum::http::header::AUTHORIZATION;

    // Get Authorization header
    let auth_header = match parts.headers.get(AUTHORIZATION) {
        Some(header) => header,
        None => return Ok(None), // No header = no JWT auth attempted
    };

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ApiError::Unauthorized("Invalid Authorization header encoding".to_string()))?;

    // Extract Bearer token
    let token = auth_str.strip_prefix("Bearer ").ok_or_else(|| {
        ApiError::Unauthorized("Authorization header must use Bearer scheme".to_string())
    })?;

    // Get JWT validator
    let validator = state
        .jwt_validator
        .as_ref()
        .ok_or_else(|| ApiError::Internal("JWT validator not configured".to_string()))?;

    // Validate token
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

    // Fetch user from database by ID (subject contains user ID)
    let user_id: uuid::Uuid = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID in token".to_string()))?;

    let user = state
        .user_service
        .find_by_id(user_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to fetch user: {}", e)))?;

    Ok(Some(user))
}

/// Try to authenticate using session cookie
#[cfg(feature = "auth-axum-login")]
async fn try_session_auth(parts: &mut Parts) -> Result<Option<User>, ApiError> {
    use infra::auth::backend::AuthSession;

    // Check if AuthSession extension is present (added by auth middleware)
    if let Some(auth_session) = parts.extensions.get::<AuthSession>() {
        if let Some(auth_user) = &auth_session.user {
            return Ok(Some(auth_user.0.clone()));
        }
    }

    Ok(None)
}
