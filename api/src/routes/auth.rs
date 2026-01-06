use axum::http::StatusCode;
use axum::{
    Router,
    extract::{Json, State},
    response::IntoResponse,
    routing::post,
};

use crate::{
    dto::{LoginRequest, RegisterRequest, UserResponse},
    error::ApiError,
    state::AppState,
};
use domain::{DomainError, Email};
use tower_sessions::Session;

pub fn router() -> Router<AppState> {
    let r = Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/logout", post(logout))
        .route("/me", post(me));

    #[cfg(feature = "auth-oidc")]
    let r = r
        .route("/login/oidc", axum::routing::get(oidc_login))
        .route("/auth/callback", axum::routing::get(oidc_callback));

    r
}

async fn login(
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

    auth_session
        .login(&user)
        .await
        .map_err(|_| ApiError::Internal("Login failed".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(UserResponse {
            id: user.0.id,
            email: user.0.email.into_inner(),
            created_at: user.0.created_at,
        }),
    ))
}

async fn register(
    State(state): State<AppState>,
    mut auth_session: crate::auth::AuthSession,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if state
        .user_service
        .find_by_email(&payload.email)
        .await?
        .is_some()
    {
        return Err(ApiError::Domain(DomainError::UserAlreadyExists(
            payload.email,
        )));
    }

    // Note: In a real app, you would hash the password here.
    // This template uses a simplified User::new which doesn't take password.
    // You should extend User to handle passwords or use an OIDC flow.
    let email = Email::try_from(payload.email).map_err(|e| ApiError::Validation(e.to_string()))?;

    // Using email as subject for local auth for now
    let user = state
        .user_service
        .find_or_create(&email.as_ref().to_string(), email.as_ref())
        .await?;

    // Log the user in
    let auth_user = crate::auth::AuthUser(user.clone());

    auth_session
        .login(&auth_user)
        .await
        .map_err(|_| ApiError::Internal("Login failed".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(UserResponse {
            id: user.id,
            email: user.email.into_inner(),
            created_at: user.created_at,
        }),
    ))
}

async fn logout(mut auth_session: crate::auth::AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn me(auth_session: crate::auth::AuthSession) -> Result<impl IntoResponse, ApiError> {
    let user = auth_session
        .user
        .ok_or(ApiError::Unauthorized("Not logged in".to_string()))?;

    Ok(Json(UserResponse {
        id: user.0.id,
        email: user.0.email.into_inner(),
        created_at: user.0.created_at,
    }))
}

#[cfg(feature = "auth-oidc")]
async fn oidc_login(
    State(state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse, ApiError> {
    let service = state
        .oidc_service
        .as_ref()
        .ok_or(ApiError::Internal("OIDC not configured".into()))?;

    let (url, csrf, nonce, pkce) = service.get_authorization_url();

    session
        .insert("oidc_csrf", csrf)
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    session
        .insert("oidc_nonce", nonce)
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;
    session
        .insert("oidc_pkce", pkce)
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?;

    Ok(axum::response::Redirect::to(&url))
}

#[cfg(feature = "auth-oidc")]
#[derive(serde::Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

#[cfg(feature = "auth-oidc")]
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

    let stored_csrf: String = session
        .get("oidc_csrf")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing CSRF token".into()))?;

    if params.state != stored_csrf {
        return Err(ApiError::Validation("Invalid CSRF token".into()));
    }

    // 2. Retrieve secrets
    let stored_pkce: String = session
        .get("oidc_pkce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing PKCE".into()))?;
    let stored_nonce: String = session
        .get("oidc_nonce")
        .await
        .map_err(|_| ApiError::Internal("Session error".into()))?
        .ok_or(ApiError::Validation("Missing Nonce".into()))?;

    let oidc_user = service
        .resolve_callback(params.code, stored_nonce, stored_pkce)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let user = state
        .user_service
        .find_or_create(&oidc_user.subject, &oidc_user.email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    auth_session
        .login(&crate::auth::AuthUser(user))
        .await
        .map_err(|_| ApiError::Internal("Login failed".into()))?;

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

    Ok(axum::response::Redirect::to("/"))
}
