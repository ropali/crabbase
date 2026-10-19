use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    middleware::from_fn_with_state,
    response::Response,
    routing::{delete, get},
};
use crabbase_core::errors::APIError;
use serde::{Deserialize, Serialize};

use crate::{middleware::auth::require_admin, state::AppState};

#[derive(Deserialize)]
pub struct PresignRequest {
    pub filename: String,
    pub content_type: String,
}

#[derive(Serialize)]
pub struct PresignResponse {
    pub upload_url: String,
    pub storage_key: String,
}

pub fn get_routes(state: AppState) -> Router<AppState> {
    // Downloads are public (openapi marks GET as unauthenticated), while
    // deleting stored bytes is an administrative operation.
    let admin_delete = Router::new()
        .route("/{*path}", delete(delete_file_handler))
        .layer(from_fn_with_state(state.clone(), require_admin));

    Router::new()
        .route("/{*path}", get(download_file_handler))
        .merge(admin_delete)
        .with_state(state)
}

pub async fn download_file_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response, APIError> {
    let data = state.file_service().download(&path).await?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "inline; filename=\"{}\"",
                path.rsplit('/').next().unwrap_or("file")
            ),
        )
        .body(Body::from(data))
        .map_err(|e| APIError::Internal {
            message: format!("failed to build response: {e}"),
            details: serde_json::Value::Null,
        })
}

pub async fn delete_file_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<StatusCode, APIError> {
    state.file_service().delete(&path).await?;
    Ok(StatusCode::NO_CONTENT)
}
