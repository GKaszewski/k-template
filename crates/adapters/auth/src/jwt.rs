use async_trait::async_trait;
use chrono::Utc;
use domain::{errors::DomainError, ports::TokenIssuer, value_objects::{Role, UserId}};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: i64,
}

pub struct JwtTokenIssuer {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiry_hours: i64,
}

impl JwtTokenIssuer {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiry_hours: 24,
        }
    }
}

#[async_trait]
impl TokenIssuer for JwtTokenIssuer {
    async fn issue(&self, user_id: &UserId, role: &Role) -> Result<String, DomainError> {
        let claims = Claims {
            sub: user_id.to_string(),
            role: role.to_string(),
            exp: (Utc::now() + chrono::Duration::hours(self.expiry_hours)).timestamp(),
        };
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DomainError::Internal(e.to_string()))
    }

    async fn verify(&self, token: &str) -> Result<(UserId, Role), DomainError> {
        let data = decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map_err(|_| DomainError::Unauthorized("Invalid or expired token".to_string()))?;
        let uuid = uuid::Uuid::parse_str(&data.claims.sub)
            .map_err(|_| DomainError::Unauthorized("Invalid token subject".to_string()))?;
        let role = Role::from_str(&data.claims.role)
            .map_err(|_| DomainError::Unauthorized("Invalid role in token".to_string()))?;
        Ok((UserId::from_uuid(uuid), role))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn issue_and_verify_roundtrip() {
        let issuer = JwtTokenIssuer::new("test-secret-key-long-enough-32chars!!");
        let user_id = UserId::new();
        let token = issuer.issue(&user_id, &Role::User).await.unwrap();
        let (verified_id, verified_role) = issuer.verify(&token).await.unwrap();
        assert_eq!(verified_id, user_id);
        assert_eq!(verified_role, Role::User);
    }

    #[tokio::test]
    async fn rejects_invalid_token() {
        let issuer = JwtTokenIssuer::new("test-secret-key-long-enough-32chars!!");
        let result = issuer.verify("not.a.valid.jwt").await;
        assert!(matches!(result, Err(DomainError::Unauthorized(_))));
    }
}
