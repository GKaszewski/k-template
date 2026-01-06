//! Application State
//!
//! Holds shared state for the application.

use axum::extract::FromRef;
#[cfg(feature = "auth-jwt")]
use infra::auth::jwt::{JwtConfig, JwtValidator};
#[cfg(feature = "auth-oidc")]
use infra::auth::oidc::OidcService;
use std::sync::Arc;

use crate::config::{AuthMode, Config};
use domain::UserService;

#[derive(Clone)]
pub struct AppState {
    pub user_service: Arc<UserService>,
    #[cfg(feature = "auth-oidc")]
    pub oidc_service: Option<Arc<OidcService>>,
    #[cfg(feature = "auth-jwt")]
    pub jwt_validator: Option<Arc<JwtValidator>>,
    pub config: Arc<Config>,
}

impl AppState {
    pub async fn new(user_service: UserService, config: Config) -> anyhow::Result<Self> {
        #[cfg(feature = "auth-oidc")]
        let oidc_service = if let (Some(issuer), Some(id), secret, Some(redirect), resource_id) = (
            &config.oidc_issuer,
            &config.oidc_client_id,
            &config.oidc_client_secret,
            &config.oidc_redirect_url,
            &config.oidc_resource_id,
        ) {
            tracing::info!("Initializing OIDC service with issuer: {}", issuer);
            Some(Arc::new(
                OidcService::new(
                    issuer.clone(),
                    id.clone(),
                    secret.clone().unwrap_or_default(),
                    redirect.clone(),
                    resource_id.clone(),
                )
                .await?,
            ))
        } else {
            None
        };

        #[cfg(feature = "auth-jwt")]
        let jwt_validator = if matches!(config.auth_mode, AuthMode::Jwt | AuthMode::Both) {
            // Use provided secret or fall back to a development secret
            let secret = if let Some(ref s) = config.jwt_secret {
                if s.is_empty() { None } else { Some(s.clone()) }
            } else {
                None
            };

            let secret = match secret {
                Some(s) => s,
                None => {
                    if config.is_production {
                        anyhow::bail!(
                            "JWT_SECRET is required when AUTH_MODE is 'jwt' or 'both' in production"
                        );
                    }
                    // Use a development-only default secret
                    tracing::warn!(
                        "⚠️  JWT_SECRET not set - using insecure development secret. DO NOT USE IN PRODUCTION!"
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
        } else {
            None
        };

        Ok(Self {
            user_service: Arc::new(user_service),
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
