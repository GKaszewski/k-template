use std::sync::Arc;
use domain::{
    entities::User,
    errors::DomainError,
    ports::{PasswordHasher, TokenIssuer, UserRepository},
    value_objects::Email,
};

pub struct LoginUser {
    repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    issuer: Arc<dyn TokenIssuer>,
}

impl LoginUser {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        issuer: Arc<dyn TokenIssuer>,
    ) -> Self {
        Self { repo, hasher, issuer }
    }

    pub async fn execute(&self, email: &str, password: &str) -> Result<(User, String), DomainError> {
        let email = Email::new(email)?;
        let user = self.repo.find_by_email(&email).await?
            .ok_or_else(|| DomainError::Unauthorized("Invalid credentials".to_string()))?;
        let valid = self.hasher.verify(password, &user.password_hash).await?;
        if !valid {
            return Err(DomainError::Unauthorized("Invalid credentials".to_string()));
        }
        let token = self.issuer.issue(&user.id, &user.role).await?;
        Ok((user, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{InMemoryUserRepository, StubPasswordHasher, StubTokenIssuer};
    use crate::use_cases::register::RegisterUser;

    async fn seeded_repo() -> Arc<InMemoryUserRepository> {
        let repo = Arc::new(InMemoryUserRepository::new());
        let r = RegisterUser::new(repo.clone(), Arc::new(StubPasswordHasher));
        r.execute("user@example.com", "password123").await.unwrap();
        repo
    }

    #[tokio::test]
    async fn login_returns_user_and_token() {
        let repo = seeded_repo().await;
        let uc = LoginUser::new(repo, Arc::new(StubPasswordHasher), Arc::new(StubTokenIssuer));
        let (user, token) = uc.execute("user@example.com", "password123").await.unwrap();
        assert_eq!(user.email.as_str(), "user@example.com");
        assert!(token.starts_with("token:"));
    }

    #[tokio::test]
    async fn login_rejects_wrong_password() {
        let repo = seeded_repo().await;
        let uc = LoginUser::new(repo, Arc::new(StubPasswordHasher), Arc::new(StubTokenIssuer));
        let result = uc.execute("user@example.com", "wrongpassword").await;
        assert!(matches!(result, Err(DomainError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn login_rejects_unknown_email() {
        let repo = seeded_repo().await;
        let uc = LoginUser::new(repo, Arc::new(StubPasswordHasher), Arc::new(StubTokenIssuer));
        let result = uc.execute("nobody@example.com", "password123").await;
        assert!(matches!(result, Err(DomainError::Unauthorized(_))));
    }
}
