//! JWT Authentication Infrastructure
//!
//! Provides JWT token creation and validation using HS256 (secret-based).
//! For OIDC/JWKS validation, see the `oidc` module.

use domain::User;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimum secret length for production (256 bits = 32 bytes)
const MIN_SECRET_LENGTH: usize = 32;

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for HS256 signing/verification
    pub secret: String,
    /// Expected issuer (for validation)
    pub issuer: Option<String>,
    /// Expected audience (for validation)
    pub audience: Option<String>,
    /// Token expiry in hours (default: 24)
    pub expiry_hours: u64,
}

impl JwtConfig {
    /// Create a new JWT config with validation
    ///
    /// In production mode, this will reject weak secrets.
    pub fn new(
        secret: String,
        issuer: Option<String>,
        audience: Option<String>,
        expiry_hours: Option<u64>,
        is_production: bool,
    ) -> Result<Self, JwtError> {
        // Validate secret strength in production
        if is_production && secret.len() < MIN_SECRET_LENGTH {
            return Err(JwtError::WeakSecret {
                min_length: MIN_SECRET_LENGTH,
                actual_length: secret.len(),
            });
        }

        Ok(Self {
            secret,
            issuer,
            audience,
            expiry_hours: expiry_hours.unwrap_or(24),
        })
    }

    /// Create config without validation (for testing)
    pub fn new_unchecked(secret: String) -> Self {
        Self {
            secret,
            issuer: None,
            audience: None,
            expiry_hours: 24,
        }
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    /// Subject - the user's unique identifier (user ID as string)
    pub sub: String,
    /// User's email address
    pub email: String,
    /// Expiry timestamp (seconds since UNIX epoch)
    pub exp: usize,
    /// Issued at timestamp (seconds since UNIX epoch)
    pub iat: usize,
    /// Issuer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
}

/// JWT-related errors
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("JWT secret is too weak: minimum {min_length} bytes required, got {actual_length}")]
    WeakSecret {
        min_length: usize,
        actual_length: usize,
    },

    #[error("Token creation failed: {0}")]
    CreationFailed(#[from] jsonwebtoken::errors::Error),

    #[error("Token validation failed: {0}")]
    ValidationFailed(String),

    #[error("Token expired")]
    Expired,

    #[error("Invalid token format")]
    InvalidFormat,

    #[error("Missing configuration")]
    MissingConfig,
}

/// JWT token validator and generator
#[derive(Clone)]
pub struct JwtValidator {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtValidator {
    /// Create a new JWT validator with the given configuration
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        let mut validation = Validation::new(Algorithm::HS256);

        // Configure issuer validation if set
        if let Some(ref issuer) = config.issuer {
            validation.set_issuer(&[issuer]);
        }

        // Configure audience validation if set
        if let Some(ref audience) = config.audience {
            validation.set_audience(&[audience]);
        }

        Self {
            config,
            encoding_key,
            decoding_key,
            validation,
        }
    }

    /// Create a JWT token for the given user
    pub fn create_token(&self, user: &User) -> Result<String, JwtError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as usize;

        let expiry = now + (self.config.expiry_hours as usize * 3600);

        let claims = JwtClaims {
            sub: user.id.to_string(),
            email: user.email.as_ref().to_string(),
            exp: expiry,
            iat: now,
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
        };

        let header = Header::new(Algorithm::HS256);
        encode(&header, &claims, &self.encoding_key).map_err(JwtError::CreationFailed)
    }

    /// Validate a JWT token and return the claims
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation).map_err(
            |e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                jsonwebtoken::errors::ErrorKind::InvalidToken => JwtError::InvalidFormat,
                _ => JwtError::ValidationFailed(e.to_string()),
            },
        )?;

        Ok(token_data.claims)
    }

    /// Get the user ID (subject) from a token without full validation
    /// Useful for logging/debugging, but should not be trusted for auth
    pub fn decode_unverified(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| JwtError::InvalidFormat)?;

        Ok(token_data.claims)
    }
}

impl std::fmt::Debug for JwtValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtValidator")
            .field("issuer", &self.config.issuer)
            .field("audience", &self.config.audience)
            .field("expiry_hours", &self.config.expiry_hours)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Email;

    fn create_test_user() -> User {
        let email = Email::try_from("test@example.com").unwrap();
        User::new("test-subject", email)
    }

    #[test]
    fn test_create_and_validate_token() {
        let config = JwtConfig::new_unchecked("test-secret-key-that-is-long-enough".to_string());
        let validator = JwtValidator::new(config);
        let user = create_test_user();

        let token = validator.create_token(&user).expect("Should create token");
        let claims = validator
            .validate_token(&token)
            .expect("Should validate token");

        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_weak_secret_rejected_in_production() {
        let result = JwtConfig::new(
            "short".to_string(), // Too short
            None,
            None,
            None,
            true, // Production mode
        );

        assert!(matches!(result, Err(JwtError::WeakSecret { .. })));
    }

    #[test]
    fn test_weak_secret_allowed_in_development() {
        let result = JwtConfig::new(
            "short".to_string(), // Too short but OK in dev
            None,
            None,
            None,
            false, // Development mode
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_token_rejected() {
        let config = JwtConfig::new_unchecked("test-secret-key-that-is-long-enough".to_string());
        let validator = JwtValidator::new(config);

        let result = validator.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let config1 = JwtConfig::new_unchecked("secret-one-that-is-long-enough".to_string());
        let config2 = JwtConfig::new_unchecked("secret-two-that-is-long-enough".to_string());

        let validator1 = JwtValidator::new(config1);
        let validator2 = JwtValidator::new(config2);

        let user = create_test_user();
        let token = validator1.create_token(&user).unwrap();

        // Token from validator1 should fail on validator2
        let result = validator2.validate_token(&token);
        assert!(result.is_err());
    }
}
