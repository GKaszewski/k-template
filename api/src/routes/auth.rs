//! Authentication routes
//!
//! Provides login, register, logout, and token endpoints.
//! Supports both session-based and JWT-based authentication.

#[cfg(feature = "auth-oidc")]
use axum::response::Response;
use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;
#[cfg(feature = "auth-oidc")]
use tower_sessions::Session;

#[cfg(feature = "auth-axum-login")]
use crate::config::AuthMode;
use crate::{
    dto::{LoginRequest, RegisterRequest, UserResponse},
    error::ApiError,
    extractors::CurrentUser,
    state::AppState,
};
#[cfg(feature = "auth-axum-login")]
use domain::DomainError;

/// Token response for JWT authentication
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Login response that can be either a user (session mode) or a token (JWT mode)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum LoginResponse {
    User(UserResponse),
    Token(TokenResponse),
}

pub fn router() -> Router<AppState> {
    let r = Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/logout", post(logout))
        .route("/me", get(me));

    // Add token endpoint for getting JWT from session
    #[cfg(feature = "auth-jwt")]
    let r = r.route("/token", post(get_token));

    #[cfg(feature = "auth-oidc")]
    let r = r
        .route("/login/oidc", get(oidc_login))
        .route("/callback", get(oidc_callback));

    r
}

/// Login endpoint
///
/// In session mode: Creates a session and returns user info
/// In JWT mode: Validates credentials and returns a JWT token
/// In both mode: Creates session AND returns JWT token
#[cfg(feature = "auth-axum-login")]
async fn login(
    State(state): State<AppState>,
    mut auth_session: crate::auth::AuthSession,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = match auth_session
        .authenticate(crate::auth::Credentials {
            email: payload.email,
            password: payload.password,
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        Some(user) => user,
        None => return Err(ApiError::Validation("Invalid credentials".to_string())),
    };

    let auth_mode = state.config.auth_mode;

    // In session or both mode, create session
    if matches!(auth_mode, AuthMode::Session | AuthMode::Both) {
        auth_session
            .login(&user)
            .await
            .map_err(|_| ApiError::Internal("Login failed".to_string()))?;
    }

    // In JWT or both mode, return token
    #[cfg(feature = "auth-jwt")]
    if matches!(auth_mode, AuthMode::Jwt | AuthMode::Both) {
        let token = create_jwt_for_user(&user.0, &state)?;
        return Ok((
            StatusCode::OK,
            Json(LoginResponse::Token(TokenResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: state.config.jwt_expiry_hours * 3600,
            })),
        ));
    }

    // Session mode: return user info
    Ok((
        StatusCode::OK,
        Json(LoginResponse::User(UserResponse {
            id: user.0.id,
            email: user.0.email.into_inner(),
            created_at: user.0.created_at,
        })),
    ))
}

/// Fallback login when auth-axum-login is not enabled
/// Without auth-axum-login, password-based authentication is not available.
/// Use OIDC login instead: GET /api/v1/auth/login/oidc
#[cfg(not(feature = "auth-axum-login"))]
async fn login(
    State(_state): State<AppState>,
    Json(_payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), ApiError> {
    Err(ApiError::Internal(
        "Password-based login not available. auth-axum-login feature is required. Use OIDC login at /api/v1/auth/login/oidc instead.".to_string(),
    ))
}

/// Register endpoint
#[cfg(feature = "auth-axum-login")]
async fn register(
    State(state): State<AppState>,
    mut auth_session: crate::auth::AuthSession,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Email is already validated by the newtype deserialization
    let email = payload.email;

    if state
        .user_service
        .find_by_email(email.as_ref())
        .await?
        .is_some()
    {
        return Err(ApiError::Domain(DomainError::UserAlreadyExists(
            email.as_ref().to_string(),
        )));
    }

    // Hash password
    let password_hash = infra::auth::backend::hash_password(payload.password.as_ref());

    // Create user with password
    let user = state
        .user_service
        .create_local(email.as_ref(), &password_hash)
        .await?;

    let auth_mode = state.config.auth_mode;

    // In session or both mode, create session
    if matches!(auth_mode, AuthMode::Session | AuthMode::Both) {
        let auth_user = crate::auth::AuthUser(user.clone());
        auth_session
            .login(&auth_user)
            .await
            .map_err(|_| ApiError::Internal("Login failed".to_string()))?;
    }

    // In JWT or both mode, return token
    #[cfg(feature = "auth-jwt")]
    if matches!(auth_mode, AuthMode::Jwt | AuthMode::Both) {
        let token = create_jwt_for_user(&user, &state)?;
        return Ok((
            StatusCode::CREATED,
            Json(LoginResponse::Token(TokenResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: state.config.jwt_expiry_hours * 3600,
            })),
        ));
    }

    Ok((
        StatusCode::CREATED,
        Json(LoginResponse::User(UserResponse {
            id: user.id,
            email: user.email.into_inner(),
            created_at: user.created_at,
        })),
    ))
}

