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
    otp: Option<u32>,
    new_pwd: Option<String>,
}

impl PasswordResetRequest {
    fn validate(&self) -> Result<(), APIError> {
        // Validate if otp and new password is provided
        if self.otp.is_none() {
            return Err(APIError::Validation {
                message: "otp value is not set".to_string(),
                details: serde_json::Value::String("otp field is missing".to_string()),
            });
        }

        if self.new_pwd.is_none() {
            return Err(APIError::Validation {
                message: "password value is not set".to_string(),
                details: serde_json::Value::String("password field is missing".to_string()),
            });
        }

        Ok(())
    }
}

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{collection}/login", post(login))
        .route("/{collection}/forget-password", post(forget_password))
        .route("/{collection}/reset-password", post(reset_password))
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
) -> Result<Json<Value>, APIError> {
    state
        .auth_service()
        .logout_session(&collection, &payload.email, &payload.refresh_token)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn forget_password(
    Path(collection): Path<String>,
    state: State<AppState>,
    Json(payload): Json<PasswordResetRequest>,
) -> Result<Json<Value>, APIError> {
    state
        .auth_service()
        .send_password_reset_email(&collection, &payload.email)
        .await?;

    Ok(Json(
        serde_json::json!({ "detail": "Password reset link sent to your email adddress."}),
    ))
}

async fn reset_password(
    Path(collection): Path<String>,
    state: State<AppState>,
    Json(payload): Json<PasswordResetRequest>,
) -> Result<Json<Value>, APIError> {
    payload.validate()?;

    state
        .auth_service()
        .reset_password(
            &collection,
            &payload.email.as_str(),
            payload.otp.unwrap(),
            &payload.new_pwd.unwrap(),
        )
        .await?;
    Ok(Json(
        serde_json::json!({ "detail": "Password reset successfully."}),
    ))
}
