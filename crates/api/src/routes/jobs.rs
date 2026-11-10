use axum::{
    Json, Router,
    extract::{Path, Query, State},
    middleware::from_fn_with_state,
    routing::get,
};
use crabbase_core::errors::APIError;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{middleware::auth::require_admin, state::AppState};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_one))
        .layer(from_fn_with_state(state.clone(), require_admin))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct JobsQuery {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

async fn list(
    State(state): State<AppState>,
    Query(params): Query<JobsQuery>,
) -> Result<Json<Value>, APIError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    let result = state
        .jobs_repo()
        .list(
            page,
            per_page,
            params.status.as_deref(),
            params.kind.as_deref(),
        )
        .await?;

    Ok(Json(serde_json::json!(result)))
}

async fn get_one(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Value>, APIError> {
    let job = state.jobs_repo().get(id).await?.ok_or(APIError::NotFound {
        resource: "job".to_string(),
    })?;

    Ok(Json(serde_json::json!(job)))
}