/// Fallback register when auth-axum-login is not enabled
#[cfg(not(feature = "auth-axum-login"))]
async fn register(
    State(_state): State<AppState>,
    Json(_payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), ApiError> {
    Err(ApiError::Internal(
        "Session-based registration not available. Use JWT token endpoint.".to_string(),
    ))
}

/// Logout endpoint
#[cfg(feature = "auth-axum-login")]
async fn logout(mut auth_session: crate::auth::AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Fallback logout when auth-axum-login is not enabled
#[cfg(not(feature = "auth-axum-login"))]
async fn logout() -> impl IntoResponse {
    // JWT tokens can't be "logged out" server-side without a blocklist
    // Just return OK
    StatusCode::OK
}

/// Get current user info
async fn me(CurrentUser(user): CurrentUser) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(UserResponse {
        id: user.id,
        email: user.email.into_inner(),
        created_at: user.created_at,
    }))
}

/// Get a JWT token for the current session user
///
/// This allows session-authenticated users to obtain a JWT for API access.
#[cfg(feature = "auth-jwt")]
async fn get_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, ApiError> {
    let token = create_jwt_for_user(&user, &state)?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expiry_hours * 3600,
    }))
}

/// Helper to create JWT for a user
#[cfg(feature = "auth-jwt")]
fn create_jwt_for_user(user: &domain::User, state: &AppState) -> Result<String, ApiError> {
    let validator = state
        .jwt_validator
        .as_ref()
        .ok_or_else(|| ApiError::Internal("JWT not configured".to_string()))?;

    validator
        .create_token(user)
        .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))
}

// ============================================================================
// OIDC Routes
// ============================================================================

#[cfg(feature = "auth-oidc")]
async fn oidc_login(State(state): State<AppState>, session: Session) -> Result<Response, ApiError> {
    use axum::http::header;

    let service = state
        .oidc_service
        .as_ref()
        .ok_or(ApiError::Internal("OIDC not configured".into()))?;

    let auth_data = service.get_authorization_url();

    session
        .insert("oidc_csrf", &auth_data.csrf_token)
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    session
        .insert("oidc_nonce", &auth_data.nonce)
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    session
        .insert("oidc_pkce", &auth_data.pkce_verifier)
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;

    let response = axum::response::Redirect::to(auth_data.url.as_str()).into_response();
    let (mut parts, body) = response.into_parts();

    parts.headers.insert(
        header::CACHE_CONTROL,
        "no-cache, no-store, must-revalidate".parse().unwrap(),
    );
    parts
        .headers
        .insert(header::PRAGMA, "no-cache".parse().unwrap());
    parts.headers.insert(header::EXPIRES, "0".parse().unwrap());

    Ok(Response::from_parts(parts, body))
}

#[cfg(feature = "auth-oidc")]
#[derive(serde::Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

