use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        models::{Collection, CollectionListResponse, CreateCollectionRequest},
        routes::records,
        state::AppState,
    },
    core::repositories::collections::CollectionRepository,
};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/{name}/records", records::get_routes(state.clone()))
        .route("/", get(list).post(create))
        .route("/{name}", get(get_one).patch(update).delete(delete))
        .with_state(state)
}

async fn create(
    state: State<AppState>,
    Json(body): Json<CreateCollectionRequest>,
) -> Result<Json<Collection>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());

    repo.create(body).await;

    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn list(state: State<AppState>) -> Result<Json<CollectionListResponse>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());
    let data = repo.list(1, 10).await.unwrap();
    Ok(Json(data))
}

async fn get_one(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<Collection>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());
    match repo.get_by_name(&name).await {
        Ok(collection) => Ok(Json(collection)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete(
    Path(_name): Path<String>,
    _state: State<AppState>,
) -> Result<Json<Collection>, StatusCode> {
    // Would need a delete_collection method on store
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn update(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<Collection>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}
