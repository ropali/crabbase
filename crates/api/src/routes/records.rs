use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::header,
    response::Response,
    routing::{get, post},
};
use serde_json::{Value, json};

use std::collections::HashMap;

use crate::{middleware::auth::RequestContext, state::AppState};
use crabbase_core::{
    errors::APIError,
    models::{
        CreateRecordRequest, DataTypes, PaginationParams, Record, RecordListResponse,
        UpdateRecordRequest,
    },
};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_records).post(create_record))
        .route("/upload", post(upload_record_file))
        .route(
            "/{id}",
            get(get_record).patch(update_record).delete(delete_record),
        )
        .with_state(state)
}

/// Collection-scoped file routes. Uploading goes through the records API
/// (`/{name}/records/upload`); here files can only be listed or downloaded.
pub fn get_file_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{name}/files", get(list_collection_files))
        .route("/{name}/files/{*file_id}", get(download_collection_file))
        .with_state(state)
}

async fn list_records(
    Path(name): Path<String>,
    Query(params): Query<PaginationParams>,
    state: axum::extract::State<AppState>,
    RequestContext(sql_context): RequestContext,
) -> Result<Json<RecordListResponse>, APIError> {
    match state.records_repo().list(&name, sql_context, params).await {
        Ok(values) => Ok(Json(values)),
        Err(err) => Err(err.into()),
    }
}

async fn get_record(
    Path((name, id)): Path<(String, String)>,
    Query(params): Query<PaginationParams>,
    state: State<AppState>,
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
    state: State<AppState>,
    RequestContext(mut sql_context): RequestContext,
    Json(body): Json<CreateRecordRequest>,
) -> Result<Json<Record>, APIError> {
    sql_context.data = Some(Value::Object(body.data.clone()));
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
    state: State<AppState>,
    RequestContext(sql_context): RequestContext,
    Json(body): Json<UpdateRecordRequest>,
) -> Result<Json<Record>, APIError> {
    let updated = state
        .records_repo()
        .update_record(&name, &id, body, &sql_context)
        .await
        .map_err(APIError::from)?;

    Ok(Json(updated))
}

async fn delete_record(
    Path((name, id)): Path<(String, String)>,
    state: State<AppState>,
    RequestContext(sql_context): RequestContext,
) -> Result<Json<Value>, APIError> {
    state
        .records_repo()
        .delete_record(&name, &id, &sql_context)
        .await
        .map_err(APIError::from)?;

    Ok(Json(json!({"details": "record deleted successfully."})))
}

