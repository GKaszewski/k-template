//! Application State
//!
//! Holds shared state for the application.

use axum::extract::FromRef;
use std::sync::Arc;

use crate::config::Config;
use template_domain::UserService;

#[derive(Clone)]
pub struct AppState {
    pub user_service: Arc<UserService>,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(user_service: UserService, config: Config) -> Self {
        Self {
            user_service: Arc::new(user_service),
            config: Arc::new(config),
        }
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
