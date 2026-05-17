use async_trait::async_trait;
use domain::{
    entities::User,
    errors::DomainError,
    ports::UserRepository,
    value_objects::{Email, PasswordHash, Role, UserId},
};
use std::str::FromStr;
use crate::db::PgPool;

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let row = sqlx::query!(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE id = $1",
            *id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        row.map(|r| Ok(User {
            id: UserId::from_uuid(r.id),
            email: Email::new(r.email)?,
            password_hash: PasswordHash::from_hash(r.password_hash),
            role: Role::from_str(&r.role).map_err(DomainError::Internal)?,
            created_at: r.created_at,
        }))
        .transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let row = sqlx::query!(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE email = $1",
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        row.map(|r| Ok(User {
            id: UserId::from_uuid(r.id),
            email: Email::new(r.email)?,
            password_hash: PasswordHash::from_hash(r.password_hash),
            role: Role::from_str(&r.role).map_err(DomainError::Internal)?,
            created_at: r.created_at,
        }))
        .transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, role, created_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (id) DO UPDATE SET
               email = EXCLUDED.email,
               password_hash = EXCLUDED.password_hash,
               role = EXCLUDED.role",
            *user.id.as_uuid(),
            user.email.as_str(),
            user.password_hash.as_str(),
            user.role.to_string(),
            user.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: &UserId) -> Result<(), DomainError> {
        sqlx::query!("DELETE FROM users WHERE id = $1", *id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }
}
