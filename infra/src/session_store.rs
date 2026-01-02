use async_trait::async_trait;
use sqlx;
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
};
#[cfg(feature = "postgres")]
use tower_sessions_sqlx_store::PostgresStore;
#[cfg(feature = "sqlite")]
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Clone, Debug)]
pub enum InfraSessionStore {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteStore),
    #[cfg(feature = "postgres")]
    Postgres(PostgresStore),
}

#[async_trait]
impl SessionStore for InfraSessionStore {
    async fn save(&self, session_record: &Record) -> tower_sessions::session_store::Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(store) => store.save(session_record).await,
            #[cfg(feature = "postgres")]
            Self::Postgres(store) => store.save(session_record).await,
            #[allow(unreachable_patterns)]
            _ => Err(tower_sessions::session_store::Error::Backend(
                "No backend enabled".to_string(),
            )),
        }
    }

    async fn load(&self, session_id: &Id) -> tower_sessions::session_store::Result<Option<Record>> {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(store) => store.load(session_id).await,
            #[cfg(feature = "postgres")]
            Self::Postgres(store) => store.load(session_id).await,
            #[allow(unreachable_patterns)]
            _ => Err(tower_sessions::session_store::Error::Backend(
                "No backend enabled".to_string(),
            )),
        }
    }

    async fn delete(&self, session_id: &Id) -> tower_sessions::session_store::Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(store) => store.delete(session_id).await,
            #[cfg(feature = "postgres")]
            Self::Postgres(store) => store.delete(session_id).await,
            #[allow(unreachable_patterns)]
            _ => Err(tower_sessions::session_store::Error::Backend(
                "No backend enabled".to_string(),
            )),
        }
    }
}

impl InfraSessionStore {
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(store) => store.migrate().await,
            #[cfg(feature = "postgres")]
            Self::Postgres(store) => store.migrate().await,
            #[allow(unreachable_patterns)]
            _ => Err(sqlx::Error::Configuration("No backend enabled".into())),
        }
    }
}
