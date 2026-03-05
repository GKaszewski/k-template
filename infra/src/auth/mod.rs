//! Authentication infrastructure
//!
//! This module contains the concrete implementation of authentication mechanisms.

/// Hash a password using the password-auth crate
pub fn hash_password(password: &str) -> String {
    password_auth::generate_hash(password)
}

/// Verify a password against a stored hash
pub fn verify_password(password: &str, hash: &str) -> bool {
    password_auth::verify_password(password, hash).is_ok()
}

#[cfg(feature = "auth-oidc")]
pub mod oidc;

#[cfg(feature = "auth-jwt")]
pub mod jwt;
