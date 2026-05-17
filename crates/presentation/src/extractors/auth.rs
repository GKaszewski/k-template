use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use domain::value_objects::{Role, UserId};
use serde_json::json;
use crate::state::AppState;

pub struct JwtClaims {
    pub user_id: UserId,
    pub role: Role,
}

impl FromRequestParts<AppState> for JwtClaims {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Missing Authorization header" }))).into_response()
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid Authorization format" }))).into_response()
        })?;

        let (user_id, role) = state.token_issuer.verify(token).await.map_err(|_| {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid or expired token" }))).into_response()
        })?;

        Ok(JwtClaims { user_id, role })
    }
}
