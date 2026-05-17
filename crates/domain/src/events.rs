use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DomainEvent {
    UserRegistered { user_id: Uuid },
    UserLoggedIn { user_id: Uuid },
}
