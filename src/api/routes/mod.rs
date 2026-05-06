use axum::{Json, Router, routing::get};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct AppState {}

fn router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(async || Json(json!({"message": "Hello, Crabbase"}))),
    )
}

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .nest("/collections", router())
        .nest("/files", router())
        .nest("/settings", router())
        .with_state(state);

    Router::new().nest("/api", api)
}
