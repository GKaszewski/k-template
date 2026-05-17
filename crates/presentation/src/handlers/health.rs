use axum::{http::StatusCode, Json};
use serde_json::json;

#[utoipa::path(get, path = "/health", responses((status = 200, description = "Service is healthy")))]
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}
