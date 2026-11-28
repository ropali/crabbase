use crate::middleware::auth::require_admin;
use crate::middleware::log::RequestLog;
use crate::state::AppState;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use crabbase_core::enums;
use crabbase_core::errors::APIError;
use crabbase_db::repositories::settings::{AppSettings, EmailTemplates, MailSettings};
use crabbase_files::storage::StorageConfig;
use serde_json::{Value, json};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/mail", get(get_mail_settings).post(save_mail_settings))
        .route("/app", get(get_app_setings).post(save_app_settings))
        .route(
            "/email-templates",
            get(get_email_templates).post(save_email_templates),
        )
        .route(
            "/storage",
            get(get_storage_settings).post(save_storage_settings),
        )
        .route(
            "/backups",
            get(get_backups_settings).post(save_backups_settings),
        )
        .route("/jobs", get(get_jobs_settings).post(save_jobs_settings))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_admin,
        ))
        .with_state(state)
}

/// Top-level keys that changed between the stored settings object and the new
/// payload. Values are deliberately not captured; only key names. Removed keys
/// are prefixed with `-`.
fn changed_keys(old: Option<&Value>, new: &Value) -> Vec<String> {
    let Some(new_obj) = new.as_object() else {
        return Vec::new();
    };
    let old_obj = old.and_then(Value::as_object);

    let mut keys: Vec<String> = new_obj
        .iter()
        .filter(|(key, value)| old_obj.and_then(|old| old.get(*key)) != Some(*value))
        .map(|(key, _)| key.clone())
        .collect();

    if let Some(old_obj) = old_obj {
        for key in old_obj.keys() {
            if !new_obj.contains_key(key) {
                keys.push(format!("-{key}"));
            }
        }
    }

    keys.sort();
    keys
}

async fn setting_changes(
    state: &AppState,
    group: &str,
    new: &Value,
) -> Result<Vec<String>, APIError> {
    let old: Option<Value> = state.settings_repo().get(group).await?;
    Ok(changed_keys(old.as_ref(), new))
}

fn attach_setting_change(log: Option<Extension<RequestLog>>, group: &str, changed: Vec<String>) {
    if let Some(Extension(log)) = log {
        log.add_data(json!({ "setting": group, "changedKeys": changed }));
    }
}

async fn get_mail_settings(state: State<AppState>) -> Result<Json<MailSettings>, APIError> {
    Ok(Json(state.settings_repo().get_mail_settings().await?))
}

async fn save_mail_settings(
    state: State<AppState>,
    log: Option<Extension<RequestLog>>,
    Json(payload): Json<MailSettings>,
) -> Result<Json<Value>, APIError> {
    let new_value = serde_json::to_value(&payload).unwrap_or(Value::Null);
    let changed = setting_changes(&state, "mail", &new_value).await?;
    state.settings_repo().set_mail_settings(&payload).await?;
    attach_setting_change(log, "mail", changed);

    Ok(Json(json!({"details": "Email settings saved"})))
}

