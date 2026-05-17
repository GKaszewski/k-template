use std::sync::Arc;
use domain::{
    entities::User,
    errors::DomainError,
    ports::{PasswordHasher, UserRepository},
    value_objects::{Email, UserId},
};

pub struct RegisterUser {
    repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl RegisterUser {
    pub fn new(repo: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { repo, hasher }
    }

    pub async fn execute(&self, email: &str, password: &str) -> Result<User, DomainError> {
        if password.len() < 8 {
            return Err(DomainError::Validation("Password must be at least 8 characters".to_string()));
        }
        let email = Email::new(email)?;
        if self.repo.find_by_email(&email).await?.is_some() {
            return Err(DomainError::Conflict(format!("Email {} is already registered", email.as_str())));
        }
        let hash = self.hasher.hash(password).await?;
        let user = User::new(UserId::new(), email, hash);
        self.repo.save(&user).await?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{InMemoryUserRepository, StubPasswordHasher};

    #[tokio::test]
    async fn register_creates_user() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let uc = RegisterUser::new(repo.clone(), Arc::new(StubPasswordHasher));
        let user = uc.execute("test@example.com", "password123").await.unwrap();
        assert_eq!(user.email.as_str(), "test@example.com");
        assert_eq!(repo.all().await.len(), 1);
    }

    #[tokio::test]
    async fn register_rejects_duplicate_email() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let uc = RegisterUser::new(repo.clone(), Arc::new(StubPasswordHasher));
        uc.execute("test@example.com", "password123").await.unwrap();
        let result = uc.execute("test@example.com", "different1").await;
        assert!(matches!(result, Err(DomainError::Conflict(_))));
    }

    #[tokio::test]
    async fn register_rejects_short_password() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let uc = RegisterUser::new(repo, Arc::new(StubPasswordHasher));
        let result = uc.execute("test@example.com", "short").await;
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn register_rejects_invalid_email() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let uc = RegisterUser::new(repo, Arc::new(StubPasswordHasher));
        let result = uc.execute("notanemail", "password123").await;
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }
}
