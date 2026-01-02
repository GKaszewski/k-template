use std::net::SocketAddr;
use std::time::Duration as StdDuration;

use template_domain::UserService;
use template_infra::factory::build_user_repository;
use template_infra::{db, session_store};
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
    let db_config = db::DatabaseConfig {
        url: config.database_url.clone(),
        max_connections: 5,
        min_connections: 1,
        acquire_timeout: StdDuration::from_secs(30),
    };
    
    // We assume generic connection logic in k-core/template-infra
    // But here we use k-core via template-infra
    #[cfg(feature = "sqlite")]
    let pool = k_core::db::connect_sqlite(&db_config.url).await?; 
    
    #[cfg(feature = "postgres")]
    let pool = k_core::db::connect_postgres(&db_config.url).await?;

    #[cfg(feature = "sqlite")]
    let db_pool = template_infra::db::DatabasePool::Sqlite(pool.clone());
    #[cfg(feature = "postgres")]
    let db_pool = template_infra::db::DatabasePool::Postgres(pool.clone());

    // 4. Run migrations
    db::run_migrations(&db_pool).await?;

    // 5. Initialize Services
    let user_repo = build_user_repository(&db_pool).await?;
    let user_service = UserService::new(user_repo.clone());
    
    // 6. Setup Session Store
    #[cfg(feature = "sqlite")]
    let session_store = session_store::InfraSessionStore::Sqlite(
        tower_sessions_sqlx_store::SqliteStore::new(pool.clone())
    );
    #[cfg(feature = "postgres")]
    let session_store = session_store::InfraSessionStore::Postgres(
        tower_sessions_sqlx_store::PostgresStore::new(pool.clone())
    );
    
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(1)));

    // 7. Setup Auth
    let auth_layer = auth::setup_auth_layer(session_layer, user_repo.clone()).await?;

    // 8. Create App State
    let state = AppState::new(user_service, config.clone());

    // 9. Build Router
    let app = routes::api_v1_router()
        .layer(auth_layer)
        .with_state(state);

    // 10. Start Server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
