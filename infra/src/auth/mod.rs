//! Authentication infrastructure
//!
//! This module contains the concrete implementation of authentication mechanisms.

#[cfg(feature = "auth-axum-login")]
pub mod backend {
    use std::sync::Arc;

    use axum_login::{AuthnBackend, UserId};
    use password_auth::verify_password;
    use serde::{Deserialize, Serialize};
    use tower_sessions::SessionManagerLayer;
    use uuid::Uuid;

    use domain::{User, UserRepository};

    // We use the same session store as defined in infra
    use crate::session_store::InfraSessionStore;

    /// Wrapper around domain User to implement AuthUser
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AuthUser(pub User);

    impl axum_login::AuthUser for AuthUser {
        type Id = Uuid;

        fn id(&self) -> Self::Id {
            self.0.id
        }

        fn session_auth_hash(&self) -> &[u8] {
            // Use password hash to invalidate sessions if password changes
            self.0
                .password_hash
                .as_ref()
                .map(|s| s.as_bytes())
                .unwrap_or(&[])
        }
    }

    #[derive(Clone)]
    pub struct AuthBackend {
        pub user_repo: Arc<dyn UserRepository>,
    }

    impl AuthBackend {
        pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
            Self { user_repo }
        }
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Credentials {
        pub email: String,
        pub password: String,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum AuthError {
        #[error(transparent)]
        Anyhow(#[from] anyhow::Error),
    }

    impl AuthnBackend for AuthBackend {
        type User = AuthUser;
        type Credentials = Credentials;
        type Error = AuthError;

        async fn authenticate(
            &self,
            creds: Self::Credentials,
        ) -> Result<Option<Self::User>, Self::Error> {
            let user = self
                .user_repo
                .find_by_email(&creds.email)
                .await
                .map_err(|e| AuthError::Anyhow(anyhow::anyhow!(e)))?;

            if let Some(user) = user {
                if let Some(hash) = &user.password_hash {
                    // Verify password
                    if verify_password(&creds.password, hash).is_ok() {
                        return Ok(Some(AuthUser(user)));
                    }
                }
            }

            Ok(None)
        }

        async fn get_user(
            &self,
            user_id: &UserId<Self>,
        ) -> Result<Option<Self::User>, Self::Error> {
            let user = self
                .user_repo
                .find_by_id(*user_id)
                .await
                .map_err(|e| AuthError::Anyhow(anyhow::anyhow!(e)))?;

            Ok(user.map(AuthUser))
        }
    }

    pub type AuthSession = axum_login::AuthSession<AuthBackend>;
    pub type AuthManagerLayer = axum_login::AuthManagerLayer<AuthBackend, InfraSessionStore>;

    pub async fn setup_auth_layer(
        session_layer: SessionManagerLayer<InfraSessionStore>,
        user_repo: Arc<dyn UserRepository>,
    ) -> Result<AuthManagerLayer, AuthError> {
        let backend = AuthBackend::new(user_repo);

        let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();
        Ok(auth_layer)
    }
}
