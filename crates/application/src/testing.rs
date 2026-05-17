use std::collections::HashMap;
use async_trait::async_trait;
use tokio::sync::Mutex;
use domain::{
    entities::User,
    errors::DomainError,
    ports::{PasswordHasher, TokenIssuer, UserRepository},
    value_objects::{Email, PasswordHash, Role, UserId},
};

pub struct InMemoryUserRepository {
    users: Mutex<HashMap<String, User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self { users: Mutex::new(HashMap::new()) }
    }

    pub async fn all(&self) -> Vec<User> {
        self.users.lock().await.values().cloned().collect()
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        Ok(self.users.lock().await.get(&id.to_string()).cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        Ok(self.users.lock().await.values()
            .find(|u| u.email.as_str() == email.as_str())
            .cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().await.insert(user.id.to_string(), user.clone());
        Ok(())
    }

    async fn delete(&self, id: &UserId) -> Result<(), DomainError> {
        self.users.lock().await.remove(&id.to_string());
        Ok(())
    }
}

pub struct StubPasswordHasher;

#[async_trait]
impl PasswordHasher for StubPasswordHasher {
    async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError> {
        Ok(PasswordHash::from_hash(format!("hashed:{password}")))
    }
    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError> {
        Ok(hash.as_str() == format!("hashed:{password}"))
    }
}

pub struct StubTokenIssuer;

#[async_trait]
impl TokenIssuer for StubTokenIssuer {
    async fn issue(&self, user_id: &UserId, _role: &Role) -> Result<String, DomainError> {
        Ok(format!("token:{user_id}"))
    }
    async fn verify(&self, token: &str) -> Result<(UserId, Role), DomainError> {
        let id_str = token.strip_prefix("token:").ok_or_else(|| {
            DomainError::Unauthorized("Invalid stub token".to_string())
        })?;
        let uuid = uuid::Uuid::parse_str(id_str)
            .map_err(|_| DomainError::Unauthorized("Bad UUID in stub token".to_string()))?;
        Ok((UserId::from_uuid(uuid), Role::User))
    }
}
