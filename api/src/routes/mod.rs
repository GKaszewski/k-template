//! API Routes
//!
//! Defines the API endpoints and maps them to handler functions.

use crate::state::AppState;
use axum::Router;

pub mod auth;
pub mod config;

/// Construct the API v1 router
pub fn api_v1_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/config", config::router())
}
