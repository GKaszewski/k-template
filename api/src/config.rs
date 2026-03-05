//! Application Configuration
//!
//! Loads configuration from environment variables.

use std::env;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub cookie_secret: String,
    pub cors_allowed_origins: Vec<String>,
    pub port: u16,
    pub host: String,
    pub secure_cookie: bool,
    pub db_max_connections: u32,
    pub db_min_connections: u32,

    // OIDC configuration
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_redirect_url: Option<String>,
    pub oidc_resource_id: Option<String>,

    // JWT configuration
    pub jwt_secret: Option<String>,
    pub jwt_issuer: Option<String>,
    pub jwt_audience: Option<String>,
    pub jwt_expiry_hours: u64,

    /// Whether the application is running in production mode
    pub is_production: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string());

        // Cookie secret for PrivateCookieJar (OIDC state encryption).
        // Must be at least 64 bytes in production.
        let cookie_secret = env::var("COOKIE_SECRET").unwrap_or_else(|_| {
            "k-template-cookie-secret-key-must-be-at-least-64-bytes-long!!".to_string()
        });

        let cors_origins_str = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173".to_string());

        let cors_allowed_origins = cors_origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let secure_cookie = env::var("SECURE_COOKIE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let db_max_connections = env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let db_min_connections = env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let oidc_issuer = env::var("OIDC_ISSUER").ok();
        let oidc_client_id = env::var("OIDC_CLIENT_ID").ok();
        let oidc_client_secret = env::var("OIDC_CLIENT_SECRET").ok();
        let oidc_redirect_url = env::var("OIDC_REDIRECT_URL").ok();
        let oidc_resource_id = env::var("OIDC_RESOURCE_ID").ok();

        let jwt_secret = env::var("JWT_SECRET").ok();
        let jwt_issuer = env::var("JWT_ISSUER").ok();
        let jwt_audience = env::var("JWT_AUDIENCE").ok();
        let jwt_expiry_hours = env::var("JWT_EXPIRY_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);

        let is_production = env::var("PRODUCTION")
            .or_else(|_| env::var("RUST_ENV"))
            .map(|v| v.to_lowercase() == "production" || v == "1" || v == "true")
            .unwrap_or(false);

        Self {
            host,
            port,
            database_url,
            cookie_secret,
            cors_allowed_origins,
            secure_cookie,
            db_max_connections,
            db_min_connections,
            oidc_issuer,
            oidc_client_id,
            oidc_client_secret,
            oidc_redirect_url,
            oidc_resource_id,
            jwt_secret,
            jwt_issuer,
            jwt_audience,
            jwt_expiry_hours,
            is_production,
        }
    }
}
