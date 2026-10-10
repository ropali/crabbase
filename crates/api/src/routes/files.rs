use crate::state::AppState;
use axum::{
    Router,
    body::Body,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{Json, Response},
    routing::{get, post},
};
use crabbase_core::errors::APIError;
use crabbase_files::{FileStorage, LocalStorage, S3Config, S3Storage};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    Router::new()
        .route("/upload", post(upload_file_handler))
        .route(
            "/{*path}",
            get(download_file_handler).delete(delete_file_handler),
        )
        .with_state(state)
}

pub async fn upload_file_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, APIError> {
    let mut uploaded_keys = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| APIError::Validation {
            message: format!("invalid multipart: {e}"),
            details: serde_json::Value::Null,
        })?
    {
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = field.bytes().await.map_err(|e| APIError::Validation {
            message: format!("failed to read field: {e}"),
            details: serde_json::Value::Null,
        })?;

        let key = state
            .file_service()
            .upload_bytes(&file_name, &content_type, bytes.to_vec())
            .await?;

        uploaded_keys.push(key);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "keys": uploaded_keys
    })))
}

pub async fn download_file_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response, APIError> {
    let data = state.file_service().download(&path).await?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
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
