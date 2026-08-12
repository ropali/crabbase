use crate::state::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use crabbase_core::errors::APIError;
use crabbase_db::repositories::settings::MailSettings;
use serde_json::{Value, json};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/mail", get(get_mail_settings).post(save_mail_settings))
        .with_state(state)
}

async fn get_mail_settings(state: State<AppState>) -> Result<Json<MailSettings>, APIError> {
    Ok(Json(state.settings_repo().get_mail_settings().await?))
}

async fn save_mail_settings(
    state: State<AppState>,
    Json(payload): Json<MailSettings>,
) -> Result<Json<Value>, APIError> {
    state.settings_repo().set_mail_settings(&payload).await?;

    Ok(Json(json!({"details": "Email settings saved"})))
}
