//! Domain Services
//!
//! Services contain the business logic of the application.

use std::sync::Arc;
use uuid::Uuid;

use crate::entities::User;
use crate::errors::{DomainError, DomainResult};
use crate::repositories::UserRepository;
use crate::value_objects::Email;

/// Service for managing users
pub struct UserService {
    user_repository: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }

    pub async fn find_or_create(&self, subject: &str, email: &str) -> DomainResult<User> {
        // 1. Try to find by subject (OIDC id)
        if let Some(user) = self.user_repository.find_by_subject(subject).await? {
            return Ok(user);
        }

        // 2. Try to find by email
        if let Some(mut user) = self.user_repository.find_by_email(email).await? {
            // Link subject if missing (account linking logic)
            if user.subject != subject {
                user.subject = subject.to_string();
                self.user_repository.save(&user).await?;
            }
            return Ok(user);
        }

        // 3. Create new user
        let email = Email::try_from(email)?;
        let user = User::new(subject, email);
        self.user_repository.save(&user).await?;

        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DomainResult<User> {
        self.user_repository
            .find_by_id(id)
            .await?
            .ok_or(DomainError::UserNotFound(id))
    }

    pub async fn find_by_email(&self, email: &str) -> DomainResult<Option<User>> {
        self.user_repository.find_by_email(email).await
    }
}
