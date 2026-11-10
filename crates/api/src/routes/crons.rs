use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    routing::{get, patch, post},
};
use crabbase_core::errors::APIError;
use crabbase_jobs::NewJob;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{middleware::auth::require_admin, state::AppState};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", patch(update))
        .route("/{id}/run", post(run_now))
        .layer(from_fn_with_state(state.clone(), require_admin))
        .with_state(state)
}

async fn list(State(state): State<AppState>) -> Result<Json<Value>, APIError> {
    let crons = state.crons_repo().list().await?;
    Ok(Json(json!(crons)))
}

#[derive(serde::Deserialize)]
pub struct UpdateCron {
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

async fn update(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<UpdateCron>,
) -> Result<Json<Value>, APIError> {
    let next_run = match body.schedule.as_deref() {
        Some(expr) => Some(
            crabbase_jobs::next_occurrence(expr, chrono::Utc::now()).map_err(|e| {
                APIError::Validation {
                    message: format!("invalid cron schedule: {e}"),
                    details: json!({ "schedule": expr }),
                }
            })?,
        ),
        None => None,
    };

    let cron = state
        .crons_repo()
        .update(id, body.schedule, body.enabled, next_run)
        .await?;

    Ok(Json(json!(cron)))
}

async fn run_now(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), APIError> {
    let cron = state
        .crons_repo()
        .get(id)
        .await?
        .ok_or(APIError::NotFound {
            resource: "cron".to_string(),
        })?;

    let job = state
        .jobs_repo()
        .enqueue(NewJob {
            kind: cron.kind,
            payload: cron.payload,
            cron_id: Some(cron.id),
            dedupe_key: None,
            ..Default::default()
        })
        .await?;

    match job {
        Some(job) => {
            let _ = state.crons_repo().set_last_status(cron.id, "queued").await;
            Ok((StatusCode::ACCEPTED, Json(json!({ "jobId": job.id }))))
        }
        None => Err(APIError::Conflict {
            message: "a run for this cron is already queued or running".to_string(),
        }),
    }
}
