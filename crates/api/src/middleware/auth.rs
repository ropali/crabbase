use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, State},
    http::{Request, request::Parts},
    middleware::Next,
    response::Response,
};
use crabbase_auth::auth::{extract_unverified_claims, verify_token};
use crabbase_core::{errors::APIError, rules::compiler::SqlContext};
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

        let col = state
            .auth_repo()
            .get_collection_by_id(&claims.collection_id)
            .await
            .map_err(|e| APIError::Internal {
                message: "Database query failed".to_string(),
                details: serde_json::json!(e.to_string()),
            })?
            .ok_or(APIError::Unauthorized)?;

        let col_token = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("secret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| APIError::Internal {
                message: "Unable to find the collection auth token".to_string(),
                details: serde_json::Value::String(format!("Collection name is: {}", col.name)),
            })?;

        let collection = state
            .collection_repo()
            .get_by_id(&claims.collection_id)
            .await?;

        let user_opt = state
            .auth_repo()
            .get_user_by_id(&collection.name, &claims.id)
            .await?;

        let user = user_opt.ok_or(APIError::Unauthorized)?;

        let claims =
            verify_token(token, col_token, &user.token_key).map_err(|_| APIError::Unauthorized)?;

        let user = state.auth_service().verify_session(&claims).await?;

        // Dynamically fetch all fields of the authenticated record from its table
        let escaped_table = crabbase_core::utils::string_utils::quote_ident(&collection.name);
        let sql = format!("SELECT * FROM {} WHERE id = $1", escaped_table);
        let row_opt = sqlx::query(&sql)
            .bind(&claims.id)
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
        }))
    }
}

pub async fn require_admin(
    AuthenticatedUser(user): AuthenticatedUser,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, APIError> {
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

pub async fn extract_auth_context(
    state: State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, APIError> {
    let auth_header_res = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(APIError::Unauthorized);

    let auth_header = match auth_header_res {
        Result::Ok(val) => val,
        Err(_) => return Ok(next.run(req).await),
    };

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(APIError::Unauthorized)?;

    let claims = extract_unverified_claims(token).map_err(|_| APIError::Unauthorized)?;

    let col = state
        .auth_repo()
        .get_collection_by_id(&claims.collection_id)
        .await
        .map_err(|e| APIError::Internal {
            message: "Database query failed".to_string(),
            details: serde_json::json!(e.to_string()),
        })?
        .ok_or(APIError::Unauthorized)?;

    let col_token = col
        .options
        .auth_token
        .as_ref()
        .and_then(|t| t.get("secret"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| APIError::Internal {
            message: "Unable to find the collection auth token".to_string(),
            details: serde_json::Value::String(format!("Collection name is: {}", col.name)),
        })?;

    let collection = state
        .collection_repo()
        .get_by_id(&claims.collection_id)
        .await?;

    let user_opt = state
        .auth_repo()
        .get_user_by_id(&collection.name, &claims.id)
        .await?;

    let user = user_opt.ok_or(APIError::Unauthorized)?;

    let claims =
        verify_token(token, col_token, &user.token_key).map_err(|_| APIError::Unauthorized)?;

    let user = state.auth_service().verify_session(&claims).await?;

    // Dynamically fetch all fields of the authenticated record from its table
    let escaped_table = crabbase_core::utils::string_utils::quote_ident(&collection.name);
    let sql = format!("SELECT * FROM {} WHERE id = $1", escaped_table);
    let row_opt = sqlx::query(&sql)
        .bind(&claims.id)
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
