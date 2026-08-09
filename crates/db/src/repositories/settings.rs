use crabbase_core::enums;
use crabbase_core::errors::RepositoryError;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

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

    pub async fn set<T: Serialize>(&self, id: &str, value: &T) -> Result<(), RepositoryError> {
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
        let settings_opt: Option<MailSettings> = self.get("mail").await?;
        Ok(settings_opt.unwrap_or_default())
    }

    pub async fn get_email_templates(&self) -> Result<Option<EmailTemplates>, RepositoryError> {
        let templates = self
            .get::<EmailTemplates>(enums::SettingsType::EmailTemplates.to_string())
            .await?;

        Ok(templates)
    }

    // pub async fn get_email_template(
    //     &self,
    //     key: enums::EmailTemplateType,
    // ) -> Result<Option<EmailTemplate>, RepositoryError> {
    //     let templates = self.get_email_templates().await?;
    //     Ok(templates.get(key.to_string()).cloned())
    // }
    //
    // pub async fn save_email_template(
    //     &self,
    //     template: &EmailTemplate,
    // ) -> Result<(), RepositoryError> {
    //     let mut templates = self.get_email_templates().await?;
    //
    //     templates.insert(template.key.clone(), template.clone());
    //     self.set(enums::SettingsType::EmailTemplates.to_string(), &templates).await
    // }
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
}
