//! Value Objects for K-Notes Domain
//!
//! Newtypes that encapsulate validation logic, following the "parse, don't validate" pattern.
//! These types can only be constructed if the input is valid, providing compile-time guarantees.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use thiserror::Error;
use url::Url;
use uuid::Uuid;

pub type UserId = Uuid;

// ============================================================================
// Validation Error
// ============================================================================

/// Errors that occur when parsing/validating value objects
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Invalid email format: {0}")]
    InvalidEmail(String),

    #[error("Password must be at least {min} characters, got {actual}")]
    PasswordTooShort { min: usize, actual: usize },

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Value cannot be empty: {0}")]
    Empty(String),

    #[error("Secret too short: minimum {min} bytes required, got {actual}")]
    SecretTooShort { min: usize, actual: usize },
}

// ============================================================================
// Email (using email_address crate for RFC-compliant validation)
// ============================================================================

/// A validated email address using RFC-compliant validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(email_address::EmailAddress);

impl Email {
    /// Create a new validated email address
    pub fn new(value: impl AsRef<str>) -> Result<Self, ValidationError> {
        let value = value.as_ref().trim().to_lowercase();
        let addr: email_address::EmailAddress = value
            .parse()
            .map_err(|_| ValidationError::InvalidEmail(value.clone()))?;
        Ok(Self(addr))
    }

    /// Get the inner value
    pub fn into_inner(self) -> String {
        self.0.to_string()
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Email {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Email {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for Email {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.as_ref())
    }
}

impl<'de> Deserialize<'de> for Email {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// Password
// ============================================================================

/// A validated password input (NOT the hash).
///
/// Enforces minimum length of 6 characters.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

/// Minimum password length
pub const MIN_PASSWORD_LENGTH: usize = 6;

impl Password {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();

        if value.len() < MIN_PASSWORD_LENGTH {
            return Err(ValidationError::PasswordTooShort {
                min: MIN_PASSWORD_LENGTH,
                actual: value.len(),
            });
        }

        Ok(Self(value))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Intentionally hide password content in Debug
impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Password(***)")
    }
}

impl TryFrom<String> for Password {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Password {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

// Note: Password should NOT implement Serialize to prevent accidental exposure

// ============================================================================
// OIDC Configuration Newtypes
// ============================================================================

/// OIDC Issuer URL - validated URL for the identity provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IssuerUrl(Url);

impl IssuerUrl {
    pub fn new(value: impl AsRef<str>) -> Result<Self, ValidationError> {
        let value = value.as_ref().trim();
        let url = Url::parse(value).map_err(|e| ValidationError::InvalidUrl(e.to_string()))?;
        Ok(Self(url))
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

impl AsRef<str> for IssuerUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for IssuerUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for IssuerUrl {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<IssuerUrl> for String {
    fn from(val: IssuerUrl) -> Self {
        val.0.to_string()
    }
}

/// OIDC Client Identifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ClientId(String);

impl ClientId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(ValidationError::Empty("client_id".to_string()));
        }
        Ok(Self(value))
    }
}

impl AsRef<str> for ClientId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ClientId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ClientId> for String {
    fn from(val: ClientId) -> Self {
        val.0
    }
}

/// OIDC Client Secret - hidden in Debug output
#[derive(Clone, PartialEq, Eq)]
pub struct ClientSecret(String);

impl ClientSecret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Check if the secret is empty (for public clients)
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl AsRef<str> for ClientSecret {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ClientSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClientSecret(***)")
    }
}

impl fmt::Display for ClientSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

impl<'de> Deserialize<'de> for ClientSecret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

// Note: ClientSecret should NOT implement Serialize

/// OAuth Redirect URL - validated URL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RedirectUrl(Url);

impl RedirectUrl {
    pub fn new(value: impl AsRef<str>) -> Result<Self, ValidationError> {
        let value = value.as_ref().trim();
        let url = Url::parse(value).map_err(|e| ValidationError::InvalidUrl(e.to_string()))?;
        Ok(Self(url))
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

impl AsRef<str> for RedirectUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RedirectUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for RedirectUrl {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RedirectUrl> for String {
    fn from(val: RedirectUrl) -> Self {
        val.0.to_string()
    }
}

/// OIDC Resource Identifier (optional audience)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(ValidationError::Empty("resource_id".to_string()));
        }
        Ok(Self(value))
    }
}

impl AsRef<str> for ResourceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ResourceId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ResourceId> for String {
    fn from(val: ResourceId) -> Self {
        val.0
    }
}

// ============================================================================
// OIDC Flow Newtypes (for type-safe session storage)
// ============================================================================

/// CSRF Token for OIDC state parameter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsrfToken(String);

impl CsrfToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for CsrfToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CsrfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Nonce for OIDC ID token verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcNonce(String);

impl OidcNonce {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for OidcNonce {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OidcNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// PKCE Code Verifier
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PkceVerifier(String);

impl PkceVerifier {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for PkceVerifier {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Hide PKCE verifier in Debug (security)
impl fmt::Debug for PkceVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PkceVerifier(***)")
    }
}

/// OAuth2 Authorization Code
#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationCode(String);

impl AuthorizationCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for AuthorizationCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Hide authorization code in Debug (security)
impl fmt::Debug for AuthorizationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AuthorizationCode(***)")
    }
}

impl<'de> Deserialize<'de> for AuthorizationCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

/// Complete authorization URL data returned when starting OIDC flow
#[derive(Debug, Clone)]
pub struct AuthorizationUrlData {
    /// The URL to redirect the user to
    pub url: Url,
    /// CSRF token to store in session
    pub csrf_token: CsrfToken,
    /// Nonce to store in session
    pub nonce: OidcNonce,
    /// PKCE verifier to store in session
    pub pkce_verifier: PkceVerifier,
}

// ============================================================================
// Configuration Newtypes
// ============================================================================

/// Database connection URL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DatabaseUrl(String);

impl DatabaseUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::Empty("database_url".to_string()));
        }
        Ok(Self(value))
    }
}

