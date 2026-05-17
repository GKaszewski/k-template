use async_trait::async_trait;
use crate::{errors::DomainError, value_objects::{PasswordHash, Role, UserId}};

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError>;
    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
}

#[async_trait]
pub trait TokenIssuer: Send + Sync {
    async fn issue(&self, user_id: &UserId, role: &Role) -> Result<String, DomainError>;
    async fn verify(&self, token: &str) -> Result<(UserId, Role), DomainError>;
}
