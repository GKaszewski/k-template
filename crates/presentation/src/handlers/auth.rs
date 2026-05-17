use axum::{extract::State, http::StatusCode, Json};
use api_types::{
    requests::{LoginRequest, RegisterRequest},
    responses::{AuthResponse, UserResponse},
};
use crate::{errors::AppError, extractors::{JwtClaims, ValidatedJson}, state::AppState};

#[utoipa::path(
    post, path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 409, description = "Email already taken"),
        (status = 422, description = "Validation error")
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let user = state.register_uc.execute(&req.email, &req.password).await?;
    let token = state.token_issuer.issue(&user.id, &user.role).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(AuthResponse { token, user: UserResponse::from_domain(&user) })))
}

#[utoipa::path(
    post, path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (user, token) = state.login_uc.execute(&req.email, &req.password).await?;
    Ok(Json(AuthResponse { token, user: UserResponse::from_domain(&user) }))
}

#[utoipa::path(
    get, path = "/api/v1/auth/me",
    security(("bearer_token" = [])),
    responses(
        (status = 200, description = "Current user profile", body = UserResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn me(
    State(state): State<AppState>,
    claims: JwtClaims,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.get_profile_uc.execute(&claims.user_id).await?;
    Ok(Json(UserResponse::from_domain(&user)))
}
