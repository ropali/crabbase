use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    routing::get,
};

use crate::api::{
    models::{
        CreateRecordRequest, PaginationParams, Record, RecordListResponse, UpdateRecordRequest,
    },
    state::AppState,
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
) -> Result<Json<RecordListResponse>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).max(1).min(100);

    match state.store.list_records(&name, page, per_page) {
        Some(response) => Ok(Json(response)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
) -> Result<Json<Record>, StatusCode> {
    match state.store.get_record(&name, &id) {
        Some(record) => Ok(Json(record)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_record(
    Path(name): Path<String>,
    state: axum::extract::State<AppState>,
    Json(body): Json<CreateRecordRequest>,
) -> Result<(StatusCode, Json<Record>), StatusCode> {
    match state.store.create_record(&name, body.data) {
        Some(record) => Ok((StatusCode::CREATED, Json(record))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn update_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
    Json(body): Json<UpdateRecordRequest>,
) -> Result<Json<Record>, StatusCode> {
    match state.store.update_record(&name, &id, body.data) {
        Some(record) => Ok(Json(record)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
) -> Result<StatusCode, StatusCode> {
    if state.store.delete_record(&name, &id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
