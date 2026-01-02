pub use k_core::db::DatabasePool;

pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::Error> {
    match pool {
        #[cfg(feature = "sqlite")]
        DatabasePool::Sqlite(pool) => {
            // Point specifically to the sqlite folder
            sqlx::migrate!("../migrations_sqlite").run(pool).await?;
        }
        #[cfg(feature = "postgres")]
        DatabasePool::Postgres(pool) => {
            // Point specifically to the postgres folder
            sqlx::migrate!("../migrations_postgres").run(pool).await?;
        }
    }
    Ok(())
}
