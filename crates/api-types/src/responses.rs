use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

impl UserResponse {
    pub fn from_domain(user: &domain::entities::User) -> Self {
        Self {
            id: *user.id.as_uuid(),
            email: user.email.to_string(),
            role: user.role.to_string(),
            created_at: user.created_at,
        }
    }
}
