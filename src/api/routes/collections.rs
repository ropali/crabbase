use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use tracing::error;

use crate::{
    api::{
        models::{
            Collection, CollectionListResponse, CreateCollectionRequest, UpdateCollectionRequest,
        },
        routes::records,
        state::AppState,
    },
    core::repositories::collections::{CollectionRepository, RepositoryError},
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
) -> Result<Json<Collection>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());

    match repo.create(body).await {
        Ok(val) => Ok(Json(val)),
        Err(err) => {
            error!("Error: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn list(state: State<AppState>) -> Result<Json<CollectionListResponse>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());
    let resp = repo.list(1, 10).await;

    match resp {
        Ok(data) => Ok(Json(data)),
        Err(err) => {
            error!("Error: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
    state: State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());

    match repo.delete(_name).await {
        Ok(_) => Ok(Json(json!({"detail": "collection deleted successfully."}))),
        Err(err) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn update(
    Path(name): Path<String>,
    state: State<AppState>,
    Json(body): Json<UpdateCollectionRequest>,
) -> Result<Json<Collection>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());

    match repo.update(name, body).await {
        Ok(collection) => Ok(Json(collection)),
        Err(RepositoryError::NotFound) => Err(StatusCode::NOT_FOUND),
        Err(RepositoryError::InvalidInput(_)) => Err(StatusCode::BAD_REQUEST),
        Err(err) => {
            eprintln!("Error: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn truncate(
    Path(name): Path<String>,
    state: State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let repo = CollectionRepository::new(state.db.clone());

    match repo.truncate(name).await {
        Ok(v) => Ok(Json(json!({"detail": "collection truncated successfully"}))),
        Err(err) => {
            error!("Error: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
