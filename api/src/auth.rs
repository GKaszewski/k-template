//! Authentication logic
//!
//! Proxies to infra implementation if enabled.

use std::sync::Arc;

use domain::UserRepository;
use infra::session_store::{InfraSessionStore, SessionManagerLayer};

use crate::error::ApiError;

#[cfg(feature = "auth-axum-login")]
pub use infra::auth::backend::{AuthManagerLayer, AuthSession, AuthUser, Credentials};

#[cfg(feature = "auth-axum-login")]
pub async fn setup_auth_layer(
    session_layer: SessionManagerLayer<InfraSessionStore>,
    user_repo: Arc<dyn UserRepository>,
) -> Result<AuthManagerLayer, ApiError> {
    infra::auth::backend::setup_auth_layer(session_layer, user_repo)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))
}
