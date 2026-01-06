//! Application State
//!
//! Holds shared state for the application.

use axum::extract::FromRef;
#[cfg(feature = "auth-oidc")]
use infra::auth::oidc::OidcService;
use std::sync::Arc;

use crate::config::Config;
use domain::UserService;

#[derive(Clone)]
pub struct AppState {
    pub user_service: Arc<UserService>,
    #[cfg(feature = "auth-oidc")]
    pub oidc_service: Option<Arc<OidcService>>,

    pub config: Arc<Config>,
}

impl AppState {
    pub async fn new(user_service: UserService, config: Config) -> anyhow::Result<Self> {
        #[cfg(feature = "auth-oidc")]
        let oidc_service = if let (Some(issuer), Some(id), Some(secret), Some(redirect)) = (
            &config.oidc_issuer,
            &config.oidc_client_id,
            &config.oidc_client_secret,
            &config.oidc_redirect_url,
        ) {
            tracing::info!("Initializing OIDC service with issuer: {}", issuer);
            Some(Arc::new(
                OidcService::new(issuer.clone(), id.clone(), secret.clone(), redirect.clone())
                    .await?,
            ))
        } else {
            None
        };

        Ok(Self {
            user_service: Arc::new(user_service),
            #[cfg(feature = "auth-oidc")]
            oidc_service,
            config: Arc::new(config),
        })
    }
}

impl FromRef<AppState> for Arc<UserService> {
    fn from_ref(input: &AppState) -> Self {
        input.user_service.clone()
    }
}

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(input: &AppState) -> Self {
        input.config.clone()
    }
}
