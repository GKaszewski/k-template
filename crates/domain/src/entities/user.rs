use chrono::{DateTime, Utc};
use crate::value_objects::{Email, PasswordHash, Role, UserId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub password_hash: PasswordHash,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(id: UserId, email: Email, password_hash: PasswordHash) -> Self {
        Self { id, email, password_hash, role: Role::User, created_at: Utc::now() }
    }
}
