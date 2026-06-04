use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
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
    token: String,
}

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{collection}/login", post(login))
        .route("/profile", get(profile))
        .with_state(state)
}

async fn login(
    Path(collection): Path<String>,
    state: State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, APIError> {
    let token = state
        .auth_service()
        .authenticate(&collection, &body.email, &body.password)
        .await?;

    Ok(Json(LoginResponse { token }))
}

async fn profile(AuthenticatedUser(user): AuthenticatedUser) -> Result<Json<Value>, APIError> {
    Ok(Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
    })))
}
