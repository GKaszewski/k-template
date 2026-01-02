use axum::{
    extract::{State, Json},
    response::IntoResponse,
    Router, routing::post,
};
use axum::http::StatusCode;

use crate::{
    dto::{LoginRequest, RegisterRequest, UserResponse},
    error::ApiError,
    state::AppState,
};
use template_domain::{DomainError, Email};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/logout", post(logout))
        .route("/me", post(me))
}

async fn login(
    mut auth_session: crate::auth::AuthSession,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = match auth_session.authenticate(crate::auth::Credentials {
        email: payload.email,
        password: payload.password,
    }).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(ApiError::Validation("Invalid credentials".to_string())),
        Err(_) => return Err(ApiError::Internal("Authentication failed".to_string())),
    };

    auth_session.login(&user).await.map_err(|_| ApiError::Internal("Login failed".to_string()))?;

    Ok((StatusCode::OK, Json(UserResponse {
        id: user.0.id,
        email: user.0.email.into_inner(),
        created_at: user.0.created_at,
    })))
}

async fn register(
    State(state): State<AppState>,
    mut auth_session: crate::auth::AuthSession,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if state.user_service.find_by_email(&payload.email).await?.is_some() {
        return Err(ApiError::Domain(DomainError::UserAlreadyExists(payload.email)));
    }

    // Note: In a real app, you would hash the password here. 
    // This template uses a simplified User::new which doesn't take password.
    // You should extend User to handle passwords or use an OIDC flow.
    let email = Email::try_from(payload.email).map_err(|e| ApiError::Validation(e.to_string()))?;
    
    // Using email as subject for local auth for now
    let user = state.user_service.find_or_create(&email.as_ref().to_string(), email.as_ref()).await?;
    
    // Log the user in
    let auth_user = crate::auth::AuthUser(user.clone());
    
    auth_session.login(&auth_user).await.map_err(|_| ApiError::Internal("Login failed".to_string()))?;

    Ok((StatusCode::CREATED, Json(UserResponse {
        id: user.id,
        email: user.email.into_inner(),
        created_at: user.created_at,
    })))
}

async fn logout(mut auth_session: crate::auth::AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn me(auth_session: crate::auth::AuthSession) -> Result<impl IntoResponse, ApiError> {
    let user = auth_session.user.ok_or(ApiError::Unauthorized("Not logged in".to_string()))?;
    
    Ok(Json(UserResponse {
        id: user.0.id,
        email: user.0.email.into_inner(),
        created_at: user.0.created_at, 
    }))
}
