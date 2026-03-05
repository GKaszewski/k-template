# k-template

A production-ready, modular Rust API template for K-Suite applications, following Hexagonal Architecture principles.

## Features

- **Hexagonal Architecture**: Clear separation between Domain, Infrastructure, and API layers
- **JWT-Only Authentication**: Stateless Bearer token auth — no server-side sessions
- **OIDC Integration**: Connect to any OpenID Connect provider (Keycloak, Auth0, Zitadel, etc.) with stateless cookie-based flow state
- **Database Flexibility**: SQLite (default) or PostgreSQL via feature flags
- **Type-Safe Domain**: Newtypes with built-in validation for emails, passwords, secrets, and OIDC values
- **Cargo Generate Ready**: Pre-configured for scaffolding new services

## Quick Start

### Option 1: Use cargo-generate (Recommended)

```bash
cargo generate --git https://github.com/GKaszewski/k-template.git
```

You'll be prompted to choose:
- **Project name**: Your new service name
- **Database**: `sqlite` or `postgres`
- **JWT auth**: Enable Bearer token authentication
- **OIDC**: Enable OpenID Connect integration

### Option 2: Clone directly

```bash
git clone https://github.com/GKaszewski/k-template.git my-api
cd my-api
cp .env.example .env
# Edit .env with your configuration
cargo run
```

The API will be available at `http://localhost:3000/api/v1/`.

## Configuration

All configuration is done via environment variables. See [.env.example](.env.example) for all options.

### Key Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:data.db?mode=rwc` | Database connection string |
| `COOKIE_SECRET` | *(insecure dev default)* | Secret for encrypting OIDC state cookie (≥64 bytes in production) |
| `JWT_SECRET` | *(insecure dev default)* | Secret for signing JWT tokens (≥32 bytes in production) |
| `JWT_EXPIRY_HOURS` | `24` | Token lifetime in hours |
| `CORS_ALLOWED_ORIGINS` | `http://localhost:5173` | Comma-separated allowed origins |
| `SECURE_COOKIE` | `false` | Set `true` when serving over HTTPS |
| `PRODUCTION` | `false` | Enforces minimum secret lengths |

### OIDC Integration

To enable "Login with Google/Keycloak/etc.":

1. Enable the `auth-oidc` feature (on by default in cargo-generate)
2. Set these environment variables:
   ```env
   OIDC_ISSUER=https://your-provider.com
   OIDC_CLIENT_ID=your-client-id
   OIDC_CLIENT_SECRET=your-secret
   OIDC_REDIRECT_URL=http://localhost:3000/api/v1/auth/callback
   ```
3. Users start the flow at `GET /api/v1/auth/login/oidc`

OIDC state (CSRF token, PKCE verifier, nonce) is stored in a short-lived encrypted cookie — no database session table required.

## Feature Flags

```toml
[features]
default = ["sqlite", "auth-jwt"]
```

| Feature | Description |
|---------|-------------|
| `sqlite` | SQLite database (default) |
| `postgres` | PostgreSQL database |
| `auth-jwt` | JWT Bearer token authentication |
| `auth-oidc` | OpenID Connect integration |

### Common Configurations

**JWT-only (minimal, default)**:
```toml
default = ["sqlite", "auth-jwt"]
```

**OIDC + JWT (typical SPA backend)**:
```toml
default = ["sqlite", "auth-oidc", "auth-jwt"]
```

**PostgreSQL + OIDC + JWT**:
```toml
default = ["postgres", "auth-oidc", "auth-jwt"]
```

## API Endpoints

### Authentication

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| `POST` | `/api/v1/auth/register` | — | Register with email + password → JWT |
| `POST` | `/api/v1/auth/login` | — | Login with email + password → JWT |
| `POST` | `/api/v1/auth/logout` | — | Returns 200; client drops the token |
| `GET` | `/api/v1/auth/me` | Bearer | Current user info |
| `POST` | `/api/v1/auth/token` | Bearer | Issue a fresh JWT (`auth-jwt`) |
| `GET` | `/api/v1/auth/login/oidc` | — | Start OIDC flow, sets encrypted state cookie (`auth-oidc`) |
| `GET` | `/api/v1/auth/callback` | — | Complete OIDC flow → JWT, clears cookie (`auth-oidc`) |

### Example: Register and use a token

```bash
# Register
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "mypassword"}'
# → {"access_token": "eyJ...", "token_type": "Bearer", "expires_in": 86400}

# Use the token
curl http://localhost:3000/api/v1/auth/me \
  -H "Authorization: Bearer eyJ..."
```

## Project Structure

```
k-template/
├── domain/          # Pure business logic — zero I/O dependencies
│   └── src/
│       ├── entities.rs       # User entity
│       ├── value_objects.rs  # Email, Password, JwtSecret, OIDC newtypes
│       ├── repositories.rs   # Repository interfaces (ports)
│       ├── services.rs       # Domain services
│       └── errors.rs         # DomainError (Unauthenticated 401, Forbidden 403, …)
│
├── infra/           # Infrastructure adapters
│   └── src/
│       ├── auth/
│       │   ├── jwt.rs        # JwtValidator — create + verify tokens
│       │   └── oidc.rs       # OidcService + OidcState (cookie-serializable)
│       ├── user_repository.rs # SQLite / PostgreSQL adapter
│       ├── db.rs             # DatabasePool re-export
│       └── factory.rs        # build_user_repository()
│
├── api/             # HTTP layer
│   └── src/
│       ├── routes/
│       │   ├── auth.rs       # Login, register, logout, me, OIDC flow
│       │   └── config.rs     # /config endpoint
│       ├── config.rs         # Config::from_env()
│       ├── state.rs          # AppState (user_service, cookie_key, jwt_validator, …)
│       ├── extractors.rs     # CurrentUser (JWT Bearer extractor)
│       ├── error.rs          # ApiError → HTTP status mapping
│       └── dto.rs            # LoginRequest, RegisterRequest, TokenResponse, …
│
├── migrations_sqlite/
├── migrations_postgres/
├── .env.example
└── compose.yml      # Docker Compose for local dev
```

## Development

### Running Tests

```bash
# All tests
cargo test

# Domain only
cargo test -p domain

# Infra only (SQLite integration tests)
cargo test -p infra
```

### Database Migrations

```bash
# SQLite
sqlx migrate run --source migrations_sqlite

# PostgreSQL
sqlx migrate run --source migrations_postgres
```

### Building with specific features

```bash
# Minimal: SQLite + JWT only
cargo build -F sqlite,auth-jwt

# Full: SQLite + JWT + OIDC
cargo build -F sqlite,auth-jwt,auth-oidc

# PostgreSQL variant
cargo build --no-default-features -F postgres,auth-jwt,auth-oidc
```

## License

MIT
