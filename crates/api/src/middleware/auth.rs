use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, State},
    http::{Request, request::Parts},
    middleware::Next,
    response::Response,
};
use crabbase_auth::auth::{Claims, extract_unverified_claims, verify_token};
use crabbase_core::{errors::APIError, models::Collection, rules::compiler::SqlContext};
use crabbase_db::repositories::auth::AuthUser;
use sqlx::{Column as _, Row as _};

use crate::state::AppState;

pub struct AuthenticatedUser(pub AuthUser);

#[derive(Clone, Debug)]
pub struct AuthRecord(pub serde_json::Value);

pub struct RequestContext(pub SqlContext);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = APIError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(APIError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(APIError::Unauthorized)?;

        let claims = extract_unverified_claims(token).map_err(|_| APIError::Unauthorized)?;

        parts.extensions.insert(claims.clone());

        let collection = state
            .collection_repo()
            .get_by_id(&claims.collection_id)
            .await
            .map_err(|_| APIError::Unauthorized)?;

        let col_token = collection
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("secret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| APIError::Internal {
                message: "Unable to find the collection auth token".to_string(),
                details: serde_json::Value::String(format!(
                    "Collection name is: {}",
                    collection.name
                )),
            })?;

        let user_opt = state
            .user_repo()
            .get_user_by_id(&collection.name, &claims.id)
            .await?;

        let user = user_opt.ok_or(APIError::Unauthorized)?;

        let claims = verify_token(token, &format!("{col_token}-{}", user.token_key))
            .map_err(|_| APIError::Unauthorized)?;

        let user = state.auth_service().verify_session(&claims).await?;

        // Dynamically fetch all fields of the authenticated record from its table
        let escaped_table = crabbase_core::utils::string_utils::quote_ident(&collection.name);
        let sql = format!("SELECT * FROM {} WHERE id = $1", escaped_table);

        let id_uuid = uuid::Uuid::parse_str(&claims.id).ok();

        let query = sqlx::query(&sql);
        let query = if let Some(uuid) = id_uuid {
            query.bind(uuid)
        } else {
            query.bind(&claims.id)
        };
        let row_opt = query
            .fetch_optional(&state.db)
            .await
            .map_err(|e| APIError::Internal {
                message: "Failed to fetch dynamic auth record".to_string(),
                details: serde_json::json!(e.to_string()),
            })?;

        if let Some(row) = row_opt {
            let mut fields = serde_json::Map::new();
            for col in row.columns() {
                let col_name = col.name();
                let val = crabbase_core::models::Record::column_to_value(&row, col_name);
                fields.insert(col_name.to_string(), val);
            }
            // Add collection metadata fields
            fields.insert(
                "collectionId".to_string(),
                serde_json::Value::String(collection.id.clone()),
            );
            fields.insert(
                "collectionName".to_string(),
                serde_json::Value::String(collection.name.clone()),
            );

            parts
                .extensions
                .insert(AuthRecord(serde_json::Value::Object(fields)));
        }

        Ok(AuthenticatedUser(user))
    }
}

impl FromRequestParts<AppState> for RequestContext {
    type Rejection = APIError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mut auth_val = None;

        // Resolve Auth user context dynamically if logged in
        if let Some(AuthRecord(record_val)) = parts.extensions.get::<AuthRecord>() {
            auth_val = Some(record_val.clone());
        }

        // Parse query String params
        let query_string = parts.uri.query().unwrap_or("");
        let query_params: HashMap<String, String> =
            serde_urlencoded::from_str(query_string).unwrap_or_default();

        // Assemble and return RequestContext containing the SqlContext
        Ok(RequestContext(SqlContext {
            auth: auth_val,
            query: query_params,
            data: None,
        }))
    }
}

