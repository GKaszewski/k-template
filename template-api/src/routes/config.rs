use axum::{Json, Router, routing::get};
use crate::dto::ConfigResponse;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_config))
}

async fn get_config() -> Json<ConfigResponse> {
    Json(ConfigResponse {
        allow_registration: true, // Default to true for template
    })
}
