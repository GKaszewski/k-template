// If you chose postgres at cargo generate time, replace adapters_sqlite with
// adapters_postgres throughout this file (connect, run_migrations, PostgresUserRepository).
use std::sync::Arc;
use anyhow::Result;
use axum::Router;
use axum::http::HeaderValue;
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};

use adapters_auth::{BcryptPasswordHasher, JwtTokenIssuer};
use adapters_sqlite::{connect, run_migrations, SqliteUserRepository};
use application::use_cases::{GetProfile, LoginUser, RegisterUser};
use presentation::{routes::app_router, state::AppState};

use crate::config::Config;

pub async fn build_app(config: &Config) -> Result<Router> {
    let pool = connect(&config.database_url).await?;
    run_migrations(&pool).await?;

    let user_repo = Arc::new(SqliteUserRepository::new(pool));
    let hasher = Arc::new(BcryptPasswordHasher);
    let issuer = Arc::new(JwtTokenIssuer::new(&config.jwt_secret));

    let register_uc = Arc::new(RegisterUser::new(user_repo.clone(), hasher.clone()));
    let login_uc = Arc::new(LoginUser::new(user_repo.clone(), hasher, issuer.clone()));
    let get_profile_uc = Arc::new(GetProfile::new(user_repo));

    let state = AppState::new(register_uc, login_uc, get_profile_uc, issuer);

    let cors = CorsLayer::new()
        .allow_origin(
            config.cors_allowed_origins.iter()
                .filter_map(|o| o.parse::<HeaderValue>().ok())
                .collect::<Vec<_>>(),
        )
        .allow_methods(Any)
        .allow_headers(Any);

    Ok(app_router()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors))
}
