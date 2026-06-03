use axum::{
    Json, Router,
    extract::FromRequestParts,
    http::request::Parts,
    routing::{get, post},
};
use crabbase_auth::auth::{Claims, TokenType, create_token, verify_token};
use crabbase_core::errors::APIError;
use crabbase_db::repositories::auth::AuthUser;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    token: String,
}

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

        let claims = verify_token(token).map_err(|_| APIError::Unauthorized)?;

        let user = state.auth_service().verify_session(&claims).await?;
        Ok(AuthenticatedUser(user))
    }
}

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/profile", get(profile))
        .with_state(state)
}

async fn login(Json(body): Json<LoginRequest>) -> Result<Json<LoginResponse>, APIError> {
    let token = create_token(&body.username, TokenType::Auth).unwrap();

    Ok(Json(LoginResponse { token }))
}

async fn profile(AuthenticatedUser(user): AuthenticatedUser) -> Result<Json<Value>, APIError> {
    Ok(Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
    })))
}
