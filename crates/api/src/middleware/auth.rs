use axum::{
    extract::FromRequestParts,
    http::{Request, request::Parts},
    middleware::Next,
    response::Response,
};
use crabbase_auth::auth::{extract_unverified_claims, verify_token};
use crabbase_core::errors::APIError;
use crabbase_db::repositories::auth::AuthUser;

use crate::state::AppState;

pub struct AuthenticatedUser(pub AuthUser);

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

        Ok(AuthenticatedUser(user))
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
