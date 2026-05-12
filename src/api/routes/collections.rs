use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{models::CollectionListResponse, routes::records, state::AppState},
    core::repositories::collections::CollectionRepository,
};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/{name}/records", records::get_routes(state.clone()))
        .route("/", get(list).post(create))
        .route("/{name}", get(get_one).patch(update).delete(delete))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionResponse {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
}

async fn create(
    state: State<AppState>,
    Json(body): Json<CreateCollectionRequest>,
) -> Result<(), StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());

    repo.create(body.name).await;

    Ok(())
}

async fn list(state: State<AppState>) -> Result<Json<CollectionListResponse>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());
    let data = repo.list(1, 10).await.unwrap();
    Ok(Json(data))
}

async fn get_one(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<CollectionResponse>, StatusCode> {
    todo!()
}

async fn delete(
    Path(_name): Path<String>,
    _state: State<AppState>,
) -> Result<StatusCode, StatusCode> {
    // Would need a delete_collection method on store
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn update(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<CollectionResponse>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}
