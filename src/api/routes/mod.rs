pub mod collections;
pub mod files;
pub mod records;

use axum::Router;

use crate::api::state::AppState;

pub fn get_app_routes(state: AppState) -> Router {
    let api = Router::new()
        .nest("/collections", collections::get_routes(state.clone()))
        .with_state(state);
    // .nest("/files", router())
    // .nest("/settings", router())

    Router::new().nest("/api", api)
}
