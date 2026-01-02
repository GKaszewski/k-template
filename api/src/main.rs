use std::net::SocketAddr;
use std::time::Duration as StdDuration;

use domain::UserService;
use infra::factory::build_session_store;
use infra::factory::build_user_repository;
use k_core::logging;
use tokio::net::TcpListener;
use tower_sessions::{Expiry, SessionManagerLayer};
use tracing::info;

mod auth;
mod config;
mod dto;
mod error;
mod routes;
mod state;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init("api");

    dotenvy::dotenv().ok();
    let config = Config::new().expect("Failed to load configuration");

    info!("Starting server on {}:{}", config.host, config.port);

    let db_config = k_core::db::DatabaseConfig {
        url: config.database_url.clone(),
        max_connections: 5,
        acquire_timeout: StdDuration::from_secs(30),
    };

    let db_pool = k_core::db::connect(&db_config).await?;

    infra::db::run_migrations(&db_pool).await?;

    let user_repo = build_user_repository(&db_pool).await?;
    let user_service = UserService::new(user_repo.clone());

    let session_store = build_session_store(&db_pool).await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(1)));

    let auth_layer = auth::setup_auth_layer(session_layer, user_repo.clone()).await?;

    let state = AppState::new(user_service, config.clone());

    let app = routes::api_v1_router().layer(auth_layer).with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
