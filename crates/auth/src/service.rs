use std::collections::HashMap;

use crate::{
    auth::{Claims, TokenParams, TokenType, create_token, verify_password, verify_token},
    repositories::{auth::AuthRepository, otp::OtpRepository},
};
use chrono::Utc;
use crabbase_core::{enums, errors::APIError};
use crabbase_db::repositories::settings::{AppSettings, EmailTemplate, EmailTemplates};
use crabbase_db::repositories::{
    auth::{AuthUser, UserRepository},
    settings::{MailSettings, SettingsRepository},
};
use lettre::{
    Message, SmtpTransport, Transport,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::task;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthTokens {
    #[serde(rename = "accessToken")]
    pub access_token: String,

    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

pub struct AuthService {
    user_repo: UserRepository,
    auth_repo: AuthRepository,
    settings_repo: SettingsRepository,
    otp_repo: OtpRepository,
}

impl AuthService {
    pub fn new(
        user_repo: UserRepository,
        auth_repo: AuthRepository,
        settings_repo: SettingsRepository,
        otp_repo: OtpRepository,
    ) -> Self {
        Self {
            user_repo,
            auth_repo,
            settings_repo,
            otp_repo,
        }
    }

    // Verifies auth user session from claims
    pub async fn verify_session(&self, claims: &Claims) -> Result<AuthUser, APIError> {
        let collection_name = self
            .user_repo
            .get_collection_by_id(&claims.collection_id)
            .await
            .map_err(|e| APIError::Internal {
                message: "Database query failed".to_string(),
                details: serde_json::json!(e.to_string()),
            })?
            .map(|col| col.name)
            .unwrap_or_else(|| claims.collection_id.clone());

        let user_opt = if collection_name == "_superusers" || collection_name == "admin" {
            self.user_repo
                .get_superuser_by_id(&claims.id)
                .await
                .map_err(|e| APIError::Internal {
                    message: "Database query failed".to_string(),
                    details: serde_json::json!(e.to_string()),
                })?
        } else {
            self.user_repo
                .get_user_by_id(&collection_name, &claims.id)
                .await
                .map_err(|e| APIError::Internal {
                    message: "Database query failed".to_string(),
                    details: serde_json::json!(e.to_string()),
                })?
        };

        match user_opt {
            Some(user) => {
                if !user.verified {
                    return Err(APIError::Forbidden);
                }

                Ok(user)
            }
            None => Err(APIError::Unauthorized),
        }
    }

    pub async fn authenticate(
        &self,
        collection: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthTokens, APIError> {
        let col = match self.user_repo.get_collection_by_name(collection).await? {
            Some(id) => id,
            None => {
                return Err(APIError::NotFound {
                    resource: collection.to_string(),
                });
            }
        };

        let user_opt = self.user_repo.get_user_by_email(collection, email).await?;

        let user = user_opt.ok_or(APIError::NotFound {
            resource: email.to_string(),
        })?;

        let is_valid =
            verify_password(password, &user.password).map_err(|_| APIError::Unauthorized)?;

        if !is_valid {
            return Err(APIError::Unauthorized);
        }

        let col_token = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("secret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| APIError::Internal {
                message: "Unable to find the collection auth token".to_string(),
                details: serde_json::Value::String(format!("Collection name is: {}", col.name)),
            })?;

        let secret = format!("{}-{}", col_token, user.token_key);

        let duration: Option<usize> = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("duration"))
            .and_then(|v| v.as_number())
            .and_then(|n| n.as_u64())
            .and_then(|num| num.try_into().ok());

        let access_token_params = TokenParams {
            user_id: &user.id,
            collection_id: &col.id,
            collection_name: &col.name,
            secret: &secret,
            token_type: TokenType::Auth,
            duration: duration,
            jti: None,
            family_id: None,
        };

        let access_token = create_token(access_token_params).map_err(|_| APIError::Unauthorized)?;

        // Create Refresh Token

        // TODO: remove the hardcoded duration

        let refresh_duration = 604800; // 7 days in seconds
        let family_id = uuid::Uuid::new_v4();
        let jti = uuid::Uuid::new_v4().to_string();

        let refresh_token_params = TokenParams {
            user_id: &user.id,
            collection_id: &col.id,
            collection_name: &col.name,
            secret: &secret,
            token_type: TokenType::Refresh,
            duration: Some(refresh_duration),
            jti: Some(jti.clone()),
            family_id: Some(family_id),
        };

        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(refresh_duration as i64);

        let refresh_token =
            create_token(refresh_token_params).map_err(|_| APIError::Unauthorized)?;

        // Record initial refresh token in db
        self.auth_repo
            .create_refresh_token(family_id, &collection, &user.id, &jti, None, expires_at)
            .await?;

        let tokens = AuthTokens {
            access_token: access_token,
            refresh_token: refresh_token,
        };

        Ok(tokens)
    }

    pub async fn refresh_token(
        &self,
        collection: &str,
        email: &str,
        refresh_token: &str,
    ) -> Result<AuthTokens, APIError> {
        let user = self
            .user_repo
            .get_user_by_email(collection, email)
            .await?
            .unwrap(); //  User will always be present as already authencated

        let col = match self.user_repo.get_collection_by_name(collection).await? {
            Some(id) => id,
            None => {
                return Err(APIError::NotFound {
                    resource: collection.to_string(),
                });
            }
        };

        let col_token = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("secret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| APIError::Internal {
                message: "Unable to find the collection auth token".to_string(),
                details: serde_json::Value::String(format!("Collection name is: {}", col.name)),
            })?;

        let secret = format!("{}-{}", col_token, user.token_key);

        let claims = verify_token(refresh_token, &secret).map_err(|_| APIError::Unauthorized)?;

        if claims.token_type != "refresh" {
            return Err(APIError::Unauthorized);
        }

        let old_jti = claims.jti.as_ref().ok_or(APIError::Unauthorized)?;
        let family_id_str = claims.family_id.as_ref().ok_or(APIError::Unauthorized)?;
        let family_id = uuid::Uuid::parse_str(family_id_str).map_err(|_| APIError::Unauthorized)?;

        let new_jti = uuid::Uuid::new_v4().to_string();
        let refresh_duration = 604800; // 7 days
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(refresh_duration as i64);

        // Consume old token and insert new token
        let new_record_opt = self
            .auth_repo
            .rotate_refreh_token(old_jti, &new_jti, expires_at)
            .await?;

        if let Some(new_record) = new_record_opt {
            // Successful rotation
            let duration: Option<usize> = col
                .options
                .auth_token
                .as_ref()
                .and_then(|t| t.get("duration"))
                .and_then(|v| v.as_number())
                .and_then(|n| n.as_u64())
                .and_then(|num| num.try_into().ok());

            let access_token_params = TokenParams {
                user_id: &user.id,
                collection_id: &col.id,
                collection_name: &col.name,
                secret: &secret,
                token_type: TokenType::Auth,
                duration: duration,
                jti: None,
                family_id: None,
            };

            let access_token =
                create_token(access_token_params).map_err(|_| APIError::Unauthorized)?;

            let refresh_token_params = TokenParams {
                user_id: &user.id,
                collection_id: &col.id,
                collection_name: &col.name,
                secret: &secret,
                token_type: TokenType::Refresh,
                duration: Some(refresh_duration),
                jti: Some(new_jti),
                family_id: Some(new_record.family_id),
            };

            let refresh_token =
                create_token(refresh_token_params).map_err(|_| APIError::Unauthorized)?;

            return Ok(AuthTokens {
                access_token,
                refresh_token,
            });
        }

        // Check if token already used, revoked, or non existent
        let existing_token = self
            .auth_repo
            .get_refresh_token_by_jti(old_jti)
            .await?
            .ok_or(APIError::Unauthorized)?;

        if existing_token.revoked {
            return Err(APIError::Unauthorized);
        }

        // check for grace period for concurrent refresh
        if let Some(used_at) = existing_token.used_at {
            let elapsed = chrono::Utc::now().signed_duration_since(used_at);

            if elapsed <= chrono::Duration::seconds(10) {
                if let Some(child_record) = self
                    .auth_repo
                    .get_child_refresh_token(existing_token.id)
                    .await?
                {
                    let duration: Option<usize> = col
                        .options
                        .auth_token
                        .as_ref()
                        .and_then(|t| t.get("duration"))
                        .and_then(|v| v.as_number())
                        .and_then(|n| n.as_u64())
                        .and_then(|num| num.try_into().ok());

                    let access_token_params = TokenParams {
                        user_id: &user.id,
                        collection_id: &col.id,
                        collection_name: &col.name,
                        secret: &secret,
                        token_type: TokenType::Auth,
                        duration: duration,
                        jti: None,
                        family_id: None,
                    };

                    let access_token =
                        create_token(access_token_params).map_err(|_| APIError::Unauthorized)?;

                    let new_jti_grace = uuid::Uuid::new_v4().to_string();
                    let refresh_token_params = TokenParams {
                        user_id: &user.id,
                        collection_id: &col.id,
                        collection_name: &col.name,
                        secret: &secret,
                        token_type: TokenType::Refresh,
                        duration: Some(refresh_duration),
                        jti: Some(new_jti_grace),
                        family_id: Some(child_record.family_id),
                    };
                    let refresh_token =
                        create_token(refresh_token_params).map_err(|_| APIError::Unauthorized)?;

                    return Ok(AuthTokens {
                        access_token,
                        refresh_token,
                    });
                }
            }
        }

        // Reuse detected
        self.auth_repo.revoke_token_family(family_id).await?;

        tracing::warn!(user_id = %user.id, family_id = %family_id, "Refresh token reuse detected. Revoking family session.");

        Err(APIError::Unauthorized)
    }

    pub async fn logout_session(
        &self,
        collection: &str,
        email: &str,
        refresh_token: &str,
    ) -> Result<(), APIError> {
        let user = self
            .user_repo
            .get_user_by_email(collection, email)
            .await?
            .ok_or(APIError::NotFound {
                resource: email.to_string(),
            })?;

        let col = self
            .user_repo
            .get_collection_by_name(collection)
            .await?
            .ok_or(APIError::NotFound {
                resource: collection.to_string(),
            })?;

        let col_token = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("secret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| APIError::Internal {
                message: "Unable to find collection auth token".to_string(),
                details: serde_json::Value::String(format!("Collection: {}", col.name)),
            })?;

        let key = format!("{}-{}", col_token, user.token_key);
        let claims = verify_token(refresh_token, &key).map_err(|_| APIError::Unauthorized)?;

        if let Some(family_id_str) = claims.family_id {
            if let Ok(family_id) = uuid::Uuid::parse_str(&family_id_str) {
                self.auth_repo.revoke_token_family(family_id).await?;
            }
        }

        Ok(())
    }

    pub async fn send_password_reset_email(
        &self,
        collection: &str,
        email: &str,
    ) -> Result<(), APIError> {
        // check if user exist
        let user_opt = self.user_repo.get_user_by_email(collection, email).await?;

        if user_opt.is_none() {
            info!(
                "Password reset triggered for non-existent user with email {}",
                email
            );
            // Do not say user exist or not to prevent guessing the user email
            return Ok(());
        }

        let user = user_opt.unwrap();

        let app_settings = self
            .settings_repo
            .get::<AppSettings>(&enums::SettingsType::App.to_string())
            .await?
            .ok_or(APIError::NotFound {
                resource: "App settings".to_string(),
            })?;

        let email_setting = self
            .settings_repo
            .get::<MailSettings>(&enums::SettingsType::Mail.to_string())
            .await?
            .ok_or(APIError::NotFound {
                resource: "email SMTP settings".to_string(),
            })?;

        let mut templates = self
            .settings_repo
            .get::<EmailTemplates>(&enums::SettingsType::EmailTemplates.to_string())
            .await?
            .ok_or(APIError::NotFound {
                resource: "Password reset email template".to_string(),
            })?;

        let receiver_name = email.split_once("@").map(|(u, _)| u).unwrap_or(email);

        // Scope rng tightly so it is dropped before any await — ThreadRng is !Send
        let otp = self.otp_repo.generate_otp(&collection, &user).await?;

        // Prepare the email template variables replacement
        let otp_str = otp.to_string();
        let vars = HashMap::from([
            ("name", receiver_name),
            ("app_name", app_settings.app_name.as_str()),
            ("email", user.email.as_str()),
            ("link", app_settings.app_url.as_str()),
            ("otp", otp_str.as_str()),
        ]);

        let pwd_reset_tmpl = templates.password_reset.render(&vars);

        let email_msg = Message::builder()
            .from(Mailbox::new(
                Some(email_setting.sender_name.parse().unwrap()),
                email_setting.sender_address.parse().unwrap(),
            ))
            .to(Mailbox::new(
                Some(receiver_name.to_owned()),
                email.parse().unwrap(),
            ))
            .subject(templates.password_reset.subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(String::from(pwd_reset_tmpl.body_text)), // Every message should have a plain text fallback.
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(String::from(pwd_reset_tmpl.body_html)),
                    ),
            )
            .map_err(|e| APIError::Internal {
                message: "Failed to build email message".to_string(),
                details: serde_json::json!(format!("Failed to build email message: {}", e)),
            })?;

        let creds = Credentials::new(email_setting.smtp_username, email_setting.smtp_password);

        // Send the email
        task::spawn_blocking(move || {
            // Open a remote connection
            let mailer = if email_setting.smtp_port == 465 {
                // Implicit TLS
                SmtpTransport::relay(&*email_setting.smtp_host)
                    .unwrap()
                    .credentials(creds)
                    .build()
            } else {
                // STARTTLS (port 587 or 25)
                SmtpTransport::starttls_relay(&*email_setting.smtp_host)
                    .unwrap()
                    .port(email_setting.smtp_port)
                    .credentials(creds)
                    .build()
            };

            if let Err(e) = mailer.send(&email_msg) {
                tracing::error!("Failed to send email: {}", e);
            }
        });

        Ok(())
    }

    pub async fn reset_password(
        &self,
        collection: &str,
        email: &str,
        otp: u32,
        new_pwd: &str,
    ) -> Result<(), APIError> {
        let user_opt = self.user_repo.get_user_by_email(collection, email).await?;

        if user_opt.is_none() {
            info!(
                "Password reset attempted for non-existent user with email {}",
                email
            );
            // Do not say user exist or not to prevent guessing the user email
            return Ok(());
        }

        let user = user_opt.unwrap();

        let otp_record = self.otp_repo.get_otp(otp, &user).await?;

        let record = otp_record.ok_or_else(|| APIError::Validation {
            message: "Invalid OTP".to_string(),
            details: serde_json::Value::String("OTP maybe incorrent or expired".to_string()),
        })?;

        self.user_repo
            .update_password(&record.collection_ref, &email, &new_pwd)
            .await?;

        self.otp_repo.delete(record.id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Claims, hash_password, verify_token};
    use crabbase_core::errors::APIError;
    use sqlx::postgres::PgPoolOptions;

    async fn setup_service(schema: &str) -> (AuthService, sqlx::Pool<sqlx::Postgres>) {
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

        // Clean seeded collections to keep tests deterministic
        sqlx::query("DELETE FROM _collections;")
            .execute(&pool)
            .await
            .unwrap();

        let repo = UserRepository::new(pool.clone());
        let auth_repo = AuthRepository::new(pool.clone());
        let settings_repo = SettingsRepository::new(pool.clone());
        let service = AuthService::new(repo, auth_repo, settings_repo);
        (service, pool)
    }

    #[tokio::test]
    async fn test_verify_session_superuser() {
        let (service, pool) = setup_service("auth_verify_session_superuser").await;

        let admin_uuid_1 = uuid::Uuid::parse_str("936da01f-9abd-4d9d-80c7-02af85c822a8").unwrap();
        let admin_uuid_2 = uuid::Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();

        // 1. Setup a verified superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(admin_uuid_1)
        .bind("admin1@example.com")
        .bind("hash")
        .bind("token")
        .bind(true) // verified
        .execute(&pool)
        .await
        .unwrap();

        // 2. Setup an unverified superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(admin_uuid_2)
        .bind("admin2@example.com")
        .bind("hash")
        .bind("token")
        .bind(false) // unverified
        .execute(&pool)
        .await
        .unwrap();

        // Test verified superuser session verification
        let claims_verified = Claims {
            token_type: "auth".to_string(),
            id: "936da01f-9abd-4d9d-80c7-02af85c822a8".to_string(),
            collection_id: "_superusers".to_string(),
            collection_name: "_superusers".to_string(),
            refreshable: false,
            sub: "936da01f-9abd-4d9d-80c7-02af85c822a8".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let user = service.verify_session(&claims_verified).await.unwrap();
        assert_eq!(user.id, "936da01f-9abd-4d9d-80c7-02af85c822a8");
        assert_eq!(user.email, "admin1@example.com");
        assert!(user.verified);

        // Test with collection_id as "admin"
        let claims_admin = Claims {
            token_type: "auth".to_string(),
            id: "936da01f-9abd-4d9d-80c7-02af85c822a8".to_string(),
            collection_id: "admin".to_string(),
            collection_name: "_superusers".to_string(),
            refreshable: false,
            sub: "936da01f-9abd-4d9d-80c7-02af85c822a8".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let user_admin = service.verify_session(&claims_admin).await.unwrap();
        assert_eq!(user_admin.id, "936da01f-9abd-4d9d-80c7-02af85c822a8");

        // Test unverified superuser session verification
        let claims_unverified = Claims {
            token_type: "auth".to_string(),
            id: "f47ac10b-58cc-4372-a567-0e02b2c3d479".to_string(),
            collection_id: "_superusers".to_string(),
            collection_name: "_superusers".to_string(),
            refreshable: false,
            sub: "f47ac10b-58cc-4372-a567-0e02b2c3d479".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let err_forbidden = service
            .verify_session(&claims_unverified)
            .await
            .unwrap_err();
        assert!(matches!(err_forbidden, APIError::Forbidden));

        // Test non-existent superuser session verification
        let claims_nonexistent = Claims {
            token_type: "auth".to_string(),
            id: "ba8f95c5-cc1a-4fa6-a70e-f0bcfd96c9e0".to_string(),
            collection_id: "_superusers".to_string(),
            collection_name: "_superusers".to_string(),
            refreshable: false,
            sub: "ba8f95c5-cc1a-4fa6-a70e-f0bcfd96c9e0".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let err_unauthorized = service
            .verify_session(&claims_nonexistent)
            .await
            .unwrap_err();
        assert!(matches!(err_unauthorized, APIError::Unauthorized));
    }

    async fn create_users_table(pool: &sqlx::Pool<sqlx::Postgres>) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id             TEXT PRIMARY KEY NOT NULL,
                email          TEXT UNIQUE NOT NULL,
                password       TEXT NOT NULL,
                token_key      TEXT NOT NULL,
                email_visible  BOOLEAN NOT NULL DEFAULT FALSE,
                verified       BOOLEAN NOT NULL DEFAULT FALSE,
                created        TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated        TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_verify_session_regular_user() {
        let (service, pool) = setup_service("auth_verify_session_regular_user").await;
        create_users_table(&pool).await;

        // 1. Setup a verified user
        sqlx::query(
            "INSERT INTO users (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_1")
        .bind("user1@example.com")
        .bind("hash")
        .bind("token")
        .bind(true) // verified
        .execute(&pool)
        .await
        .unwrap();

        // 2. Setup an unverified user
        sqlx::query(
            "INSERT INTO users (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_2")
        .bind("user2@example.com")
        .bind("hash")
        .bind("token")
        .bind(false) // unverified
        .execute(&pool)
        .await
        .unwrap();

        // Test verified user session verification
        let claims_verified = Claims {
            token_type: "auth".to_string(),
            id: "user_id_1".to_string(),
            collection_id: "users".to_string(),
            collection_name: "users".to_string(),
            refreshable: false,
            sub: "user_id_1".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let user = service.verify_session(&claims_verified).await.unwrap();
        assert_eq!(user.id, "user_id_1");
        assert_eq!(user.email, "user1@example.com");
        assert!(user.verified);

        // Test unverified user session verification
        let claims_unverified = Claims {
            token_type: "auth".to_string(),
            id: "user_id_2".to_string(),
            collection_id: "users".to_string(),
            collection_name: "users".to_string(),
            refreshable: false,
            sub: "user_id_2".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let err_forbidden = service
            .verify_session(&claims_unverified)
            .await
            .unwrap_err();
        assert!(matches!(err_forbidden, APIError::Forbidden));

        // Test non-existent user session verification
        let claims_nonexistent = Claims {
            token_type: "auth".to_string(),
            id: "nonexistent_user".to_string(),
            collection_id: "users".to_string(),
            collection_name: "users".to_string(),
            refreshable: false,
            sub: "nonexistent_user".to_string(),
            exp: 0,
            iat: 0,
            jti: None,
            family_id: None,
        };
        let err_unauthorized = service
            .verify_session(&claims_nonexistent)
            .await
            .unwrap_err();
        assert!(matches!(err_unauthorized, APIError::Unauthorized));
    }

    #[tokio::test]
    async fn test_authenticate_superuser() {
        let (service, pool) = setup_service("auth_authenticate_superuser").await;

        let password = "admin_secure_password";
        let hash = hash_password(password).unwrap();

        let admin_uuid_1 = uuid::Uuid::parse_str("936da01f-9abd-4d9d-80c7-02af85c822a8").unwrap();

        // Setup a superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(admin_uuid_1)
        .bind("admin@example.com")
        .bind(hash)
        .bind("token")
        .bind(true)
        .execute(&pool)
        .await
        .unwrap();

        // 1. Test login fail - _superusers collection not in _collections
        let err_not_found_col = service
            .authenticate("_superusers", "admin@example.com", password)
            .await
            .unwrap_err();
        assert!(
            matches!(err_not_found_col, APIError::NotFound { ref resource } if resource == "_superusers")
        );

        // Setup "_superusers" in _collections table so collection ID can be queried
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
        )
        .bind("admin_col_id")
        .bind(1)
        .bind("auth")
        .bind("_superusers")
        .bind("[]")
        .bind("{\"authToken\": {\"secret\": \"super-secret-key\"}}")
        .execute(&pool)
        .await
        .unwrap();

        // 2. Test login success (returns token)
        let tokens = service
            .authenticate("_superusers", "admin@example.com", password)
            .await
            .unwrap();
        let claims = verify_token(
            &tokens.access_token,
            &format!("{}-{}", "super-secret-key", "token"),
        )
        .unwrap();
        assert_eq!(claims.id, "936da01f-9abd-4d9d-80c7-02af85c822a8");
        assert_eq!(claims.collection_id, "admin_col_id");

        // 3. Test login fail - wrong password
        let err_unauthorized = service
            .authenticate("_superusers", "admin@example.com", "wrong_pass")
            .await
            .unwrap_err();
        assert!(matches!(err_unauthorized, APIError::Unauthorized));

        // 4. Test login fail - non-existent superuser
        let err_not_found = service
            .authenticate("_superusers", "nonexistent@example.com", password)
            .await
            .unwrap_err();
        assert!(
            matches!(err_not_found, APIError::NotFound { ref resource } if resource == "nonexistent@example.com")
        );
    }

    #[tokio::test]
    async fn test_authenticate_regular_user() {
        let (service, pool) = setup_service("auth_authenticate_regular_user").await;
        create_users_table(&pool).await;

        let password = "user_secure_password";
        let hash = hash_password(password).unwrap();

        // Setup a user
        sqlx::query(
            "INSERT INTO users (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_1")
        .bind("user@example.com")
        .bind(hash)
        .bind("token")
        .bind(true)
        .execute(&pool)
        .await
        .unwrap();

        // 1. Test login fail - collection exists as a table but not registered in _collections
        let err_not_found_col = service
            .authenticate("users", "user@example.com", password)
            .await
            .unwrap_err();
        assert!(
            matches!(err_not_found_col, APIError::NotFound { ref resource } if resource == "users")
        );

        // Setup collection entry in _collections
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
        )
        .bind("users_col_id")
        .bind(1)
        .bind("auth")
        .bind("users")
        .bind("[]")
        .bind("{\"authToken\": {\"secret\": \"super-secret-key\"}}")
        .execute(&pool)
        .await
        .unwrap();

        // 2. Test login success (now that it is registered in _collections)
        let tokens = service
            .authenticate("users", "user@example.com", password)
            .await
            .unwrap();
        let claims = verify_token(
            &tokens.access_token,
            &format!("{}-{}", "super-secret-key", "token"),
        )
        .unwrap();
        assert_eq!(claims.id, "user_id_1");
        assert_eq!(claims.collection_id, "users_col_id");

        // 3. Test login fail - wrong password
        let err_unauthorized = service
            .authenticate("users", "user@example.com", "wrong_pass")
            .await
            .unwrap_err();
        assert!(matches!(err_unauthorized, APIError::Unauthorized));

        // 4. Test login fail - user does not exist in the collection
        let err_not_found_user = service
            .authenticate("users", "nonexistent@example.com", password)
            .await
            .unwrap_err();
        assert!(
            matches!(err_not_found_user, APIError::NotFound { ref resource } if resource == "nonexistent@example.com")
        );
    }

    #[tokio::test]
    async fn test_refresh_token_success() {
        let (service, pool) = setup_service("auth_refresh_token_success").await;

        let password = "admin_secure_password";
        let hash = hash_password(password).unwrap();
        let admin_uuid = uuid::Uuid::parse_str("936da01f-9abd-4d9d-80c7-02af85c822a8").unwrap();

        sqlx::query(
            "INSERT INTO _superusers (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(admin_uuid)
        .bind("admin@example.com")
        .bind(hash)
        .bind("token")
        .bind(true)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
        )
        .bind("admin_col_id")
        .bind(1)
        .bind("auth")
        .bind("_superusers")
        .bind("[]")
        .bind("{\"authToken\": {\"secret\": \"super-secret-key\"}}")
        .execute(&pool)
        .await
        .unwrap();

        let login_tokens = service
            .authenticate("_superusers", "admin@example.com", password)
            .await
            .unwrap();

        let refreshed_tokens = service
            .refresh_token(
                "_superusers",
                "admin@example.com",
                &login_tokens.refresh_token,
            )
            .await
            .unwrap();

        assert!(!refreshed_tokens.access_token.is_empty());
        assert!(!refreshed_tokens.refresh_token.is_empty());
    }
}
