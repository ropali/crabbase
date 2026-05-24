use axum::{
    Error, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use tracing::error;

use crate::{
    api::{
        models::{
            Collection, CollectionListResponse, CreateCollectionRequest, PaginationParams,
            UpdateCollectionRequest,
        },
        routes::records,
        state::AppState,
    },
    core::{
        errors::{APIError, RepositoryError},
        repositories::collections::CollectionRepository,
    },
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
    let repo = CollectionRepository::new(state.db.clone());

    match repo.create(body).await {
        Ok(val) => Ok(Json(val)),
        Err(err) => {
            error!("Error: {:?}", err);
            Err(APIError::Internal {
                message: "something went wrong".to_string(),
                details: serde_json::Value::String("".to_string()),
            })
        }
    }
}

async fn list(
    state: State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<CollectionListResponse>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).max(1).min(100);

    let repo = CollectionRepository::new(state.db.clone());
    let resp = repo.list(page, per_page).await;

    match resp {
        Ok(data) => Ok(Json(data)),
        Err(err) => {
            error!("Error: {:?}", err);
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

        Err(err) => {
            eprintln!("Error: {:?}", err);
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
            error!("Error: {:?}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
