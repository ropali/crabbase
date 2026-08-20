use crabbase_core::enums;
use crabbase_core::errors::RepositoryError;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MailSettings {
    pub sender_name: String,
    pub sender_address: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
}

impl Default for MailSettings {
    fn default() -> Self {
        Self {
            sender_name: "Crabbase Support".to_string(),
            sender_address: "support@example.com".to_string(),
            smtp_host: "".to_string(),
            smtp_port: 587,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub app_name: String,
    pub app_url: String,
    pub contact_email: String,
    pub allow_public_user_registration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailTemplate {
    pub subject: String,
    #[serde(rename = "bodyHtml", alias = "body_html")]
    pub body_html: String,
    #[serde(rename = "bodyText", alias = "body_text")]
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

    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), RepositoryError> {
        let val_str = serde_json::to_string(value).map_err(|e| RepositoryError::Validation {
            message: format!("Failed to serialize setting value: {e}"),
            field: Some(key.to_string()),
        })?;

        sqlx::query(
            r#"
            INSERT INTO _settings (name, value, updated)
            VALUES ($1, $2, now())
            ON CONFLICT (name) DO UPDATE
            SET value = EXCLUDED.value, updated = now();
            "#,
        )
        .bind(key)
        .bind(val_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_mail_settings(&self) -> Result<MailSettings, RepositoryError> {
        let settings_opt: Option<MailSettings> = self.get("mail").await?;
        Ok(settings_opt.unwrap_or_default())
    }

    pub async fn set_mail_settings(&self, settings: &MailSettings) -> Result<(), RepositoryError> {
        self.set(&enums::SettingsType::Mail.to_string(), settings)
            .await?;

        Ok(())
    }

    pub async fn get_email_templates(&self) -> Result<Option<EmailTemplates>, RepositoryError> {
        let templates = self
            .get::<EmailTemplates>(&enums::SettingsType::EmailTemplates.to_string())
            .await?;

        Ok(templates)
    }

    pub async fn save_email_temaplates(
        &self,
        templ: &EmailTemplates,
    ) -> Result<(), RepositoryError> {
        self.set(&enums::SettingsType::EmailTemplates.to_string(), &templ)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    // ── Test infrastructure ───────────────────────────────────────────────────

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

    fn make_repo(pool: Pool<Postgres>) -> SettingsRepository {
        SettingsRepository::new(pool)
    }

    fn sample_mail_settings() -> MailSettings {
        MailSettings {
            sender_name: "Test Sender".to_string(),
            sender_address: "sender@example.com".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_username: "user@example.com".to_string(),
            smtp_password: "s3cr3t".to_string(),
        }
    }

    fn sample_email_templates() -> EmailTemplates {
        EmailTemplates {
            password_reset: EmailTemplate {
                subject: "Reset your password".to_string(),
                body_html: "<p>Click <a href=\"{{reset_link}}\">here</a></p>".to_string(),
                body_text: "Reset link: {{reset_link}}".to_string(),
            },
            user_registration: EmailTemplate {
                subject: "Welcome!".to_string(),
                body_html: "<p>Hi {{name}}, welcome aboard!</p>".to_string(),
                body_text: "Hi {{name}}, welcome aboard!".to_string(),
            },
        }
    }

    // ── Generic get / set ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_returns_none_for_missing_key() {
        let repo = make_repo(setup_pool("settings_get_missing").await);

        let result: Option<serde_json::Value> = repo.get("nonexistent_key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_roundtrip() {
        let repo = make_repo(setup_pool("settings_set_get_roundtrip").await);

        let value = serde_json::json!({"foo": "bar", "num": 42});
        repo.set("test_key", &value).await.unwrap();

        let fetched: Option<serde_json::Value> = repo.get("test_key").await.unwrap();
        assert_eq!(fetched.unwrap(), value);
    }

    #[tokio::test]
    async fn test_set_upserts_existing_key() {
        let repo = make_repo(setup_pool("settings_set_upsert").await);

        let v1 = serde_json::json!({"version": 1});
        let v2 = serde_json::json!({"version": 2});

        repo.set("upsert_key", &v1).await.unwrap();
        repo.set("upsert_key", &v2).await.unwrap();

        let fetched: Option<serde_json::Value> = repo.get("upsert_key").await.unwrap();
        assert_eq!(fetched.unwrap()["version"], 2);
    }

    #[tokio::test]
    async fn test_get_returns_error_on_corrupt_json() {
        let repo = make_repo(setup_pool("settings_corrupt_json").await);

        // Insert raw corrupt JSON directly into the table
        sqlx::query("INSERT INTO _settings (name, value) VALUES ($1, $2)")
            .bind("bad_key")
            .bind("not valid json {{{")
            .execute(&repo.pool)
            .await
            .unwrap();

        let result: Result<Option<serde_json::Value>, _> = repo.get("bad_key").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::QueryFailed { message, .. } => {
                assert!(message.contains("Failed to deserialize"));
            }
            e => panic!("unexpected error: {:?}", e),
        }
    }

    // ── Mail settings ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_mail_settings_returns_default_when_missing() {
        let repo = make_repo(setup_pool("settings_mail_default").await);

        // Delete the seeded mail setting if the migration added one
        sqlx::query("DELETE FROM _settings WHERE name = 'mail'")
            .execute(&repo.pool)
            .await
            .unwrap();

        let settings = repo.get_mail_settings().await.unwrap();
        assert_eq!(settings, MailSettings::default());
    }

    #[tokio::test]
    async fn test_set_and_get_mail_settings_roundtrip() {
        let repo = make_repo(setup_pool("settings_mail_roundtrip").await);

        let original = sample_mail_settings();
        repo.set_mail_settings(&original).await.unwrap();

        let fetched = repo.get_mail_settings().await.unwrap();
        assert_eq!(fetched, original);
    }

    #[tokio::test]
    async fn test_set_mail_settings_upserts() {
        let repo = make_repo(setup_pool("settings_mail_upsert").await);

        let first = sample_mail_settings();
        repo.set_mail_settings(&first).await.unwrap();

        let updated = MailSettings {
            sender_name: "Updated Sender".to_string(),
            smtp_port: 587,
            ..first.clone()
        };
        repo.set_mail_settings(&updated).await.unwrap();

        let fetched = repo.get_mail_settings().await.unwrap();
        assert_eq!(fetched.sender_name, "Updated Sender");
        assert_eq!(fetched.smtp_port, 587);
        // other fields unchanged
        assert_eq!(fetched.smtp_host, first.smtp_host);
    }

    #[tokio::test]
    async fn test_mail_settings_serialises_as_camel_case() {
        let repo = make_repo(setup_pool("settings_mail_camel_case").await);

        let settings = sample_mail_settings();
        repo.set_mail_settings(&settings).await.unwrap();

        // Read the raw JSON from the DB and check camelCase keys
        let row: (String,) = sqlx::query_as("SELECT value FROM _settings WHERE name = 'mail'")
            .fetch_one(&repo.pool)
            .await
            .unwrap();

        let json: serde_json::Value = serde_json::from_str(&row.0).unwrap();
        assert!(
            json.get("senderName").is_some(),
            "expected camelCase senderName"
        );
        assert!(json.get("senderAddress").is_some());
        assert!(json.get("smtpHost").is_some());
        assert!(json.get("smtpPort").is_some());
        assert!(json.get("smtpUsername").is_some());
        assert!(json.get("smtpPassword").is_some());
        // snake_case keys must NOT be present
        assert!(json.get("sender_name").is_none());
    }

    // ── Email templates ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_email_templates_returns_seeded_value() {
        // The migration seeds email_templates — verify they are loaded correctly
        let repo = make_repo(setup_pool("settings_tmpl_seeded").await);

        let templates = repo.get_email_templates().await.unwrap();
        assert!(templates.is_some(), "migration should seed email_templates");

        let t = templates.unwrap();
        assert!(!t.password_reset.subject.is_empty());
        assert!(!t.password_reset.body_html.is_empty());
        assert!(!t.password_reset.body_text.is_empty());
        assert!(!t.user_registration.subject.is_empty());
        assert!(!t.user_registration.body_html.is_empty());
        assert!(!t.user_registration.body_text.is_empty());
    }

    #[tokio::test]
    async fn test_get_email_templates_returns_none_when_missing() {
        let repo = make_repo(setup_pool("settings_tmpl_missing").await);

        sqlx::query("DELETE FROM _settings WHERE name = 'email_templates'")
            .execute(&repo.pool)
            .await
            .unwrap();

        let result = repo.get_email_templates().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_save_and_get_email_templates_roundtrip() {
        let repo = make_repo(setup_pool("settings_tmpl_roundtrip").await);

        let original = sample_email_templates();
        repo.save_email_temaplates(&original).await.unwrap();

        let fetched = repo.get_email_templates().await.unwrap().unwrap();
        assert_eq!(
            fetched.password_reset.subject,
            original.password_reset.subject
        );
        assert_eq!(
            fetched.password_reset.body_html,
            original.password_reset.body_html
        );
        assert_eq!(
            fetched.password_reset.body_text,
            original.password_reset.body_text
        );
        assert_eq!(
            fetched.user_registration.subject,
            original.user_registration.subject
        );
        assert_eq!(
            fetched.user_registration.body_html,
            original.user_registration.body_html
        );
        assert_eq!(
            fetched.user_registration.body_text,
            original.user_registration.body_text
        );
    }

    #[tokio::test]
    async fn test_save_email_templates_upserts() {
        let repo = make_repo(setup_pool("settings_tmpl_upsert").await);

        let v1 = sample_email_templates();
        repo.save_email_temaplates(&v1).await.unwrap();

        let v2 = EmailTemplates {
            password_reset: EmailTemplate {
                subject: "Updated subject".to_string(),
                body_html: "<p>Updated</p>".to_string(),
                body_text: "Updated".to_string(),
            },
            user_registration: v1.user_registration.clone(),
        };
        repo.save_email_temaplates(&v2).await.unwrap();

        let fetched = repo.get_email_templates().await.unwrap().unwrap();
        assert_eq!(fetched.password_reset.subject, "Updated subject");
        // user_registration unchanged
        assert_eq!(
            fetched.user_registration.subject,
            v1.user_registration.subject
        );
    }

    #[tokio::test]
    async fn test_email_templates_serialises_as_camel_case() {
        let repo = make_repo(setup_pool("settings_tmpl_camel_case").await);

        repo.save_email_temaplates(&sample_email_templates())
            .await
            .unwrap();

        let row: (String,) =
            sqlx::query_as("SELECT value FROM _settings WHERE name = 'email_templates'")
                .fetch_one(&repo.pool)
                .await
                .unwrap();

        let json: serde_json::Value = serde_json::from_str(&row.0).unwrap();
        let pr = &json["passwordReset"];
        assert!(pr.get("bodyHtml").is_some(), "expected camelCase bodyHtml");
        assert!(pr.get("bodyText").is_some(), "expected camelCase bodyText");
        // snake_case aliases must NOT appear in serialized output
        assert!(pr.get("body_html").is_none());
        assert!(pr.get("body_text").is_none());
    }

    #[tokio::test]
    async fn test_email_templates_deserialises_legacy_snake_case() {
        // Ensures the serde alias on body_html / body_text handles old DB rows
        let repo = make_repo(setup_pool("settings_tmpl_legacy_snake").await);

        let legacy_json = r#"{
            "passwordReset": {
                "subject": "Legacy reset",
                "body_html": "<p>old html</p>",
                "body_text": "old text"
            },
            "userRegistration": {
                "subject": "Legacy welcome",
                "body_html": "<p>old welcome html</p>",
                "body_text": "old welcome text"
            }
        }"#;

        sqlx::query(
            "INSERT INTO _settings (name, value) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind("email_templates")
        .bind(legacy_json)
        .execute(&repo.pool)
        .await
        .unwrap();

        let templates = repo.get_email_templates().await.unwrap().unwrap();
        assert_eq!(templates.password_reset.subject, "Legacy reset");
        assert_eq!(templates.password_reset.body_html, "<p>old html</p>");
        assert_eq!(templates.password_reset.body_text, "old text");
        assert_eq!(templates.user_registration.subject, "Legacy welcome");
        assert_eq!(
            templates.user_registration.body_html,
            "<p>old welcome html</p>"
        );
    }
}
