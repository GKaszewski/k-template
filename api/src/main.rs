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
    // 1. Initialize logging
    logging::init("template-api");

    // 2. Load configuration
    //   We use dotenvy explicitly here since config crate might not pick up .env automatically reliably
    dotenvy::dotenv().ok();
    let config = Config::new().expect("Failed to load configuration");

    info!("Starting server on {}:{}", config.host, config.port);

    // 3. Connect to database
    // k-core handles the "Which DB are we using?" logic internally based on feature flags
    // and returns the correct Enum variant.
    let db_config = k_core::db::DatabaseConfig {
        url: config.database_url.clone(),
        max_connections: 5,
        acquire_timeout: StdDuration::from_secs(30),
    };

    // Returns k_core::db::DatabasePool
    let db_pool = k_core::db::connect(&db_config).await?;

    // 4. Run migrations (using the re-export if you kept it, or direct k_core)
    infra::db::run_migrations(&db_pool).await?;

    // 5. Initialize Services
    let user_repo = build_user_repository(&db_pool).await?;
    let user_service = UserService::new(user_repo.clone());

    // 6. Setup Session Store
    let session_store = build_session_store(&db_pool).await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(1)));

    // 7. Setup Auth
    let auth_layer = auth::setup_auth_layer(session_layer, user_repo.clone()).await?;

    // 8. Create App State
    let state = AppState::new(user_service, config.clone());

    // 9. Build Router
    let app = routes::api_v1_router().layer(auth_layer).with_state(state);

    // 10. Start Server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
