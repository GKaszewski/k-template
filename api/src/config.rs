//! Application Configuration
//!
//! Loads configuration from environment variables.

use std::env;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub session_secret: String,
    pub cors_allowed_origins: Vec<String>,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_host")]
    pub host: String,
}

fn default_port() -> u16 {
    3000
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default())
            //.add_source(config::File::with_name(".env").required(false)) // Optional .env file
            .build()?
            .try_deserialize()
    }

    pub fn from_env() -> Self {
        // Load .env file if it exists, ignore errors if it doesn't
        let _ = dotenvy::dotenv();

        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string());

        let session_secret = env::var("SESSION_SECRET").unwrap_or_else(|_| {
            "k-notes-super-secret-key-must-be-at-least-64-bytes-long!!!!".to_string()
        });

        let cors_origins_str = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173".to_string());

        let cors_allowed_origins = cors_origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            host,
            port,
            database_url,
            session_secret,
            cors_allowed_origins,
        }
    }
}
