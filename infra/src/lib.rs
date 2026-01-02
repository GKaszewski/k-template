//! K-Notes Infrastructure Layer
//!
//! This crate provides concrete implementations (adapters) for the
//! repository ports defined in the domain layer.
//!
//! ## Adapters
//!
//! - [`SqliteNoteRepository`] - SQLite adapter for notes with FTS5 search
//! - [`SqliteUserRepository`] - SQLite adapter for users (OIDC-ready)
//! - [`SqliteTagRepository`] - SQLite adapter for tags
//!
//! ## Database
//!
//! - [`db::create_pool`] - Create a database connection pool
//! - [`db::run_migrations`] - Run database migrations

pub mod db;
pub mod factory;
pub mod session_store;
mod user_repository;

// Re-export for convenience
pub use db::run_migrations;
#[cfg(feature = "sqlite")]
pub use user_repository::SqliteUserRepository;