async fn get_app_setings(state: State<AppState>) -> Result<impl IntoResponse, APIError> {
    let settings: Option<AppSettings> = state
        .settings_repo()
        .get::<AppSettings>(&enums::SettingsType::App.to_string())
        .await?;

    Ok(match settings {
        Some(s) => Json(s).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}

async fn save_app_settings(
    state: State<AppState>,
    log: Option<Extension<RequestLog>>,
    Json(payload): Json<AppSettings>,
) -> Result<Json<Value>, APIError> {
    let group = enums::SettingsType::App.to_string();
    let new_value = serde_json::to_value(&payload).unwrap_or(Value::Null);
    let changed = setting_changes(&state, &group, &new_value).await?;
    state.settings_repo().set(&group, &payload).await?;
    attach_setting_change(log, &group, changed);

    Ok(Json(json!({"details": "App settings saved."})))
}

async fn get_email_templates(state: State<AppState>) -> Result<Json<EmailTemplates>, APIError> {
    Ok(Json(
        state
            .settings_repo()
            .get_email_templates()
            .await?
            .ok_or(APIError::NotFound {
                resource: "email_templates".to_string(),
            })?,
    ))
}

async fn save_email_templates(
    state: State<AppState>,
    log: Option<Extension<RequestLog>>,
    Json(payload): Json<EmailTemplates>,
) -> Result<Json<Value>, APIError> {
    let group = enums::SettingsType::EmailTemplates.to_string();
    let new_value = serde_json::to_value(&payload).unwrap_or(Value::Null);
    let changed = setting_changes(&state, &group, &new_value).await?;
    state
        .settings_repo()
        .save_email_temaplates(&payload)
        .await?;
    attach_setting_change(log, &group, changed);

    Ok(Json(json!({"details": "Email templates saved."})))
}

async fn get_storage_settings(state: State<AppState>) -> Result<Json<StorageConfig>, APIError> {
    let settings = state
        .settings_repo()
        .get::<StorageConfig>("storage")
        .await?
        .ok_or(APIError::NotFound {
            resource: "storage".to_string(),
        })?;

    Ok(Json(settings))
}

async fn save_storage_settings(
    state: State<AppState>,
    log: Option<Extension<RequestLog>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, APIError> {
    let changed = setting_changes(&state, "storage", &payload).await?;
    state.settings_repo().set("storage", &payload).await?;
    attach_setting_change(log, "storage", changed);

    Ok(Json(json!({"details": "Storage settings saved."})))
}

async fn get_backups_settings(
    state: State<AppState>,
) -> Result<Json<crabbase_jobs::BackupSettings>, APIError> {
    Ok(Json(crabbase_jobs::load_backup_settings(&state.db).await?))
}

async fn save_backups_settings(
    state: State<AppState>,
    log: Option<Extension<RequestLog>>,
    Json(payload): Json<crabbase_jobs::BackupSettings>,
) -> Result<Json<Value>, APIError> {
    let new_value = serde_json::to_value(&payload).unwrap_or(Value::Null);
    let changed = setting_changes(&state, "backups", &new_value).await?;
    state.settings_repo().set("backups", &payload).await?;
    attach_setting_change(log, "backups", changed);

    Ok(Json(json!({"details": "Backup settings saved."})))
}

async fn get_jobs_settings(
    state: State<AppState>,
) -> Result<Json<crabbase_jobs::JobsSettings>, APIError> {
    Ok(Json(crabbase_jobs::load_jobs_settings(&state.db).await?))
}

async fn save_jobs_settings(
    state: State<AppState>,
    log: Option<Extension<RequestLog>>,
    Json(payload): Json<crabbase_jobs::JobsSettings>,
) -> Result<Json<Value>, APIError> {
    let new_value = serde_json::to_value(&payload).unwrap_or(Value::Null);
    let changed = setting_changes(&state, "jobs", &new_value).await?;
    state.settings_repo().set("jobs", &payload).await?;
    attach_setting_change(log, "jobs", changed);

    Ok(Json(json!({"details": "Job settings saved."})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_keys_reports_added_changed_and_removed() {
        let old = json!({"a": 1, "b": "x", "gone": true});
        let new = json!({"a": 1, "b": "y", "added": 2});
        assert_eq!(changed_keys(Some(&old), &new), vec!["-gone", "added", "b"]);
    }

    #[test]
    fn first_write_marks_every_key() {
        let new = json!({"a": 1, "b": 2});
        assert_eq!(changed_keys(None, &new), vec!["a", "b"]);
    }

    #[test]
    fn changed_keys_returns_names_not_values() {
        let old = json!({
            "smtpPassword": "old-secret",
            "smtpHost": "smtp.example.com"
        });
        let new = json!({
            "smtpPassword": "new-secret",
            "smtpHost": "smtp.example.com"
        });

        let keys = changed_keys(Some(&old), &new);

        assert_eq!(keys, vec!["smtpPassword"]);
        assert!(
            !keys
                .iter()
                .any(|key| key == "old-secret" || key == "new-secret")
        );
    }
}
