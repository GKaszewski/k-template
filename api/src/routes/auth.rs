//! Authentication routes
//!
//! Provides login, register, logout, token, and OIDC endpoints.
//! All authentication is JWT-based. OIDC state is stored in an encrypted cookie.

use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};

use crate::{
    dto::{LoginRequest, RegisterRequest, TokenResponse, UserResponse},
    error::ApiError,
    extractors::CurrentUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    let r = Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/logout", post(logout))
        .route("/me", get(me));

    #[cfg(feature = "auth-jwt")]
    let r = r.route("/token", post(get_token));

    #[cfg(feature = "auth-oidc")]
    let r = r
        .route("/login/oidc", get(oidc_login))
        .route("/callback", get(oidc_callback));

    r
}

/// Login with email + password → JWT token
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .user_service
        .find_by_email(payload.email.as_ref())
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    let hash = user
        .password_hash
        .as_deref()
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    if !infra::auth::verify_password(payload.password.as_ref(), hash) {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let token = create_jwt(&user, &state)?;

    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.config.jwt_expiry_hours * 3600,
        }),
    ))
}

/// Register a new local user → JWT token
async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let password_hash = infra::auth::hash_password(payload.password.as_ref());

    let user = state
        .user_service
        .create_local(payload.email.as_ref(), &password_hash)
        .await?;

    let token = create_jwt(&user, &state)?;

    Ok((
        StatusCode::CREATED,
        Json(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.config.jwt_expiry_hours * 3600,
        }),
    ))
}

/// Logout — JWT is stateless; instruct the client to drop the token
async fn logout() -> impl IntoResponse {
    StatusCode::OK
}

/// Get current user info from JWT
async fn me(CurrentUser(user): CurrentUser) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(UserResponse {
        id: user.id,
        email: user.email.into_inner(),
        created_at: user.created_at,
    }))
}

/// Issue a new JWT for the currently authenticated user (OIDC→JWT exchange or token refresh)
#[cfg(feature = "auth-jwt")]
async fn get_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, ApiError> {
    let token = create_jwt(&user, &state)?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expiry_hours * 3600,
    }))
}

/// Helper: create JWT for a user
#[cfg(feature = "auth-jwt")]
fn create_jwt(user: &domain::User, state: &AppState) -> Result<String, ApiError> {
    let validator = state
        .jwt_validator
        .as_ref()
        .ok_or_else(|| ApiError::Internal("JWT not configured".to_string()))?;

    validator
        .create_token(user)
        .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))
}

#[cfg(not(feature = "auth-jwt"))]
fn create_jwt(_user: &domain::User, _state: &AppState) -> Result<String, ApiError> {
    Err(ApiError::Internal("JWT feature not enabled".to_string()))
}

// ============================================================================
// OIDC Routes
// ============================================================================

#[cfg(feature = "auth-oidc")]
#[derive(serde::Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

/// Start OIDC login: generate authorization URL and store state in encrypted cookie
#[cfg(feature = "auth-oidc")]
async fn oidc_login(
    State(state): State<AppState>,
    jar: axum_extra::extract::PrivateCookieJar,
) -> Result<impl IntoResponse, ApiError> {
    use axum::http::header;
    use axum::response::Response;
    use axum_extra::extract::cookie::{Cookie, SameSite};

    let service = state
        .oidc_service
        .as_ref()
        .ok_or(ApiError::Internal("OIDC not configured".into()))?;

    let (auth_data, oidc_state) = service.get_authorization_url();

    let state_json = serde_json::to_string(&oidc_state)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize OIDC state: {}", e)))?;

    let cookie = Cookie::build(("oidc_state", state_json))
        .max_age(time::Duration::minutes(5))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.config.secure_cookie)
        .path("/")
        .build();

    let updated_jar = jar.add(cookie);

    let redirect = axum::response::Redirect::to(auth_data.url.as_str()).into_response();
    let (mut parts, body) = redirect.into_parts();
    parts.headers.insert(
        header::CACHE_CONTROL,
        "no-cache, no-store, must-revalidate".parse().unwrap(),
    );
    parts
        .headers
        .insert(header::PRAGMA, "no-cache".parse().unwrap());
    parts.headers.insert(header::EXPIRES, "0".parse().unwrap());

    Ok((updated_jar, Response::from_parts(parts, body)))
}

/// Handle OIDC callback: verify state cookie, complete exchange, issue JWT, clear cookie
#[cfg(feature = "auth-oidc")]
async fn oidc_callback(
    State(state): State<AppState>,
    jar: axum_extra::extract::PrivateCookieJar,
    axum::extract::Query(params): axum::extract::Query<CallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    use infra::auth::oidc::OidcState;

    let service = state
        .oidc_service
        .as_ref()
        .ok_or(ApiError::Internal("OIDC not configured".into()))?;

    // Read and decrypt OIDC state from cookie
    let cookie = jar
        .get("oidc_state")
        .ok_or(ApiError::Validation("Missing OIDC state cookie".into()))?;

    let oidc_state: OidcState = serde_json::from_str(cookie.value())
        .map_err(|_| ApiError::Validation("Invalid OIDC state cookie".into()))?;

    // Verify CSRF token
    if params.state != oidc_state.csrf_token.as_ref() {
        return Err(ApiError::Validation("Invalid CSRF token".into()));
    }

    // Complete OIDC exchange
    let oidc_user = service
        .resolve_callback(
            domain::AuthorizationCode::new(params.code),
            oidc_state.nonce,
            oidc_state.pkce_verifier,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let user = state
        .user_service
        .find_or_create(&oidc_user.subject, &oidc_user.email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Clear the OIDC state cookie
    let cleared_jar = jar.remove(axum_extra::extract::cookie::Cookie::from("oidc_state"));

    let token = create_jwt(&user, &state)?;

    Ok((
        cleared_jar,
        Json(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.config.jwt_expiry_hours * 3600,
        }),
    ))
}
