use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    routing::{delete, get, patch, post},
};

use crate::api::state::AppState;

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_record).post(create_record))
        .route(
            "/{id}",
            get(get_record).patch(update_record).delete(delete_record),
        )
        .with_state(state)
}

async fn get_record() -> Result<String, StatusCode> {
    Ok(String::from("get record"))
}

async fn create_record() -> Result<String, StatusCode> {
    Ok(String::from("create record"))
}

async fn update_record(Path(id): Path<u32>) -> Result<String, StatusCode> {
    Ok(String::from(format!("update record {}", id)))
}

async fn delete_record(Path(id): Path<u32>) -> Result<String, StatusCode> {
    Ok(String::from(format!("delete record {}", id)))
}
