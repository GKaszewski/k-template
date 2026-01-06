//! Request and Response DTOs
//!
//! Data Transfer Objects for the API.
//! Uses domain newtypes for validation instead of the validator crate.

use chrono::{DateTime, Utc};
use domain::{Email, Password};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Login request with validated email and password newtypes
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Email is validated on deserialization
    pub email: Email,
    /// Password is validated on deserialization (min 6 chars)
    pub password: Password,
}

/// Register request with validated email and password newtypes
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Email is validated on deserialization
    pub email: Email,
    /// Password is validated on deserialization (min 6 chars)
    pub password: Password,
}

/// User response DTO
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// System configuration response
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub allow_registration: bool,
}
