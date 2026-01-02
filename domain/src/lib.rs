//! Domain Logic
//!
//! This crate contains the core business logic, entities, and repository interfaces.
//! It is completely independent of the infrastructure layer (databases, HTTP, etc.).

pub mod entities;
pub mod errors;
pub mod repositories;
pub mod services;
pub mod value_objects;

// Re-export commonly used types
pub use entities::*;
pub use errors::{DomainError, DomainResult};
pub use repositories::*;
pub use services::UserService;
pub use value_objects::*;
