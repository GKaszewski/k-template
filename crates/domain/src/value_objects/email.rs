use crate::errors::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into().trim().to_lowercase();
        if value.is_empty() || !value.contains('@') {
            return Err(DomainError::Validation("Invalid email address".to_string()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() { assert!(Email::new("").is_err()); }

    #[test]
    fn rejects_no_at() { assert!(Email::new("notanemail").is_err()); }

    #[test]
    fn accepts_valid() { assert!(Email::new("user@example.com").is_ok()); }

    #[test]
    fn lowercases_and_trims() {
        let email = Email::new("  User@Example.Com  ").unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }
}
