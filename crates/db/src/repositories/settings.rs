use crabbase_core::errors::RepositoryError;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MailSettings {
    pub sender_name: String,
    pub sender_address: String,
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub encryption: String,  // "TLS", "SSL", "NONE"
    pub auth_method: String, // "PLAIN", "LOGIN", "CRAM-MD5"
    pub timeout_seconds: u64,
}

impl Default for MailSettings {
    fn default() -> Self {
        Self {
            sender_name: "Crabbase Support".to_string(),
            sender_address: "support@example.com".to_string(),
            smtp_enabled: false,
            smtp_host: "".to_string(),
            smtp_port: 587,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            encryption: "TLS".to_string(),
            auth_method: "PLAIN".to_string(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailTemplate {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmailTemplates {
    pub password_reset: EmailTemplate,
    pub user_registration: EmailTemplate,
}

pub struct SettingsRepository {
    pool: Pool<Postgres>,
}

impl SettingsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
    ) -> Result<Option<T>, RepositoryError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM _settings WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some((val_str,)) => {
                let parsed: T =
                    serde_json::from_str(&val_str).map_err(|e| RepositoryError::QueryFailed {
                        message: "Failed to deserialize setting value".to_string(),
                        source: Some(e.to_string()),
                    })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn set_param<T: Serialize>(
        &self,
        id: &str,
        value: &T,
    ) -> Result<(), RepositoryError> {
        let val_str = serde_json::to_string(value).map_err(|e| RepositoryError::Validation {
            message: format!("Failed to serialize setting value: {e}"),
            field: Some(id.to_string()),
        })?;

        sqlx::query(
            r#"
            INSERT INTO _settings (id, name, value, updated)
            VALUES ($1, $1, $2, now())
            ON CONFLICT (id) DO UPDATE
            SET value = EXCLUDED.value, updated = now();
            "#,
        )
        .bind(id)
        .bind(val_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_mail_settings(&self) -> Result<MailSettings, RepositoryError> {
        let settings_opt: Option<MailSettings> = self.get("mail_settings").await?;
        Ok(settings_opt.unwrap_or_default())
    }

    pub async fn save_mail_settings(&self, settings: &MailSettings) -> Result<(), RepositoryError> {
        self.set_param("mail_settings", settings).await
    }

    pub fn default_email_templates() -> HashMap<String, EmailTemplate> {
        let mut map = HashMap::new();

        map.insert(
            "password_reset".to_string(),
            EmailTemplate {
                key: "password_reset".to_string(),
                name: "Password Reset".to_string(),
                subject: "Reset your password - {app_name}".to_string(),
                body_html: r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>body{font-family:sans-serif;background-color:#f4f4f5;margin:0;padding:40px 0;}.card{max-width:560px;margin:0 auto;background:#ffffff;padding:32px;border-radius:12px;border:1px solid #e4e4e7;}.button{display:inline-block;background-color:#2563eb;color:#ffffff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;margin-top:16px;}</style></head><body><div class="card"><h2>Password Reset Request</h2><p>Hello,</p><p>We received a request to reset your password for <strong>{app_name}</strong> ({user_email}). Click the button below to proceed:</p><a href="{action_url}" class="button">Reset Password</a><p style="margin-top:24px;font-size:14px;color:#71717a;">If you did not request this email, please ignore it.</p></div></body></html>"#.to_string(),
                body_text: "Hello,\n\nWe received a request to reset your password for {app_name} ({user_email}). Please visit the following link to reset your password:\n\n{action_url}\n\nIf you did not request this, please ignore this email.".to_string(),
            },
        );

        map.insert(
            "user_registration".to_string(),
            EmailTemplate {
                key: "user_registration".to_string(),
                name: "User Registration & Verification".to_string(),
                subject: "Welcome to {app_name}! Confirm your email".to_string(),
                body_html: r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>body{font-family:sans-serif;background-color:#f4f4f5;margin:0;padding:40px 0;}.card{max-width:560px;margin:0 auto;background:#ffffff;padding:32px;border-radius:12px;border:1px solid #e4e4e7;}.button{display:inline-block;background-color:#16a34a;color:#ffffff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;margin-top:16px;}</style></head><body><div class="card"><h2>Welcome to {app_name}!</h2><p>Hello,</p><p>Thank you for registering. Please confirm your email address ({user_email}) by clicking the button below:</p><a href="{action_url}" class="button">Confirm Email Address</a><p style="margin-top:24px;font-size:14px;color:#71717a;">If you did not create an account, no further action is required.</p></div></body></html>"#.to_string(),
                body_text: "Hello,\n\nWelcome to {app_name}! Please confirm your email address ({user_email}) by visiting the link below:\n\n{action_url}\n\nIf you did not create an account, no further action is required.".to_string(),
            },
        );

        map.insert(
            "email_change".to_string(),
            EmailTemplate {
                key: "email_change".to_string(),
                name: "Email Address Change".to_string(),
                subject: "Confirm your new email address - {app_name}".to_string(),
                body_html: r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>body{font-family:sans-serif;background-color:#f4f4f5;margin:0;padding:40px 0;}.card{max-width:560px;margin:0 auto;background:#ffffff;padding:32px;border-radius:12px;border:1px solid #e4e4e7;}.button{display:inline-block;background-color:#9333ea;color:#ffffff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;margin-top:16px;}</style></head><body><div class="card"><h2>Confirm New Email Address</h2><p>Hello,</p><p>You requested to change your email address to <strong>{user_email}</strong> on <strong>{app_name}</strong>. Please confirm this change below:</p><a href="{action_url}" class="button">Confirm New Email</a><p style="margin-top:24px;font-size:14px;color:#71717a;">If you did not initiate this change, please contact support immediately.</p></div></body></html>"#.to_string(),
                body_text: "Hello,\n\nYou requested to change your email address to {user_email} on {app_name}. Please confirm this change by visiting:\n\n{action_url}\n\nIf you did not request this, please contact support immediately.".to_string(),
            },
        );

        map
    }

    pub async fn get_email_templates(
        &self,
    ) -> Result<HashMap<String, EmailTemplate>, RepositoryError> {
        let templates_opt: Option<HashMap<String, EmailTemplate>> =
            self.get("email_templates").await?;

        let mut templates = templates_opt.unwrap_or_default();
        let defaults = Self::default_email_templates();

        // Merge defaults if any template is missing
        let mut updated = false;
        for (k, v) in defaults {
            if !templates.contains_key(&k) {
                templates.insert(k, v);
                updated = true;
            }
        }

        if updated {
            let _ = self.set_param("email_templates", &templates).await;
        }

        Ok(templates)
    }

    pub async fn get_email_template(
        &self,
        key: &str,
    ) -> Result<Option<EmailTemplate>, RepositoryError> {
        let templates = self.get_email_templates().await?;
        Ok(templates.get(key).cloned())
    }

    pub async fn save_email_template(
        &self,
        template: &EmailTemplate,
    ) -> Result<(), RepositoryError> {
        let mut templates = self.get_email_templates().await?;
        templates.insert(template.key.clone(), template.clone());
        self.set_param("email_templates", &templates).await
    }

    pub async fn reset_email_template(&self, key: &str) -> Result<EmailTemplate, RepositoryError> {
        let defaults = Self::default_email_templates();
        let default_tmpl = defaults.get(key).cloned().ok_or_else(|| {
            RepositoryError::NotFound(format!("Email template '{key}' not found"))
        })?;

        self.save_email_template(&default_tmpl).await?;
        Ok(default_tmpl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn setup_pool(schema: &str) -> Pool<Postgres> {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

        let init_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .unwrap();

        let schema_ident = format!("\"{}\"", schema);
        let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE;", schema_ident))
            .execute(&init_pool)
            .await;

        sqlx::query(&format!("CREATE SCHEMA {};", schema_ident))
            .execute(&init_pool)
            .await
            .unwrap();

        init_pool.close().await;

        let mut options: sqlx::postgres::PgConnectOptions = db_url.parse().unwrap();
        options = options.options([("search_path", schema)]);

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_mail_settings_lifecycle() {
        let pool = setup_pool("db_params_mail_settings").await;
        let repo = SettingsRepository::new(pool);

        let default_settings = repo.get_mail_settings().await.unwrap();
        assert_eq!(default_settings, MailSettings::default());

        let new_settings = MailSettings {
            sender_name: "Custom Support".to_string(),
            sender_address: "custom@example.com".to_string(),
            smtp_enabled: true,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_username: "user".to_string(),
            smtp_password: "password".to_string(),
            encryption: "SSL".to_string(),
            auth_method: "LOGIN".to_string(),
            timeout_seconds: 60,
        };

        repo.save_mail_settings(&new_settings).await.unwrap();

        let loaded_settings = repo.get_mail_settings().await.unwrap();
        assert_eq!(loaded_settings, new_settings);
    }

    #[tokio::test]
    async fn test_email_template_lifecycle() {
        let pool = setup_pool("db_params_email_template").await;
        let repo = SettingsRepository::new(pool);

        let templates = repo.get_email_templates().await.unwrap();
        assert!(templates.contains_key("password_reset"));

        let mut tmpl = repo
            .get_email_template("password_reset")
            .await
            .unwrap()
            .unwrap();
        tmpl.subject = "New Subject".to_string();

        repo.save_email_template(&tmpl).await.unwrap();
        let loaded = repo
            .get_email_template("password_reset")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.subject, "New Subject");

        let reset = repo.reset_email_template("password_reset").await.unwrap();
        assert_eq!(reset.subject, "Reset your password - {app_name}");

        let not_found_err = repo
            .reset_email_template("nonexistent_key")
            .await
            .unwrap_err();
        match not_found_err {
            RepositoryError::NotFound(msg) => {
                assert!(msg.contains("nonexistent_key"));
            }
            other => panic!("Expected RepositoryError::NotFound, got {:?}", other),
        }
    }
}
