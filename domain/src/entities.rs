//! Domain entities
//!
//! This module contains pure domain types with no I/O dependencies.
//! These represent the core business concepts of the application.

pub use crate::value_objects::{Email, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user in the system.
///
/// Designed to be OIDC-ready: the `subject` field stores the OIDC subject claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub subject: String,
    pub email: Email,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(subject: impl Into<String>, email: Email) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject: subject.into(),
            email,
            password_hash: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_id(
        id: Uuid,
        subject: impl Into<String>,
        email: Email,
        password_hash: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            subject: subject.into(),
            email,
            password_hash,
            created_at,
        }
    }

    pub fn new_local(email: Email, password_hash: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject: format!("local|{}", Uuid::new_v4()),
            email,
            password_hash: Some(password_hash.into()),
            created_at: Utc::now(),
        }
    }

    /// Helper to get email as string
    pub fn email_str(&self) -> &str {
        self.email.as_ref()
    }
}
