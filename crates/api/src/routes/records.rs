use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use serde_json::{Value, json};

use crate::{middleware::auth::RequestContext, state::AppState};
use crabbase_core::{
    errors::APIError,
    models::{
        CreateRecordRequest, PaginationParams, Record, RecordListResponse, UpdateRecordRequest,
    },
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
    RequestContext(sql_context): RequestContext,
) -> Result<Json<RecordListResponse>, APIError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    match state.records_repo().list(&name, sql_context, params).await {
        Ok(values) => Ok(Json(values)),
        Err(err) => Err(err.into()),
    }
}

async fn get_record(
    Path((name, id)): Path<(String, String)>,
    Query(params): Query<PaginationParams>,
    state: axum::extract::State<AppState>,
    RequestContext(sql_context): RequestContext,
) -> Result<Json<Record>, APIError> {
    match state
        .records_repo()
        .get_record(&name, &id, params.expand.as_deref(), &sql_context)
        .await
    {
        Ok(res) => Ok(Json(res)),
        Err(err) => Err(err.into()),
    }
}

async fn create_record(
    Path(name): Path<String>,
    state: axum::extract::State<AppState>,
    RequestContext(sql_context): RequestContext,
    Json(body): Json<CreateRecordRequest>,
) -> Result<Json<Record>, APIError> {
    match state
        .records_repo()
        .create_record(name, body, sql_context)
        .await
    {
        Ok(res) => Ok(Json(res)),
        Err(err) => Err(err.into()),
    }
}

async fn update_record(
    Path((name, id)): Path<(String, String)>,
    state: axum::extract::State<AppState>,
    RequestContext(sql_context): RequestContext,
    Json(body): Json<UpdateRecordRequest>,
) -> Result<Json<Record>, APIError> {
    match state
        .records_repo()
        .update_record(&name, &id, body, &sql_context)
        .await
    {
        Ok(r) => Ok(Json(r)),
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
