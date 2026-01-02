//! SQLite implementation of UserRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use template_domain::{DomainError, DomainResult, Email, User, UserRepository};

/// SQLite adapter for UserRepository
#[cfg(feature = "sqlite")]
#[derive(Clone)]
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Row type for SQLite query results
#[derive(Debug, FromRow)]
struct UserRow {
    id: String,
    subject: String,
    email: String,
    password_hash: Option<String>,
    created_at: String,
}

impl TryFrom<UserRow> for User {
    type Error = DomainError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| DomainError::RepositoryError(format!("Invalid UUID: {}", e)))?;
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                // Fallback for SQLite datetime format
                chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc())
            })
            .map_err(|e| DomainError::RepositoryError(format!("Invalid datetime: {}", e)))?;

        // Parse email from string - it was validated when originally stored
        let email = Email::try_from(row.email)
            .map_err(|e| DomainError::RepositoryError(format!("Invalid email in DB: {}", e)))?;

        Ok(User::with_id(
            id,
            row.subject,
            email,
            row.password_hash,
            created_at,
        ))
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<User>> {
        let id_str = id.to_string();
        let row: Option<UserRow> = sqlx::query_as(
            "SELECT id, subject, email, password_hash, created_at FROM users WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_subject(&self, subject: &str) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            "SELECT id, subject, email, password_hash, created_at FROM users WHERE subject = ?",
        )
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_email(&self, email: &str) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            "SELECT id, subject, email, password_hash, created_at FROM users WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn save(&self, user: &User) -> DomainResult<()> {
        let id = user.id.to_string();
        let created_at = user.created_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO users (id, subject, email, password_hash, created_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                subject = excluded.subject,
                email = excluded.email,
                password_hash = excluded.password_hash
            "#,
        )
        .bind(&id)
        .bind(&user.subject)
        .bind(user.email.as_ref()) // Use .as_ref() to get the inner &str
        .bind(&user.password_hash)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::db::{DatabaseConfig, DatabasePool, run_migrations};
    use k_core::db::connect; // Import k_core::db::connect

    async fn setup_test_db() -> SqlitePool {
        let config = DatabaseConfig::in_memory();
        // connect returns DatabasePool directly now
        let db_pool = connect(&config).await.expect("Failed to create pool");
        run_migrations(&db_pool).await.unwrap();
        // Extract SqlitePool from DatabasePool for SqliteUserRepository
        match db_pool {
            DatabasePool::Sqlite(pool) => pool,
            _ => panic!("Expected SqlitePool for testing"),
        }
    }

    #[tokio::test]
    async fn test_save_and_find_user() {
        let pool = setup_test_db().await;
        let repo = SqliteUserRepository::new(pool);

        let email = Email::try_from("test@example.com").unwrap();
        let user = User::new("oidc|123", email);
        repo.save(&user).await.unwrap();

        let found = repo.find_by_id(user.id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.subject, "oidc|123");
        assert_eq!(found.email_str(), "test@example.com");
        assert!(found.password_hash.is_none());
    }

    #[tokio::test]
    async fn test_save_and_find_user_with_password() {
        let pool = setup_test_db().await;
        let repo = SqliteUserRepository::new(pool);

        let email = Email::try_from("local@example.com").unwrap();
        let user = User::new_local(email, "hashed_pw");
        repo.save(&user).await.unwrap();

        let found = repo.find_by_id(user.id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.email_str(), "local@example.com");
        assert_eq!(found.password_hash, Some("hashed_pw".to_string()));
    }

    #[tokio::test]
    async fn test_find_by_subject() {
        let pool = setup_test_db().await;
        let repo = SqliteUserRepository::new(pool);

        let email = Email::try_from("user@gmail.com").unwrap();
        let user = User::new("google|456", email);
        repo.save(&user).await.unwrap();

        let found = repo.find_by_subject("google|456").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let pool = setup_test_db().await;
        let repo = SqliteUserRepository::new(pool);

        let email = Email::try_from("delete@test.com").unwrap();
        let user = User::new("test|789", email);
        repo.save(&user).await.unwrap();
        repo.delete(user.id).await.unwrap();

        let found = repo.find_by_id(user.id).await.unwrap();
        assert!(found.is_none());
    }
}

/// PostgreSQL adapter for UserRepository
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: sqlx::Pool<sqlx::Postgres>,
}

#[cfg(feature = "postgres")]
impl PostgresUserRepository {
    pub fn new(pool: sqlx::Pool<sqlx::Postgres>) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<User>> {
        let id_str = id.to_string();
        let row: Option<UserRow> = sqlx::query_as(
            "SELECT id, subject, email, password_hash, created_at FROM users WHERE id = $1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_subject(&self, subject: &str) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            "SELECT id, subject, email, password_hash, created_at FROM users WHERE subject = $1",
        )
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_email(&self, email: &str) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            "SELECT id, subject, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        row.map(User::try_from).transpose()
    }

    async fn save(&self, user: &User) -> DomainResult<()> {
        let id = user.id.to_string();
        let created_at = user.created_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO users (id, subject, email, password_hash, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT(id) DO UPDATE SET
                subject = excluded.subject,
                email = excluded.email,
                password_hash = excluded.password_hash
            "#,
        )
        .bind(&id)
        .bind(&user.subject)
        .bind(user.email.as_ref())
        .bind(&user.password_hash)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(())
    }
}
