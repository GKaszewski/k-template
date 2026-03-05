//! API Server Entry Point
//!
//! Configures and starts the HTTP server with JWT-based authentication.

use std::net::SocketAddr;
use std::time::Duration as StdDuration;

use axum::Router;
use domain::UserService;
use infra::factory::build_user_repository;
use infra::run_migrations;
use k_core::http::server::{ServerConfig, apply_standard_middleware};
use k_core::logging;
use tokio::net::TcpListener;
use tracing::info;

mod config;
mod dto;
mod error;
mod extractors;
mod routes;
mod state;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init("api");

    let config = Config::from_env();

    info!("Starting server on {}:{}", config.host, config.port);

    // Setup database
    tracing::info!("Connecting to database: {}", config.database_url);

    #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
    let db_type = k_core::db::DbType::Sqlite;

    #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
    let db_type = k_core::db::DbType::Postgres;

    // Both features enabled: fall back to URL inspection at runtime
    #[cfg(all(feature = "sqlite", feature = "postgres"))]
    let db_type = if config.database_url.starts_with("postgres") {
        k_core::db::DbType::Postgres
    } else {
        k_core::db::DbType::Sqlite
    };

    let db_config = k_core::db::DatabaseConfig {
        db_type,
        url: config.database_url.clone(),
        max_connections: config.db_max_connections,
        min_connections: config.db_min_connections,
        acquire_timeout: StdDuration::from_secs(30),
    };

    let db_pool = k_core::db::connect(&db_config).await?;
    run_migrations(&db_pool).await?;

    let user_repo = build_user_repository(&db_pool).await?;
    let user_service = UserService::new(user_repo);

    let state = AppState::new(user_service, config.clone()).await?;

    let server_config = ServerConfig {
        cors_origins: config.cors_allowed_origins.clone(),
    };

    let app = Router::new()
        .nest("/api/v1", routes::api_v1_router())
        .with_state(state);

    let app = apply_standard_middleware(app, &server_config);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("🚀 API server running at http://{}", addr);
    tracing::info!("🔒 Authentication mode: JWT (Bearer token)");

    #[cfg(feature = "auth-jwt")]
    tracing::info!("  ✓ JWT auth enabled");

    #[cfg(feature = "auth-oidc")]
    tracing::info!("  ✓ OIDC integration enabled (stateless cookie state)");

    tracing::info!("📝 API endpoints available at /api/v1/...");

    axum::serve(listener, app).await?;

    Ok(())
}
