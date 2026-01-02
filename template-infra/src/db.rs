pub use k_core::db::{DatabaseConfig, DatabasePool};

pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::Error> {
    match pool {
        DatabasePool::Sqlite(pool) => {
            sqlx::migrate!("../migrations_sqlite").run(pool).await?;
        }
        #[cfg(feature = "postgres")]
        DatabasePool::Postgres(_) => {
            // Postgres migrations would go here
            tracing::warn!("Postgres migrations not implemented in template yet");
            // Pass through the types from the core library
            // This allows you to change k-core later without breaking imports in template-infra
            // The `pub use` statement cannot be placed inside a match arm.
            // It is already present at the top of the file.
        }
    }
    Ok(())
}
