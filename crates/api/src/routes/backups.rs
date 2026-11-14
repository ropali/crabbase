use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{delete, get},
};
use crabbase_core::{errors::APIError, models::PaginationParams};
use serde_json::{Value, json};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{middleware::auth::require_admin, state::AppState};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", delete(remove))
        .route("/{id}/download", get(download))
        .layer(from_fn_with_state(state.clone(), require_admin))
        .with_state(state)
}

async fn create(State(state): State<AppState>) -> Result<(StatusCode, Json<Value>), APIError> {
    let (backup, job) = state.backup_service().enqueue_manual().await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "backup": backup,
            "jobId": job.id,
        })),
    ))
}

async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, APIError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let result = state.backups_repo().list(page, per_page).await?;
    Ok(Json(json!(result)))
}

async fn download(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Response, APIError> {
    let backup = state
        .backups_repo()
        .get(id)
        .await?
        .ok_or(APIError::NotFound {
            resource: "backup".to_string(),
        })?;

    if backup.status != "completed" {
        return Err(APIError::Conflict {
            message: format!(
                "backup is {}; download is available after completion",
                backup.status
            ),
        });
    }

    // Defense in depth: filenames are generated server-side, but never trust a stored path.
    let filename = std::path::Path::new(&backup.filename);
    if filename.components().count() != 1 {
        return Err(APIError::Internal {
            message: "invalid backup filename".to_string(),
            details: json!(backup.filename),
        });
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", backup.filename)).map_err(
            |e| APIError::Internal {
                message: "invalid header".to_string(),
                details: json!(e.to_string()),
            },
        )?,
    );

    // Cloud backups are read straight from remote storage; the local copy is
    // only a transient artifact of the upload step.
    if backup.storage_backend == "cloud" {
        let storage =
            state
                .file_service()
                .resolve_storage()
                .await
                .map_err(|e| APIError::Internal {
                    message: "backup storage is unavailable".to_string(),
                    details: json!(format!("{e:?}")),
                })?;

        let bytes = storage
            .get_object(&crabbase_jobs::backup_object_key(&backup.filename))
            .await
            .map_err(|e| match e {
                crabbase_files::StorageError::NotFound(_) => APIError::NotFound {
                    resource: "backup".to_string(),
                },
                other => APIError::Internal {
                    message: "failed to read cloud backup".to_string(),
                    details: json!(other.to_string()),
                },
            })?;

        return Ok((headers, bytes).into_response());
    }

    let settings = crabbase_jobs::load_backup_settings(&state.db).await?;
    let path = settings.dir.join(filename);

    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| APIError::Internal {
            message: "backup file is missing".to_string(),
            details: json!(e.to_string()),
        })?;

    // Stream; never read a multi-GB backup into memory.
    let body = Body::from_stream(ReaderStream::new(file));
    Ok((headers, body).into_response())
}

async fn remove(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, APIError> {
    state.backup_service().delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
