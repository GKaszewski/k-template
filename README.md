# k-template

A cargo-generate template for personal Rust web services. Gives you auth, persistence, logging, CORS, and API docs out of the box so you can start writing domain code immediately.

Follows the same hexagonal/ports-and-adapters architecture used in [thoughts](https://git.gabrielkaszewski.dev/GKaszewski/thoughts) and [movies-diary](https://git.gabrielkaszewski.dev/GKaszewski/movies-diary).

## What you get

- **Full hexagonal architecture** — `domain` → `application` → `adapters` → `presentation` → `bootstrap`, each as a separate crate with clear boundaries
- **JWT auth wired end-to-end** — register, login, and `GET /auth/me` working from day one
- **SQLite or PostgreSQL** — chosen at generation time, migrations included
- **CORS + structured logging** — tower-http middleware configured in bootstrap
- **Scalar API docs** at `/scalar`, OpenAPI JSON at `/api-docs/openapi.json`
- **Optional worker binary** — tokio-based background job runner with an example job
- **Optional OIDC stub** — placeholder adapter for OAuth2/OpenID Connect flows
- **Docker-ready** — multi-stage Dockerfile with dependency layer caching, no live DB needed at build time

## Generate a new project

```bash
cargo generate --git https://git.gabrielkaszewski.dev/GKaszewski/k-template.git
```

You'll be prompted for:

| Option | Choices | Default |
|--------|---------|---------|
| `project_name` | any snake_case string | — |
| `database` | `sqlite`, `postgres` | `sqlite` |
| `worker` | bool | false |
| `auth_oidc` | bool | false |

## Generated project structure

```
crates/
  domain/               pure Rust — entities, value objects, port traits, errors
  application/          use cases (RegisterUser, LoginUser, GetProfile) + test fakes
  api-types/            shared request/response DTOs with OpenAPI derives
  adapters/
    sqlite/             sqlx SQLite UserRepository + migrations
    postgres/           sqlx PostgreSQL UserRepository + migrations
    auth/               BcryptPasswordHasher, JwtTokenIssuer, OidcAdapter stub
  presentation/         axum handlers, JwtClaims extractor, routes, Scalar mount
  bootstrap/            Config from env, factory wiring, main entry point
  worker/               (optional) Job trait, JobRunner, ExampleJob, WorkerConfig
```

## Running locally

```bash
cp .env.example .env
cargo run -p bootstrap
```

The server starts at `http://localhost:3000`.

## Endpoints (out of the box)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/api/v1/auth/register` | — | Create account → `AuthResponse` |
| `POST` | `/api/v1/auth/login` | — | Login → `AuthResponse` |
| `GET`  | `/api/v1/auth/me` | Bearer | Current user profile |
| `GET`  | `/health` | — | `{"status":"ok"}` |
| `GET`  | `/scalar` | — | Interactive API docs |
| `GET`  | `/api-docs/openapi.json` | — | OpenAPI spec |

```bash
# Register
curl -s -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"me@example.com","password":"password123"}' | jq

# Login and get token
TOKEN=$(curl -s -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"me@example.com","password":"password123"}' | jq -r '.token')

# Profile
curl -s http://localhost:3000/api/v1/auth/me \
  -H "Authorization: Bearer $TOKEN" | jq
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite://data.db` | Database connection string |
| `JWT_SECRET` | *(required)* | Signing secret — min 32 chars in production |
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | Listen port |
| `CORS_ALLOWED_ORIGINS` | `http://localhost:3000` | Comma-separated allowed origins |

## Tests

```bash
# Unit tests (no DB required)
cargo test -p domain -p application -p adapters-auth
```

13 unit tests cover email validation, use case logic (register/login/get_profile), bcrypt roundtrip, and JWT encode/verify.

## Docker

```bash
# Build
docker build -t my-app .

# Run
docker run -p 3000:3000 \
  -e DATABASE_URL=sqlite:///data/app.db \
  -e JWT_SECRET=change-me-32-chars-minimum-here \
  my-app
```

Or with compose:

```bash
docker compose up
```

The Dockerfile uses dependency layer caching (manifests copied and fetched before source) so rebuilds after source-only changes are fast. No live database is needed at compile time — the `.sqlx` offline cache is committed.

## What to do after generating

1. Add your domain entities and value objects to `crates/domain/`
2. Write use cases in `crates/application/`
3. Add DB columns/tables via new migration files in `crates/adapters/sqlite/migrations/`
4. Add handlers in `crates/presentation/src/handlers/`
5. Wire new use cases in `crates/bootstrap/src/factory.rs`

Auth, CORS, logging, and docs are already done — focus on what makes your project unique.

## License

MIT
