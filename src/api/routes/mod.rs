pub mod collections;
pub mod files;
pub mod records;

use axum::{Router, routing::get, response::Html, Json};
use axum::http::StatusCode;

use crate::api::state::AppState;

const OPENAPI_JSON: &str = include_str!("../../../openapi.json");
const SWAGGER_HTML: &str = include_str!("swagger.html");

pub fn get_app_routes(state: AppState) -> Router {
    let api = Router::new()
        .nest("/collections", collections::get_routes(state.clone()))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
        .with_state(state);

    Router::new().nest("/api", api)
}

async fn openapi_json() -> Json<serde_json::Value> {
    let spec: serde_json::Value = serde_json::from_str(OPENAPI_JSON).unwrap();
    Json(spec)
}

async fn swagger_ui() -> Result<Html<String>, StatusCode> {
    Ok(Html(SWAGGER_HTML.to_string()))
}
