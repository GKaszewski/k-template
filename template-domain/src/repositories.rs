//! Reference Repository ports (traits)
//! 
//! These traits define the interface for data persistence.

use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::User;
use crate::errors::DomainResult;

/// Repository port for User persistence
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Find a user by their internal ID
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<User>>;

    /// Find a user by their OIDC subject (used for authentication)
    async fn find_by_subject(&self, subject: &str) -> DomainResult<Option<User>>;

    /// Find a user by their email
    async fn find_by_email(&self, email: &str) -> DomainResult<Option<User>>;

    /// Save a new user or update an existing one
    async fn save(&self, user: &User) -> DomainResult<()>;

    /// Delete a user by their ID
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
}
