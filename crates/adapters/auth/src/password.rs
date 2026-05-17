use async_trait::async_trait;
use domain::{errors::DomainError, ports::PasswordHasher, value_objects::PasswordHash};

pub struct BcryptPasswordHasher;

#[async_trait]
impl PasswordHasher for BcryptPasswordHasher {
    async fn hash(&self, password: &str) -> Result<PasswordHash, DomainError> {
        let password = password.to_owned();
        let hash = tokio::task::spawn_blocking(move || bcrypt::hash(&password, 12))
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(PasswordHash::from_hash(hash))
    }

    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, DomainError> {
        let password = password.to_owned();
        let hash = hash.as_str().to_owned();
        tokio::task::spawn_blocking(move || bcrypt::verify(&password, &hash))
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?
            .map_err(|e| DomainError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_and_verify_roundtrip() {
        let h = BcryptPasswordHasher;
        let hash = h.hash("mysecretpassword").await.unwrap();
        assert!(h.verify("mysecretpassword", &hash).await.unwrap());
        assert!(!h.verify("wrongpassword", &hash).await.unwrap());
    }
}
