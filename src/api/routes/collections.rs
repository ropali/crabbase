use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::{Value, json};

use crate::{
    api::{
        models::{
            Collection, CollectionListResponse, CreateCollectionRequest, PaginationParams,
            UpdateCollectionRequest,
        },
        routes::records,
        state::AppState,
    },
    core::errors::APIError,
};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/{name}/records", records::get_routes(state.clone()))
        .route("/", get(list).post(create))
        .route("/{name}/truncate", post(truncate))
        .route("/{name}", get(get_one).patch(update).delete(delete))
        .with_state(state)
}

async fn create(
    state: State<AppState>,
    Json(body): Json<CreateCollectionRequest>,
) -> Result<Json<Collection>, APIError> {
    match state.collection_repo().create(body).await {
        Ok(val) => Ok(Json(val)),
        Err(err) => Err(err.into()),
    }
}

async fn list(
    state: State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<CollectionListResponse>, APIError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    let resp = state.collection_repo().list(page, per_page).await;

    match resp {
        Ok(data) => Ok(Json(data)),
        Err(err) => Err(err.into()),
    }
}

async fn get_one(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<Collection>, APIError> {
    match state.collection_repo().get_by_name(&name).await {
        Ok(collection) => Ok(Json(collection)),
        Err(err) => Err(err.into()),
    }
}

async fn delete(Path(name): Path<String>, state: State<AppState>) -> Result<Json<Value>, APIError> {
    match state.collection_repo().delete(name).await {
        Ok(_) => Ok(Json(json!({"detail": "collection deleted successfully."}))),
        Err(err) => Err(err.into()),
    }
}

async fn update(
    Path(name): Path<String>,
    state: State<AppState>,
    Json(body): Json<UpdateCollectionRequest>,
) -> Result<Json<Collection>, APIError> {
    match state.collection_repo().update(name, body).await {
        Ok(collection) => Ok(Json(collection)),
        Err(err) => Err(err.into()),
    }
}

async fn truncate(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<Value>, APIError> {
    match state.collection_repo().truncate(name).await {
        Ok(_) => Ok(Json(json!({"detail": "collection truncated successfully"}))),
        Err(err) => Err(err.into()),
    }
}
