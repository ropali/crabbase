use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::api::{routes::records, state::AppState};

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
) -> Result<(StatusCode, Json<CollectionResponse>), StatusCode> {
    if state.store.collection_exists(&body.name) {
        return Err(StatusCode::CONFLICT);
    }
    state.store.create_collection(body.name.clone());
    Ok((StatusCode::CREATED, Json(CollectionResponse { name: body.name })))
}

async fn list(_state: State<AppState>) -> Result<Json<Vec<String>>, StatusCode> {
    // Return collection names - we need to add a method to store for this
    Ok(Json(vec![]))
}

async fn get_one(Path(name): Path<String>, state: State<AppState>) -> Result<Json<CollectionResponse>, StatusCode> {
    if state.store.collection_exists(&name) {
        Ok(Json(CollectionResponse { name }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn delete(Path(_name): Path<String>, _state: State<AppState>) -> Result<StatusCode, StatusCode> {
    // Would need a delete_collection method on store
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn update(Path(name): Path<String>, state: State<AppState>) -> Result<Json<CollectionResponse>, StatusCode> {
    if state.store.collection_exists(&name) {
        Ok(Json(CollectionResponse { name }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
