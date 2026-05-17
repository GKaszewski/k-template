#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new() -> Self { Self(uuid::Uuid::new_v4()) }
    pub fn from_uuid(id: uuid::Uuid) -> Self { Self(id) }
    pub fn as_uuid(&self) -> &uuid::Uuid { &self.0 }
}

impl Default for UserId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<uuid::Uuid> for UserId {
    fn from(id: uuid::Uuid) -> Self { Self(id) }
}
