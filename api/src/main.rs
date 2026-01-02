use std::net::SocketAddr;
use std::time::Duration as StdDuration;

use axum::Router;
use domain::UserService;
use infra::factory::build_session_store;
use infra::factory::build_user_repository;
use infra::run_migrations;
use k_core::http::server::ServerConfig;
use k_core::http::server::apply_standard_middleware;
use k_core::logging;
use time::Duration;
use tokio::net::TcpListener;
use tower_sessions::{Expiry, SessionManagerLayer};
use tracing::info;

mod auth;
mod config;
mod dto;
mod error;
mod routes;
mod state;

use crate::auth::setup_auth_layer;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init("api");

    let config = Config::from_env();

    info!("Starting server on {}:{}", config.host, config.port);

    // Setup database
    tracing::info!("Connecting to database: {}", config.database_url);
    let db_config = k_core::db::DatabaseConfig {
        url: config.database_url.clone(),
        max_connections: 5,
        min_connections: 1,
        acquire_timeout: StdDuration::from_secs(30),
    };

    let db_pool = k_core::db::connect(&db_config).await?;

    run_migrations(&db_pool).await?;

    let user_repo = build_user_repository(&db_pool).await?;
    let user_service = UserService::new(user_repo.clone());

    let state = AppState::new(user_service, config.clone());

    let session_store = build_session_store(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    session_store
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in prod
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    let auth_layer = setup_auth_layer(session_layer, user_repo).await?;

    let server_config = ServerConfig {
        cors_origins: config.cors_allowed_origins.clone(),
        session_secret: Some(config.session_secret.clone()),
    };

    let app = Router::new()
        .nest("/api/v1", routes::api_v1_router())
        .layer(auth_layer)
        .with_state(state);

    let app = apply_standard_middleware(app, &server_config);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("🚀 API server running at http://{}", addr);
    tracing::info!("🔒 Authentication enabled (axum-login)");
    tracing::info!("📝 API endpoints available at /api/v1/...");

    axum::serve(listener, app).await?;

    Ok(())
}
