# k-template

A production-ready, modular Rust template for K-Suite applications, following Hexagonal Architecture principles.

## 🌟 Features

- **Hexagonal Architecture**: Clear separation of concerns between Domain, Infrastructure, and API layers.
- **Modular & Swappable**: Vendor implementations (databases, message brokers) are behind feature flags and trait objects.
- **Feature-Gated Dependencies**: Compile only what you need. Unused dependencies are not included in the build.
- **Cargo Generate Ready**: Pre-configured for `cargo-generate` to easily scaffold new services.
- **Testable**: Domain logic is pure and easily testable; Infrastructure is tested with integration tests.

## 🏗️ Project Structure

The workspace consists of three main crates:

- **`template-domain`**: The core business logic.
  - Contains Entities, Value Objects, Repository Interfaces (Ports), and Services.
  - **Dependencies**: Pure Rust only (no I/O, no heavy frameworks).
  
- **`template-infra`**: The adapters layer.
  - Implements the Repository interfaces defined in `template-domain`.
  - Content is heavily feature-gated (e.g., `sqlite`, `postgres`, `broker-nats`).
  
- **`template-api`**: The application entry point (Driving Adapter).
  - Wires everything together using dependency injection.
  - Handles HTTP/REST/gRPC interfaces.

## 🚀 Getting Started

### Prerequisites

- Rust (latest stable)
- `cargo-generate` (`cargo install cargo-generate`)

### Creating a New Project

Use `cargo-generate` to scaffold a new project from this template:

```bash
cargo generate --git https://github.com/your-org/k-template.git
```

You will be prompted for:
1. **Project Name**: The name of your new service.
2. **Database**: Choose between `sqlite` (default) or `postgres`.

The template will automatically clean up unused repository implementations based on your choice.

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific feature (e.g., postgres)
cargo test -p template-infra --no-default-features --features postgres
```

## ⚙️ Configuration & Feature Flags

This template uses Cargo features to control compilation of infrastructure adapters.

| Feature | Description | Crate |
|---------|-------------|-------|
| `sqlite` | Enables SQLite repository implementations and dependencies | `template-infra`, `template-api` |
| `postgres` | Enables PostgreSQL repository implementations and dependencies | `template-infra`, `template-api` |
| `broker-nats`| Enables NATS messaging support | `template-infra` |
| `smart-features` | Enables AI/Vector DB capabilities (Qdrant, FastEmbed) | `template-infra` |

### Switching Databases

To switch from the default SQLite to PostgreSQL in an existing project, update `Cargo.toml`:

**`template-api/Cargo.toml`**:
```toml
[features]
default = ["postgres"] 
# ...
```

**`template-infra/Cargo.toml`**:
```toml
[features]
default = ["postgres"]
# ...
```

## 📐 Architecture Guide

### Adding a New Feature

1. **Domain**: Define the Entity, Value Objects, and Repository Interface in `template-domain`.
2. **Infra**: Implement the Repository Interface in `template-infra`.
   - **Important**: Wrap your implementation in a feature flag (e.g., `#[cfg(feature = "my-feature")]`).
3. **API**: Wire the new service in `template-api/src/main.rs` or a dedicated module.

### Vendor Isolation

All external dependencies (SQLx, NATS, etc.) should stay within `template-infra` or `template-api`. The `template-domain` crate should remain agnostic to specific technologies.