#[cfg(all(feature = "auth-oidc", feature = "auth-axum-login"))]
async fn oidc_callback(
    State(state): State<AppState>,
    session: Session,
    mut auth_session: crate::auth::AuthSession,
    axum::extract::Query(params): axum::extract::Query<CallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    let service = state
        .oidc_service
        .as_ref()
        .ok_or(ApiError::Internal("OIDC not configured".into()))?;

    let stored_csrf: domain::CsrfToken = session
        .get("oidc_csrf")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing CSRF token".into()))?;

    if params.state != stored_csrf.as_ref() {
        return Err(ApiError::Validation("Invalid CSRF token".into()));
    }

    let stored_pkce: domain::PkceVerifier = session
        .get("oidc_pkce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing PKCE".into()))?;
    let stored_nonce: domain::OidcNonce = session
        .get("oidc_nonce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing Nonce".into()))?;

    let oidc_user = service
        .resolve_callback(
            domain::AuthorizationCode::new(params.code),
            stored_nonce,
            stored_pkce,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let user = state
        .user_service
        .find_or_create(&oidc_user.subject, &oidc_user.email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let auth_mode = state.config.auth_mode;

    // In session or both mode, create session
    if matches!(auth_mode, AuthMode::Session | AuthMode::Both) {
        auth_session
            .login(&crate::auth::AuthUser(user.clone()))
            .await
            .map_err(|_| ApiError::Internal("Login failed".into()))?;
    }

    // Clean up OIDC state
    let _: Option<String> = session
        .remove("oidc_csrf")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    let _: Option<String> = session
        .remove("oidc_pkce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    let _: Option<String> = session
        .remove("oidc_nonce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;

    // In JWT mode, return token as JSON
    #[cfg(feature = "auth-jwt")]
    if matches!(auth_mode, AuthMode::Jwt | AuthMode::Both) {
        let token = create_jwt_for_user(&user, &state)?;
        return Ok(Json(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.config.jwt_expiry_hours * 3600,
        })
        .into_response());
    }

    // Session mode: return user info
    Ok(Json(UserResponse {
        id: user.id,
        email: user.email.into_inner(),
        created_at: user.created_at,
    })
    .into_response())
}

/// Fallback OIDC callback when auth-axum-login is not enabled
#[cfg(all(feature = "auth-oidc", not(feature = "auth-axum-login")))]
async fn oidc_callback(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Query(params): axum::extract::Query<CallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    let service = state
        .oidc_service
        .as_ref()
        .ok_or(ApiError::Internal("OIDC not configured".into()))?;

    let stored_csrf: domain::CsrfToken = session
        .get("oidc_csrf")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing CSRF token".into()))?;

    if params.state != stored_csrf.as_ref() {
        return Err(ApiError::Validation("Invalid CSRF token".into()));
    }

    let stored_pkce: domain::PkceVerifier = session
        .get("oidc_pkce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing PKCE".into()))?;
    let stored_nonce: domain::OidcNonce = session
        .get("oidc_nonce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing Nonce".into()))?;

    let oidc_user = service
        .resolve_callback(
            domain::AuthorizationCode::new(params.code),
            stored_nonce,
            stored_pkce,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let user = state
        .user_service
        .find_or_create(&oidc_user.subject, &oidc_user.email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Clean up OIDC state
    let _: Option<String> = session
        .remove("oidc_csrf")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    let _: Option<String> = session
        .remove("oidc_pkce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    let _: Option<String> = session
        .remove("oidc_nonce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;

    // Return token as JSON
    #[cfg(feature = "auth-jwt")]
    {
        let token = create_jwt_for_user(&user, &state)?;
        return Ok(Json(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.config.jwt_expiry_hours * 3600,
        }));
    }

    #[cfg(not(feature = "auth-jwt"))]
    {
        let _ = user; // Suppress unused warning
        Err(ApiError::Internal(
            "No auth backend available for OIDC callback".to_string(),
        ))
    }
}
