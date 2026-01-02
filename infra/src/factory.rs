use std::sync::Arc;

#[cfg(feature = "sqlite")]
use crate::SqliteUserRepository;
use crate::db::DatabasePool;
use domain::UserRepository;

use k_core::session::store::InfraSessionStore;

#[derive(Debug, thiserror::Error)]
pub enum FactoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] domain::DomainError),
}

pub type FactoryResult<T> = Result<T, FactoryError>;

pub async fn build_user_repository(pool: &DatabasePool) -> FactoryResult<Arc<dyn UserRepository>> {
    match pool {
        #[cfg(feature = "sqlite")]
        DatabasePool::Sqlite(pool) => Ok(Arc::new(SqliteUserRepository::new(pool.clone()))),
        #[cfg(feature = "postgres")]
        DatabasePool::Postgres(pool) => Ok(Arc::new(
            crate::user_repository::PostgresUserRepository::new(pool.clone()),
        )),
        #[allow(unreachable_patterns)]
        _ => Err(FactoryError::NotImplemented(
            "No database feature enabled".to_string(),
        )),
    }
}

pub async fn build_session_store(
    pool: &DatabasePool,
) -> FactoryResult<crate::session_store::InfraSessionStore> {
    Ok(match pool {
        #[cfg(feature = "sqlite")]
        DatabasePool::Sqlite(p) => {
            InfraSessionStore::Sqlite(tower_sessions_sqlx_store::SqliteStore::new(p.clone()))
        }
        #[cfg(feature = "postgres")]
        DatabasePool::Postgres(p) => {
            InfraSessionStore::Postgres(tower_sessions_sqlx_store::PostgresStore::new(p.clone()))
        }
    })
}
