use async_trait::async_trait;
use domain::{
    entities::User,
    errors::DomainError,
    ports::UserRepository,
    value_objects::{Email, PasswordHash, Role, UserId},
};
use std::str::FromStr;
use crate::db::SqlitePool;

pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.to_string();
        let row = sqlx::query!(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE id = ?",
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        row.map(|r| row_to_user(r.id, r.email, r.password_hash, r.role, r.created_at))
           .transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.as_str().to_owned();
        let row = sqlx::query!(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE email = ?",
            email_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        row.map(|r| row_to_user(r.id, r.email, r.password_hash, r.role, r.created_at))
           .transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        let id = user.id.to_string();
        let email = user.email.as_str().to_owned();
        let hash = user.password_hash.as_str().to_owned();
        let role = user.role.to_string();
        let created_at = user.created_at.to_rfc3339();

        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, role, created_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               email = excluded.email,
               password_hash = excluded.password_hash,
               role = excluded.role",
            id, email, hash, role, created_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: &UserId) -> Result<(), DomainError> {
        let id_str = id.to_string();
        sqlx::query!("DELETE FROM users WHERE id = ?", id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }
}

fn row_to_user(
    id: String,
    email: String,
    password_hash: String,
    role: String,
    created_at: String,
) -> Result<User, DomainError> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| DomainError::Internal(e.to_string()))?;
    let email = Email::new(email)?;
    let role = Role::from_str(&role).map_err(DomainError::Internal)?;
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .with_timezone(&chrono::Utc);
    Ok(User { id: UserId::from_uuid(uuid), email, password_hash: PasswordHash::from_hash(password_hash), role, created_at })
}
