pub mod middleware;
pub mod routes;
pub mod state;

use axum::http::StatusCode;
use axum::{Json, Router, response::Html, routing::get};
use state::AppState;

#[cfg(debug_assertions)]
const OPENAPI_JSON: &str = include_str!("../../../openapi.json");

#[cfg(debug_assertions)]
const SWAGGER_HTML: &str = include_str!("routes/swagger.html");

pub fn get_app_routes(state: AppState) -> Router {
    let mut api = Router::new()
        .nest(
            "/collections",
            routes::collections::get_routes(state.clone()),
        )
        .nest("/auth", routes::auth::get_routes(state.clone()))
        .nest("/settings", routes::setting::get_routes(state.clone()))
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
