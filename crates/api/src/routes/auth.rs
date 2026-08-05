use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use crabbase_auth::service::AuthTokens;
pub(crate) use crabbase_core::errors::APIError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{middleware::auth::AuthenticatedUser, state::AppState};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    tokens: AuthTokens,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRefreshRequest {
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    email: String,
}

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{collection}/login", post(login))
        .route("/{collection}/forget-password", post(forget_password))
        .route("/profile", get(profile))
        .route("/{collection}/auth-refresh", post(refresh_token))
        .route("/{collection}/logout", post(logout))
        .with_state(state)
}

async fn login(
    Path(collection): Path<String>,
    state: State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, APIError> {
    let tokens = state
        .auth_service()
        .authenticate(&collection, &body.email, &body.password)
        .await?;

    Ok(Json(LoginResponse { tokens }))
}

async fn profile(AuthenticatedUser(user): AuthenticatedUser) -> Result<Json<Value>, APIError> {
    Ok(Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
    })))
}

async fn refresh_token(
    Path(collection): Path<String>,
    state: State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(payload): Json<AuthRefreshRequest>,
) -> Result<Json<LoginResponse>, APIError> {
    let tokens = state
        .auth_service()
        .refresh_token(&collection, &user.email, &payload.refresh_token)
        .await?;

    Ok(Json(LoginResponse { tokens }))
}

async fn logout(
    Path(collection): Path<String>,
    state: State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, APIError> {
    state
        .auth_service()
        .logout_session(&collection, &payload.email, &payload.refresh_token)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn forget_password(
    state: State<AppState>,
    Path(collection): Path<String>,
    Json(payload): Json<PasswordResetRequest>,
) -> Result<Json<serde_json::Value>, APIError> {
    state
        .auth_service()
        .password_reset(&collection, &payload.email)
        .await?;

    Ok(Json(
        serde_json::json!({ "detail": "Password reset link sent to your email adddress."}),
    ))
}
