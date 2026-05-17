use axum::{routing::{get, post}, Router};
use crate::{handlers::{auth, health}, openapi::openapi_router, state::AppState};

pub fn api_v1_router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/me", get(auth::me))
        .route("/health", get(health::health))
}

pub fn app_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1", api_v1_router())
        .merge(openapi_router())
}
