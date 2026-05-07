use axum::{Router, extract::Path, http::StatusCode, routing::get};

use crate::api::{routes::records, state::AppState};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).patch(update).delete(delete))
        .nest("/records", records::get_routes(state.clone()))
        .with_state(state)
}

async fn create() -> Result<String, StatusCode> {
    Ok(String::from("Collection create"))
}

async fn list() -> Result<String, StatusCode> {
    Ok(String::from("Collection list"))
}

async fn get_one(Path(id): Path<u32>) -> Result<String, StatusCode> {
    Ok(String::from(format!("Collection get_one {}", id)))
}

async fn delete(Path(id): Path<u32>) -> Result<String, StatusCode> {
    Ok(String::from(format!("Collection delete {}", id)))
}

async fn update(Path(id): Path<u32>) -> Result<String, StatusCode> {
    Ok(String::from(format!("Collection update {}", id)))
}