async fn upload_record_file(
    Path(name): Path<String>,
    State(state): State<AppState>,
    RequestContext(sql_context): RequestContext,
    mut multipart: Multipart,
) -> Result<Json<Value>, APIError> {
    let collection = state
        .records_repo()
        .authorize_upload(&name, &sql_context)
        .await
        .map_err(APIError::from)?;

    // Multipart parts named after a file column get that column's constraints.
    // Parts that do not match a file column keep the generic upload behavior.
    let file_columns: Vec<&crabbase_core::models::Column> = collection
        .fields
        .iter()
        .filter(|c| c.data_type == DataTypes::File)
        .collect();

    let mut file_inputs = Vec::new();
    let mut part_counts: HashMap<String, usize> = HashMap::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| APIError::Validation {
            message: format!("invalid multipart: {e}"),
            details: Value::Null,
        })?
    {
        let part_name = field.name().unwrap_or("file").to_string();
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let column = file_columns.iter().copied().find(|c| c.name == part_name);

        if let Some(col) = column {
            let seen = part_counts.entry(col.name.clone()).or_insert(0);
            *seen += 1;
            // How many parts this field takes comes straight from min/max:
            // unset max keeps the legacy single-file rule, unless min already
            // describes a range above one.
            let max_parts = col.max.unwrap_or(if col.min.map_or(false, |m| m > 1) {
                usize::MAX
            } else {
                1
            });
            if *seen > max_parts {
                let message = if max_parts == 1 {
                    format!("field '{}' accepts a single file", col.name)
                } else {
                    format!("field '{}' accepts at most {} file(s)", col.name, max_parts)
                };
                return Err(APIError::Validation {
                    message,
                    details: Value::Null,
                });
            }
        }

        let bytes = field.bytes().await.map_err(|e| APIError::Validation {
            message: format!("failed to read field: {e}"),
            details: Value::Null,
        })?;

        if let Some(col) = column {
            col.validate_file_upload(&file_name, &content_type, bytes.len())
                .map_err(|message| APIError::Validation {
                    message,
                    details: Value::Null,
                })?;
        }

        file_inputs.push(crabbase_files::UploadFileInput::new(
            file_name,
            content_type,
            bytes.to_vec(),
        ));
    }

    if file_inputs.is_empty() {
        return Err(APIError::Validation {
            message: "No file was uploaded".to_string(),
            details: Value::Null,
        });
    }

    let uploaded_results = state.file_service().upload(&name, file_inputs).await?;

    // Response mirrors the stored `_files` metadata rows. The rare case where an
    // object was stored but its metadata row could not be written falls back to
    // the storage key so callers can still reference the uploaded object.
    let files: Vec<Value> = uploaded_results
        .into_iter()
        .map(|uploaded| match uploaded.record {
            Some(record) => serde_json::to_value(record).unwrap_or(Value::Null),
            None => json!({
                "id": Value::Null,
                "key": uploaded.key,
                "backend": uploaded.backend,
                "collectionName": name,
            }),
        })
        .collect();

    Ok(Json(json!({ "files": files })))
}

pub async fn list_collection_files(
    Path(name): Path<String>,
    State(state): State<AppState>,
    RequestContext(sql_context): RequestContext,
) -> Result<Json<Value>, APIError> {
    // 1. Authorize list_rule for collection
    state
        .records_repo()
        .authorize_list(&name, &sql_context)
        .await
        .map_err(APIError::from)?;

    // 2. Fetch all files for this collection
    let files = state.file_service().list_by_collection(&name).await?;

    let items: Vec<Value> = files
        .into_iter()
        .map(|f| {
            let path = format!("/api/collections/{}/files/{}", name, f.id);
            json!({
                "id": f.id,
                "key": f.key,
                "name": f.original_name,
                "size": f.size_bytes,
                "contentType": f.content_type,
                "collectionName": f.collection_name,
                "created": f.created,
                "updated": f.updated,
                "path": path,
            })
        })
        .collect();

    let total = items.len();

    Ok(Json(json!({
        "items": items,
        "totalItems": total,
    })))
}

pub async fn download_collection_file(
    Path((name, file_id)): Path<(String, String)>,
    State(state): State<AppState>,
    RequestContext(sql_context): RequestContext,
) -> Result<Response, APIError> {
    // 1. Authorize view_rule and load collection metadata
    state
        .records_repo()
        .authorize_view(&name, &sql_context)
        .await
        .map_err(APIError::from)?;

    // 2. Lookup file metadata if available
    let file_rec = state
        .file_service()
        .get_record_by_id_or_key(&file_id)
        .await?;

    let (content_type, filename) = match &file_rec {
        Some(rec) => (rec.content_type.clone(), rec.original_name.clone()),
        None => {
            let mime = mime_guess::from_path(&file_id)
                .first_or_octet_stream()
                .to_string();
            let fname = file_id.rsplit('/').next().unwrap_or("file").to_string();
            (mime, fname)
        }
    };

    // 3. Fetch file bytes
    let bytes = state.file_service().download(&file_id).await?;

    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(Body::from(bytes))
        .map_err(|e| APIError::Internal {
            message: format!("failed to build response: {e}"),
            details: serde_json::Value::Null,
        })
}
