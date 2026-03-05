//! Application State
//!
//! Holds shared state for the application.

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
#[cfg(feature = "auth-jwt")]
use infra::auth::jwt::{JwtConfig, JwtValidator};
#[cfg(feature = "auth-oidc")]
use infra::auth::oidc::OidcService;
use std::sync::Arc;

use crate::config::Config;
use domain::UserService;

#[derive(Clone)]
pub struct AppState {
    pub user_service: Arc<UserService>,
    pub cookie_key: Key,
    #[cfg(feature = "auth-oidc")]
    pub oidc_service: Option<Arc<OidcService>>,
    #[cfg(feature = "auth-jwt")]
    pub jwt_validator: Option<Arc<JwtValidator>>,
    pub config: Arc<Config>,
}

impl AppState {
    pub async fn new(user_service: UserService, config: Config) -> anyhow::Result<Self> {
        let cookie_key = Key::derive_from(config.cookie_secret.as_bytes());

        #[cfg(feature = "auth-oidc")]
        let oidc_service = if let (Some(issuer), Some(id), secret, Some(redirect), resource_id) = (
            &config.oidc_issuer,
            &config.oidc_client_id,
            &config.oidc_client_secret,
            &config.oidc_redirect_url,
            &config.oidc_resource_id,
        ) {
            tracing::info!("Initializing OIDC service with issuer: {}", issuer);

            let issuer_url = domain::IssuerUrl::new(issuer)
                .map_err(|e| anyhow::anyhow!("Invalid OIDC issuer URL: {}", e))?;
            let client_id = domain::ClientId::new(id)
                .map_err(|e| anyhow::anyhow!("Invalid OIDC client ID: {}", e))?;
            let client_secret = secret.as_ref().map(|s| domain::ClientSecret::new(s));
            let redirect_url = domain::RedirectUrl::new(redirect)
                .map_err(|e| anyhow::anyhow!("Invalid OIDC redirect URL: {}", e))?;
            let resource = resource_id
                .as_ref()
                .map(|r| domain::ResourceId::new(r))
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid OIDC resource ID: {}", e))?;

            Some(Arc::new(
                OidcService::new(issuer_url, client_id, client_secret, redirect_url, resource)
                    .await?,
            ))
        } else {
            None
        };

        #[cfg(feature = "auth-jwt")]
        let jwt_validator = {
            let secret = match &config.jwt_secret {
                Some(s) if !s.is_empty() => s.clone(),
                _ => {
                    if config.is_production {
                        anyhow::bail!("JWT_SECRET is required in production");
                    }
                    tracing::warn!(
                        "⚠️  JWT_SECRET not set — using insecure development secret. DO NOT USE IN PRODUCTION!"
                    );
                    "k-template-dev-secret-not-for-production-use-only".to_string()
                }
            };

            tracing::info!("Initializing JWT validator");
            let jwt_config = JwtConfig::new(
                secret,
                config.jwt_issuer.clone(),
                config.jwt_audience.clone(),
                Some(config.jwt_expiry_hours),
                config.is_production,
            )?;
            Some(Arc::new(JwtValidator::new(jwt_config)))
        };

        Ok(Self {
            user_service: Arc::new(user_service),
            cookie_key,
            #[cfg(feature = "auth-oidc")]
            oidc_service,
            #[cfg(feature = "auth-jwt")]
            jwt_validator,
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

impl FromRef<AppState> for Key {
    fn from_ref(input: &AppState) -> Self {
        input.cookie_key.clone()
    }
}
