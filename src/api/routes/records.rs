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

    todo!()
}

async fn get_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
) -> Result<Json<Record>, StatusCode> {
    todo!()
}

async fn create_record(
    Path(name): Path<String>,
    state: axum::extract::State<AppState>,
    Json(body): Json<CreateRecordRequest>,
) -> Result<(StatusCode, Json<Record>), StatusCode> {
    todo!()
}

async fn update_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
    Json(body): Json<UpdateRecordRequest>,
) -> Result<Json<Record>, StatusCode> {
    todo!()
}

async fn delete_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
) -> Result<StatusCode, StatusCode> {
    todo!()
}
