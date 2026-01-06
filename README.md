# k-template

A production-ready, modular Rust API template for K-Suite applications, following Hexagonal Architecture principles.

## Features

- **Hexagonal Architecture**: Clear separation between Domain, Infrastructure, and API layers
- **Multiple Auth Modes**: Session-based, JWT, or both - fully configurable
- **OIDC Integration**: Connect to any OpenID Connect provider (Keycloak, Auth0, Zitadel, etc.)
- **Database Flexibility**: SQLite (default) or PostgreSQL via feature flags
- **Type-Safe Configuration**: Newtypes with built-in validation for all security-sensitive values
- **Cargo Generate Ready**: Pre-configured for scaffolding new services

## Quick Start

### 1. Clone and Configure

```bash
git clone https://github.com/GKaszewski/k-template.git my-api
cd my-api
cp .env.example .env
# Edit .env with your configuration
```

### 2. Run

```bash
# Development (with hot reload via cargo-watch)
cargo watch -x run

# Or simply
cargo run
```

The API will be available at `http://localhost:3000/api/v1/`.

## Configuration

All configuration is done via environment variables. See [.env.example](.env.example) for all options.

### Authentication Modes

Set `AUTH_MODE` to one of:

| Mode | Description | Required Features |
|------|-------------|-------------------|
| `session` | Cookie-based sessions | `auth-axum-login` |
| `jwt` | Bearer token authentication | `auth-jwt` |
| `both` | Try JWT first, fall back to session | `auth-axum-login`, `auth-jwt` |

### OIDC Integration

To enable OIDC login (e.g., "Login with Google"):

1. Enable the `auth-oidc` feature (enabled by default)
2. Configure your OIDC provider in `.env`:
   ```env
   OIDC_ISSUER=https://your-provider.com
   OIDC_CLIENT_ID=your-client-id
   OIDC_CLIENT_SECRET=your-secret
   OIDC_REDIRECT_URL=http://localhost:3000/api/v1/auth/callback
   ```
3. Users can login via `GET /api/v1/auth/login/oidc`

## Feature Flags

Features are configured in `api/Cargo.toml`:

```toml
[features]
default = ["sqlite", "auth-axum-login", "auth-oidc", "auth-jwt"]
```

| Feature | Description |
|---------|-------------|
| `sqlite` | SQLite database support (default) |
| `postgres` | PostgreSQL database support |
| `auth-axum-login` | Session-based authentication |
| `auth-oidc` | OpenID Connect integration |
| `auth-jwt` | JWT token authentication |
| `auth-full` | All auth features combined |

### Common Configurations

**JWT-only API (stateless)**:
```toml
default = ["sqlite", "auth-jwt"]
```

**OIDC + JWT (typical SPA backend)**:
```toml
default = ["sqlite", "auth-oidc", "auth-jwt"]
```

**Full-featured (all auth methods)**:
```toml
default = ["sqlite", "auth-full"]
```

## API Endpoints

### Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/auth/login` | Login with email/password |
| `POST` | `/api/v1/auth/register` | Register new user |
| `POST` | `/api/v1/auth/logout` | Logout (session mode) |
| `GET` | `/api/v1/auth/me` | Get current user |
| `POST` | `/api/v1/auth/token` | Get JWT for session user |
| `GET` | `/api/v1/auth/login/oidc` | Start OIDC login flow |
| `GET` | `/api/v1/auth/callback` | OIDC callback |

## Project Structure

```
k-template/
├── domain/          # Core business logic (no I/O dependencies)
│   └── src/
│       ├── entities.rs       # User entity
│       ├── value_objects.rs  # Email, Password, OIDC newtypes
│       ├── repositories.rs   # Repository interfaces (ports)
│       └── services.rs       # Domain services
│
├── infra/           # Infrastructure adapters
│   └── src/
│       ├── auth/             # Auth backends (OIDC, JWT)
│       └── user_repository.rs
│
├── api/             # HTTP API layer
│   └── src/
│       ├── routes/           # API endpoints
│       ├── config.rs         # Configuration
│       └── state.rs          # Application state
│
├── .env.example     # Configuration template
└── compose.yml      # Docker Compose for local dev
```

## Development

### Running Tests

```bash
# All tests
cargo test --all-features

# Domain tests only
cargo test -p domain
```

### Database Migrations

```bash
# SQLite
sqlx migrate run --source migrations_sqlite

# PostgreSQL  
sqlx migrate run --source migrations_postgres
```

## License

MIT
