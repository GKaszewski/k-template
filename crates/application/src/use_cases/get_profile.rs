use std::sync::Arc;
use domain::{entities::User, errors::DomainError, ports::UserRepository, value_objects::UserId};

pub struct GetProfile {
    repo: Arc<dyn UserRepository>,
}

impl GetProfile {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self { Self { repo } }

    pub async fn execute(&self, user_id: &UserId) -> Result<User, DomainError> {
        self.repo.find_by_id(user_id).await?
            .ok_or_else(|| DomainError::NotFound(format!("User {user_id} not found")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{InMemoryUserRepository, StubPasswordHasher};
    use crate::use_cases::register::RegisterUser;

    #[tokio::test]
    async fn get_profile_returns_existing_user() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let r = RegisterUser::new(repo.clone(), Arc::new(StubPasswordHasher));
        let user = r.execute("user@example.com", "password123").await.unwrap();
        let uc = GetProfile::new(repo);
        let found = uc.execute(&user.id).await.unwrap();
        assert_eq!(found.id, user.id);
    }

    #[tokio::test]
    async fn get_profile_returns_not_found() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let uc = GetProfile::new(repo);
        let result = uc.execute(&UserId::new()).await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }
}
