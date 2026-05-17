use utoipa::{openapi::security::{Http, HttpAuthScheme, SecurityScheme}, Modify, OpenApi};
use utoipa_scalar::{Scalar, Servable};
use axum::Router;
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health,
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::me,
    ),
    components(schemas(
        api_types::requests::RegisterRequest,
        api_types::requests::LoginRequest,
        api_types::responses::AuthResponse,
        api_types::responses::UserResponse,
    )),
    modifiers(&SecurityAddon),
    info(title = "k-template", version = "0.1.0")
)]
pub struct ApiDoc;

struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

pub fn openapi_router() -> Router<AppState> {
    Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route("/api-docs/openapi.json", axum::routing::get(|| async { axum::Json(ApiDoc::openapi()) }))
}