impl AsRef<str> for DatabaseUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DatabaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for DatabaseUrl {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DatabaseUrl> for String {
    fn from(val: DatabaseUrl) -> Self {
        val.0
    }
}

/// Session secret with minimum length requirement
pub const MIN_SESSION_SECRET_LENGTH: usize = 64;

#[derive(Clone, PartialEq, Eq)]
pub struct SessionSecret(String);

impl SessionSecret {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.len() < MIN_SESSION_SECRET_LENGTH {
            return Err(ValidationError::SecretTooShort {
                min: MIN_SESSION_SECRET_LENGTH,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Create without validation (for development/testing)
    pub fn new_unchecked(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for SessionSecret {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionSecret(***)")
    }
}

/// JWT signing secret with minimum length requirement
pub const MIN_JWT_SECRET_LENGTH: usize = 32;

#[derive(Clone, PartialEq, Eq)]
pub struct JwtSecret(String);

impl JwtSecret {
    pub fn new(value: impl Into<String>, is_production: bool) -> Result<Self, ValidationError> {
        let value = value.into();
        if is_production && value.len() < MIN_JWT_SECRET_LENGTH {
            return Err(ValidationError::SecretTooShort {
                min: MIN_JWT_SECRET_LENGTH,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Create without validation (for development/testing)
    pub fn new_unchecked(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for JwtSecret {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for JwtSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JwtSecret(***)")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod email_tests {
        use super::*;

        #[test]
        fn test_valid_email() {
            assert!(Email::new("user@example.com").is_ok());
            assert!(Email::new("USER@EXAMPLE.COM").is_ok()); // Should lowercase
            assert!(Email::new("  user@example.com  ").is_ok()); // Should trim
        }

        #[test]
        fn test_email_normalizes() {
            let email = Email::new("  USER@EXAMPLE.COM  ").unwrap();
            assert_eq!(email.as_ref(), "user@example.com");
        }

        #[test]
        fn test_invalid_email_no_at() {
            assert!(Email::new("userexample.com").is_err());
        }

        #[test]
        fn test_invalid_email_no_domain() {
            assert!(Email::new("user@").is_err());
        }

        #[test]
        fn test_invalid_email_no_local() {
            assert!(Email::new("@example.com").is_err());
        }
    }

    mod password_tests {
        use super::*;

        #[test]
        fn test_valid_password() {
            assert!(Password::new("secret123").is_ok());
            assert!(Password::new("123456").is_ok()); // Exactly 6 chars
        }

        #[test]
        fn test_password_too_short() {
            assert!(Password::new("12345").is_err()); // 5 chars
            assert!(Password::new("").is_err());
        }

        #[test]
        fn test_password_debug_hides_content() {
            let password = Password::new("supersecret").unwrap();
            let debug = format!("{:?}", password);
            assert!(!debug.contains("supersecret"));
            assert!(debug.contains("***"));
        }
    }

    mod oidc_tests {
        use super::*;

        #[test]
        fn test_issuer_url_valid() {
            assert!(IssuerUrl::new("https://auth.example.com").is_ok());
        }

        #[test]
        fn test_issuer_url_invalid() {
            assert!(IssuerUrl::new("not-a-url").is_err());
        }

        #[test]
        fn test_client_id_non_empty() {
            assert!(ClientId::new("my-client").is_ok());
            assert!(ClientId::new("").is_err());
            assert!(ClientId::new("   ").is_err());
        }

        #[test]
        fn test_client_secret_hides_in_debug() {
            let secret = ClientSecret::new("super-secret");
            let debug = format!("{:?}", secret);
            assert!(!debug.contains("super-secret"));
            assert!(debug.contains("***"));
        }
    }

    mod secret_tests {
        use super::*;

        #[test]
        fn test_session_secret_min_length() {
            let short = "short";
            let long = "a".repeat(64);

            assert!(SessionSecret::new(short).is_err());
            assert!(SessionSecret::new(long).is_ok());
        }

        #[test]
        fn test_jwt_secret_production_check() {
            let short = "short";
            let long = "a".repeat(32);

            // Production mode enforces length
            assert!(JwtSecret::new(short, true).is_err());
            assert!(JwtSecret::new(&long, true).is_ok());

            // Development mode allows short secrets
            assert!(JwtSecret::new(short, false).is_ok());
        }

        #[test]
        fn test_secrets_hide_in_debug() {
            let session = SessionSecret::new_unchecked("secret");
            let jwt = JwtSecret::new_unchecked("secret");

            assert!(!format!("{:?}", session).contains("secret"));
            assert!(!format!("{:?}", jwt).contains("secret"));
        }
    }
}
