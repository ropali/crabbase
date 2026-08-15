use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use crabbase_core::enums;
use crabbase_core::errors::APIError;
use crabbase_db::repositories::settings::{AppSettings, MailSettings};
use serde_json::{Value, json};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/mail", get(get_mail_settings).post(save_mail_settings))
        .route("/app", get(get_app_setings).post(save_app_settings))
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

async fn get_app_setings(state: State<AppState>) -> Result<impl IntoResponse, APIError> {
    let settings: Option<AppSettings> = state
        .settings_repo()
        .get::<AppSettings>(enums::SettingsType::App.to_string())
        .await?;

    Ok(match settings {
        Some(s) => Json(s).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}

async fn save_app_settings(
    state: State<AppState>,
    Json(payload): Json<AppSettings>,
) -> Result<Json<Value>, APIError> {
    state
        .settings_repo()
        .set(enums::SettingsType::App.to_string(), &payload)
        .await?;

    Ok(Json(json!({"details": "App settings saved."})))
}
