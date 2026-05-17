use std::sync::Arc;
use application::use_cases::{GetProfile, LoginUser, RegisterUser};
use domain::ports::TokenIssuer;

#[derive(Clone)]
pub struct AppState {
    pub register_uc: Arc<RegisterUser>,
    pub login_uc: Arc<LoginUser>,
    pub get_profile_uc: Arc<GetProfile>,
    pub token_issuer: Arc<dyn TokenIssuer>,
}

impl AppState {
    pub fn new(
        register_uc: Arc<RegisterUser>,
        login_uc: Arc<LoginUser>,
        get_profile_uc: Arc<GetProfile>,
        token_issuer: Arc<dyn TokenIssuer>,
    ) -> Self {
        Self { register_uc, login_uc, get_profile_uc, token_issuer }
    }
}
