use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use serde_json::{Value, json};

use crate::{
    api::{
        models::{
            CreateRecordRequest, PaginationParams, Record, RecordListResponse, UpdateRecordRequest,
        },
        state::AppState,
    },
    core::errors::APIError,
};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_records).post(create_record))
        .route(
            "/{id}",
            get(get_record).patch(update_record).delete(delete_record),
        )
        .with_state(state)
}

async fn list_records(
    Path(name): Path<String>,
    Query(params): Query<PaginationParams>,
    state: axum::extract::State<AppState>,
) -> Result<Json<RecordListResponse>, APIError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    match state.records_repo().list(&name, page, per_page).await {
        Ok(values) => Ok(Json(values)),
        Err(err) => Err(err.into()),
    }
}

async fn get_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
) -> Result<Json<Record>, APIError> {
    match state.records_repo().get_record(&name, &id).await {
        Ok(res) => Ok(Json(res)),
        Err(err) => Err(err.into()),
    }
}

async fn create_record(
    Path(name): Path<String>,
    state: axum::extract::State<AppState>,
    Json(body): Json<CreateRecordRequest>,
) -> Result<Json<Record>, APIError> {
    match state.records_repo().create_record(name, body).await {
        Ok(res) => Ok(Json(res)),
        Err(err) => Err(err.into()),
    }
}

async fn update_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
    Json(body): Json<UpdateRecordRequest>,
) -> Result<Json<Value>, APIError> {
    match state.records_repo().update_record(&name, &id, body).await {
        Ok(_) => Ok(Json(json!({"details": "record updatedsuccessfully."}))),
        Err(err) => Err(err.into()),
    }
}

async fn delete_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
) -> Result<Json<Value>, APIError> {
    match state.records_repo().delete_record(&name, &id).await {
        Ok(_) => Ok(Json(json!({"details": "record deleted successfully."}))),
        Err(err) => Err(err.into()),
    }
}