pub async fn require_admin(
    AuthenticatedUser(user): AuthenticatedUser,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, APIError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(APIError::Unauthorized)?;

    if !claims.collection_name.eq_ignore_ascii_case("_superusers") {
        return Err(APIError::Forbidden);
    }

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

pub async fn extract_auth_context(
    state: State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, APIError> {
    let token_opt = if let Some(header_val) = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
    {
        header_val.strip_prefix("Bearer ").map(|s| s.to_string())
    } else {
        req.uri()
            .query()
            .and_then(|q| serde_urlencoded::from_str::<HashMap<String, String>>(q).ok())
            .and_then(|params| params.get("token").cloned())
    };

    let token = match token_opt {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };

    let claims = extract_unverified_claims(&token).map_err(|_| APIError::Unauthorized)?;

    // `?token=` may carry a file token on /api/files/... routes. It is not a
    // request auth token; the file handler resolves it against the file's
    // owning collection. Returning early also means a file token can never
    // authenticate records, settings, or admin routes.
    if claims.token_type == "file" {
        return Ok(next.run(req).await);
    }

    let collection = state
        .collection_repo()
        .get_by_id(&claims.collection_id)
        .await
        .map_err(|_| APIError::Unauthorized)?;

    let col_token = collection
        .options
        .auth_token
        .as_ref()
        .and_then(|t| t.get("secret"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| APIError::Internal {
            message: "Unable to find the collection auth token".to_string(),
            details: serde_json::Value::String(format!("Collection name is: {}", collection.name)),
        })?;

    let user_opt = state
        .user_repo()
        .get_user_by_id(&collection.name, &claims.id)
        .await?;

    let user = user_opt.ok_or(APIError::Unauthorized)?;

    let claims = verify_token(&token, &format!("{col_token}-{}", user.token_key))
        .map_err(|_| APIError::Unauthorized)?;

    let user = state.auth_service().verify_session(&claims).await?;

    // Dynamically fetch all fields of the authenticated record from its table
    let escaped_table = crabbase_core::utils::string_utils::quote_ident(&collection.name);
    let sql = format!("SELECT * FROM {} WHERE id = $1", escaped_table);
    let id_uuid = uuid::Uuid::parse_str(&claims.id).ok();
    let query = sqlx::query(&sql);
    let query = if let Some(uuid) = id_uuid {
        query.bind(uuid)
    } else {
        query.bind(&claims.id)
    };
    let row_opt = query
        .fetch_optional(&state.db)
        .await
        .map_err(|e| APIError::Internal {
            message: "Failed to fetch dynamic auth record".to_string(),
            details: serde_json::json!(e.to_string()),
        })?;

    if let Some(row) = row_opt {
        let mut fields = serde_json::Map::new();
        for col in row.columns() {
            let col_name = col.name();
            let val = crabbase_core::models::Record::column_to_value(&row, col_name);
            fields.insert(col_name.to_string(), val);
        }
        // Add collection metadata fields
        fields.insert(
            "collectionId".to_string(),
            serde_json::Value::String(collection.id.clone()),
        );
        fields.insert(
            "collectionName".to_string(),
            serde_json::Value::String(collection.name.clone()),
        );

        req.extensions_mut()
            .insert(AuthRecord(serde_json::Value::Object(fields)));
    }

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

fn collection_auth_secret(collection: &Collection) -> Result<&str, APIError> {
    collection
        .options
        .auth_token
        .as_ref()
        .and_then(|t| t.get("secret"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| APIError::Internal {
            message: "Unable to find the collection auth token".to_string(),
            details: serde_json::Value::String(format!("Collection name is: {}", collection.name)),
        })
}

/// Signing key for file tokens. The `-file` suffix prevents a file token from
/// ever being accepted as a regular auth token (and vice versa).

pub fn file_token_signing_key(
    collection: &Collection,
    user_token_key: &str,
) -> Result<String, APIError> {
    Ok(format!(
        "{}-{}-file",
        collection_auth_secret(collection)?,
        user_token_key
    ))
}

/// Returns true only for a non-expired JWT whose payload says `type: "file"`.
/// The signature is not checked here; `file_token_auth_record` does that
pub fn is_file_token(token: &str) -> bool {
    extract_unverified_claims(token)
        .map(|claims| claims.token_type == "file")
        .unwrap_or(false)
}

/// Loads the authenticated record fields as the `@request.auth` JSON object.
/// This is the same shape `extract_auth_context` builds for Bearer auth.
async fn build_auth_record_value(
    state: &AppState,
    collection: &Collection,
    user_id: &str,
) -> Result<serde_json::Value, APIError> {
    let escaped_table = crabbase_core::utils::string_utils::quote_ident(&collection.name);
    let sql = format!("SELECT * FROM {} WHERE id = $1", escaped_table);

    let id_uuid = uuid::Uuid::parse_str(user_id).ok();
    let query = sqlx::query(&sql);
    let query = if let Some(uuid) = id_uuid {
        query.bind(uuid)
    } else {
        query.bind(user_id)
    };

    let row_opt = query
        .fetch_optional(&state.db)
        .await
        .map_err(|e| APIError::Internal {
            message: "Failed to fetch dynamic auth record".to_string(),
            details: serde_json::json!(e.to_string()),
        })?;

    let Some(row) = row_opt else {
        return Err(APIError::Unauthorized);
    };

    let mut fields = serde_json::Map::new();
    for col in row.columns() {
        let col_name = col.name();
        let val = crabbase_core::models::Record::column_to_value(&row, col_name);
        fields.insert(col_name.to_string(), val);
    }

    fields.insert(
        "collectionId".to_string(),
        serde_json::Value::String(collection.id.clone()),
    );
    fields.insert(
        "collectionName".to_string(),
        serde_json::Value::String(collection.name.clone()),
    );

    Ok(serde_json::Value::Object(fields))
}

/// Resolves a `type: "file"` JWT into an auth record. This is the Crabbase
/// equivalent of PocketBase's `FindAuthRecordByToken(token, TokenTypeFile)`.
async fn file_token_auth_record(
    state: &AppState,
    token: &str,
) -> Result<serde_json::Value, APIError> {
    let unverified = extract_unverified_claims(token).map_err(|_| APIError::Unauthorized)?;
    if unverified.token_type != "file" {
        return Err(APIError::Unauthorized);
    }

    let collection = state
        .collection_repo()
        .get_by_id(&unverified.collection_id)
        .await
        .map_err(|_| APIError::Unauthorized)?;

    let user = state
        .user_repo()
        .get_user_by_id(&collection.name, &unverified.id)
        .await?
        .ok_or(APIError::Unauthorized)?;

    let claims = verify_token(
        token,
        &file_token_signing_key(&collection, &user.token_key)?,
    )
    .map_err(|_| APIError::Unauthorized)?;

    if claims.token_type != "file" {
        return Err(APIError::Unauthorized);
    }

    if !user.verified {
        return Err(APIError::Forbidden);
    }

    build_auth_record_value(state, &collection, &user.id).await
}

/// Replaces the request auth with a file-token identity when `?token=` is a
/// file token. Regular auth tokens are left alone (already handled by the
/// `extract_auth_context` middleware).
pub async fn resolve_file_token(
    state: &AppState,
    token: Option<&str>,
    mut sql_context: SqlContext,
) -> Result<SqlContext, APIError> {
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        if is_file_token(token) {
            sql_context.auth = Some(file_token_auth_record(state, token).await?);
        }
    }
    Ok(sql_context)
}
