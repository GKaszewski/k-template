//! Database connection pool management

use sqlx::Pool;
#[cfg(feature = "postgres")]
use sqlx::Postgres;
#[cfg(feature = "sqlite")]
use sqlx::Sqlite;
#[cfg(feature = "sqlite")]
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
#[cfg(feature = "sqlite")]
use std::str::FromStr;
use std::time::Duration;

/// Configuration for the database connection
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:data.db?mode=rwc".to_string(),
            max_connections: 5,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(5),
        }
    }
}

impl DatabaseConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn in_memory() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 1, // SQLite in-memory is single-connection
            min_connections: 1,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub enum DatabasePool {
    #[cfg(feature = "sqlite")]
    Sqlite(Pool<Sqlite>),
    #[cfg(feature = "postgres")]
    Postgres(Pool<Postgres>),
}

/// Create a database connection pool
#[cfg(feature = "sqlite")]
pub async fn create_pool(config: &DatabaseConfig) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(&config.url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect_with(options)
        .await?;

    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::Error> {
    match pool {
        #[cfg(feature = "sqlite")]
        DatabasePool::Sqlite(pool) => {
            sqlx::migrate!("../migrations").run(pool).await?;
        }
        #[cfg(feature = "postgres")]
        DatabasePool::Postgres(_pool) => {
            // Placeholder for Postgres migrations
            // sqlx::migrate!("../migrations/postgres").run(_pool).await?;
            tracing::warn!("Postgres migrations not yet implemented");
            return Err(sqlx::Error::Configuration(
                "Postgres migrations not yet implemented".into(),
            ));
        }
        #[allow(unreachable_patterns)]
        _ => {
            return Err(sqlx::Error::Configuration(
                "No database feature enabled".into(),
            ));
        }
    }

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_in_memory_pool() {
        let config = DatabaseConfig::in_memory();
        let pool = create_pool(&config).await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let config = DatabaseConfig::in_memory();
        let pool = create_pool(&config).await.unwrap();
        let db_pool = DatabasePool::Sqlite(pool);
        let result = run_migrations(&db_pool).await;
        assert!(result.is_ok());
    }
}
