use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    middleware::from_fn_with_state,
    response::Response,
    routing::{delete, get, post},
};
use crabbase_auth::auth::{Claims, TokenParams, TokenType, create_token};
use crabbase_core::{
    errors::{APIError, RepositoryError},
    rules::compiler::SqlContext,
};
use crabbase_db::repositories::files::FileRecord;
use serde::{Deserialize, Serialize};

use crate::{
    middleware::auth::{
        AuthenticatedUser, RequestContext, extract_auth_context, file_token_signing_key,
        require_admin, resolve_file_token,
    },
    state::AppState,
};

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

#[derive(Debug, Deserialize, Default)]
pub struct FileQuery {
    pub token: Option<String>,
    pub download: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTokenResponse {
    pub token: String,
    pub expires_in: usize,
}

pub const FILE_TOKEN_TTL_SECS: usize = 180;

pub fn get_routes(state: AppState) -> Router<AppState> {
    // Deleting stored bytes is an administrative operation.
    let admin_delete = Router::new()
        .route("/{*path}", delete(delete_file_handler))
        .layer(from_fn_with_state(state.clone(), require_admin));

    // Downloads may be public or protected by the owning collection's
    // view_rule. `extract_auth_context` supports normal Bearer auth; file
    // tokens are skipped there and resolved inside the handler.
    let download = Router::new()
        .route("/{*path}", get(download_file_handler))
        .layer(from_fn_with_state(state.clone(), extract_auth_context));

    Router::new()
        .route("/token", post(issue_file_token))
        .merge(download)
        .merge(admin_delete)
        .with_state(state)
}

pub async fn issue_file_token(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Extension(claims): Extension<Claims>,
) -> Result<Json<FileTokenResponse>, APIError> {
    let collection = state
        .collection_repo()
        .get_by_id(&claims.collection_id)
        .await
        .map_err(|_| APIError::Unauthorized)?;

    let token = create_token(TokenParams {
        user_id: &claims.id,
        collection_id: &claims.collection_id,
        collection_name: &claims.collection_name,
        secret: &file_token_signing_key(&collection, &user.token_key)?,
        token_type: TokenType::File,
        duration: Some(FILE_TOKEN_TTL_SECS),
        jti: None,
        family_id: None,
    })
    .map_err(|e| APIError::Internal {
        message: "Failed to generate file token".to_string(),
        details: serde_json::json!(e.to_string()),
    })?;

    Ok(Json(FileTokenResponse {
        token,
        expires_in: FILE_TOKEN_TTL_SECS,
    }))
}

pub async fn download_file_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FileQuery>,
    RequestContext(sql_context): RequestContext,
) -> Result<Response, APIError> {
    let file_rec = state
        .file_service()
        .get_record_by_id_or_key(&path)
        .await?
        .ok_or_else(|| APIError::NotFound {
            resource: format!("File {path}"),
        })?;

    let sql_context = resolve_file_token(&state, query.token.as_deref(), sql_context).await?;
    authorize_file_view(&state, &file_rec, &sql_context).await?;

    let data = state.file_service().download(&file_rec.key).await?;
    let kind = if is_truthy(query.download.as_deref()) {
        "attachment"
    } else {
        "inline"
    };

    Response::builder()
        .header(header::CONTENT_TYPE, file_rec.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            content_disposition(kind, &file_rec.original_name),
        )
        .body(Body::from(data))
        .map_err(|e| APIError::Internal {
            message: format!("failed to build response: {e}"),
            details: serde_json::Value::Null,
        })
}

/// Applies the owning collection's `view_rule`.
///
/// Compatibility carve-out: old generic uploads have `collection_name =
/// "default"` and no `_collections` row. Those were public before this change
/// and remain public. All files uploaded through a collection have a real
/// collection row and are protected normally.
async fn authorize_file_view(
    state: &AppState,
    file_rec: &FileRecord,
    sql_context: &SqlContext,
) -> Result<(), APIError> {
    match state
        .records_repo()
        .authorize_view(&file_rec.collection_name, sql_context)
        .await
    {
        Ok(_) => Ok(()),
        Err(RepositoryError::NotFound(_)) if file_rec.collection_name == "default" => Ok(()),
        Err(err) => Err(err.into()),
    }
}

pub(crate) fn is_truthy(value: Option<&str>) -> bool {
    matches!(value, Some("1") | Some("true") | Some("TRUE") | Some("yes"))
}

pub(crate) fn content_disposition(kind: &str, filename: &str) -> String {
    let safe: String = filename
        .chars()
        .map(|c| {
            if (c.is_ascii_graphic() && c != '"' && c != '\\') || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let safe = if safe.is_empty() {
        "file".to_string()
    } else {
        safe
    };

    format!("{kind}; filename=\"{safe}\"")
}

pub async fn delete_file_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<StatusCode, APIError> {
    state.file_service().delete(&path).await?;
    Ok(StatusCode::NO_CONTENT)
}
