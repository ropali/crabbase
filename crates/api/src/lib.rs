pub mod middleware;
pub mod routes;
pub mod state;

use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::{Json, Router, response::Html, routing::get};
use state::AppState;

/// Upper bound for request bodies (multipart file uploads included).
pub const MAX_REQUEST_BODY_BYTES: usize = 100 * 1024 * 1024;

#[cfg(debug_assertions)]
const OPENAPI_JSON: &str = include_str!("../../../openapi.json");

#[cfg(debug_assertions)]
const SWAGGER_HTML: &str = include_str!("routes/swagger.html");

pub fn get_app_routes(state: AppState) -> Router {
    let collections_router = routes::collections::get_routes(state.clone());
    let mut api = Router::new()
        .nest("/collections", collections_router.clone())
        .nest("/collection", collections_router)
        .nest("/auth", routes::auth::get_routes(state.clone()))
        .nest("/settings", routes::setting::get_routes(state.clone()))
        .nest("/files", routes::files::get_routes(state.clone()))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(state);

    #[cfg(debug_assertions)]
    {
        api = api
            .route("/docs", get(swagger_ui))
            .route("/openapi.json", get(openapi_json))
    }

    Router::new().nest("/api", api)
}

#[cfg(debug_assertions)]
async fn openapi_json() -> Json<serde_json::Value> {
    let spec: serde_json::Value = serde_json::from_str(OPENAPI_JSON).unwrap();
    Json(spec)
}

#[cfg(debug_assertions)]
async fn swagger_ui() -> Result<Html<String>, StatusCode> {
    Ok(Html(SWAGGER_HTML.to_string()))
}
