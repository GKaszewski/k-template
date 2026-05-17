// Manual Debug — redacts hash to prevent it appearing in logs
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordHash(String);

impl std::fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PasswordHash").field(&"[redacted]").finish()
    }
}

impl PasswordHash {
    pub fn from_hash(hash: String) -> Self { Self(hash) }
    pub fn as_str(&self) -> &str { &self.0 }
}
