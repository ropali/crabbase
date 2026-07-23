use crabbase_core::errors::APIError;
use crabbase_db::repositories::auth::{AuthRepository, AuthUser};
use serde::{Deserialize, Serialize};

use crate::auth::{Claims, TokenType, create_token, verify_password, verify_token};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthTokens {
    #[serde(rename = "accessToken")]
    pub access_token: String,

    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

pub struct AuthService {
    repo: AuthRepository,
}

impl AuthService {
    pub fn new(repo: AuthRepository) -> Self {
        Self { repo }
    }

    // Verifies auth user session from claims
    pub async fn verify_session(&self, claims: &Claims) -> Result<AuthUser, APIError> {
        let collection_name = self
            .repo
            .get_collection_by_id(&claims.collection_id)
            .await
            .map_err(|e| APIError::Internal {
                message: "Database query failed".to_string(),
                details: serde_json::json!(e.to_string()),
            })?
            .map(|col| col.name)
            .unwrap_or_else(|| claims.collection_id.clone());

        let user_opt = if collection_name == "_superusers" || collection_name == "admin" {
            self.repo
                .get_superuser_by_id(&claims.id)
                .await
                .map_err(|e| APIError::Internal {
                    message: "Database query failed".to_string(),
                    details: serde_json::json!(e.to_string()),
                })?
        } else {
            self.repo
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
        let user_opt = self.repo.get_user_by_email(collection, email).await?;

        let user = user_opt.ok_or(APIError::NotFound {
            resource: email.to_string(),
        })?;

        let is_valid =
            verify_password(password, &user.password).map_err(|_| APIError::Unauthorized)?;

        if !is_valid {
            return Err(APIError::Unauthorized);
        }

        let col = match self.repo.get_collection_by_name(collection).await? {
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

        let key = format!("{}-{}", col_token, user.token_key);

        let duration: Option<usize> = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("duration"))
            .and_then(|v| v.as_number())
            .and_then(|n| n.as_u64())
            .and_then(|num| num.try_into().ok());

        let access_token = create_token(&user.id, &col.id, &key, TokenType::Auth, duration)
            .map_err(|_| APIError::Unauthorized)?;

        // 7 days valid
        // TODO: remove the hardcoded duration
        let refresh_token = create_token(&user.id, &col.id, &key, TokenType::Refresh, Some(604800))
            .map_err(|_| APIError::Unauthorized)?;

        let token = AuthTokens {
            access_token: access_token,
            refresh_token: refresh_token,
        };

        Ok(token)
    }

    pub async fn refresh_token(
        &self,
        collection: &str,
        email: &str,
        refresh_token: &str,
    ) -> Result<AuthTokens, APIError> {
        let user_opt = self.repo.get_user_by_email(collection, email).await?;

        let user = user_opt.ok_or(APIError::NotFound {
            resource: email.to_string(),
        })?;

        let col = match self.repo.get_collection_by_name(collection).await? {
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

        let claims = verify_token(refresh_token, col_token, &user.token_key)
            .map_err(|_| APIError::Unauthorized)?;

        let key = format!("{}-{}", col_token, user.token_key);

        let duration: Option<usize> = col
            .options
            .auth_token
            .as_ref()
            .and_then(|t| t.get("duration"))
            .and_then(|v| v.as_number())
            .and_then(|n| n.as_u64())
            .and_then(|num| num.try_into().ok());

        let access_token = create_token(&user.id, &col.id, &key, TokenType::Auth, duration)
            .map_err(|_| APIError::Unauthorized)?;

        // 7 days valid
        // TODO: remove the hardcoded duration
        let refresh_token = create_token(&user.id, &col.id, &key, TokenType::Refresh, Some(604800))
            .map_err(|_| APIError::Unauthorized)?;

        let token = AuthTokens {
            access_token: access_token,
            refresh_token: refresh_token,
        };

        Ok(token)
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

        let repo = AuthRepository::new(pool.clone());
        let service = AuthService::new(repo);
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
            refreshable: false,
            sub: "936da01f-9abd-4d9d-80c7-02af85c822a8".to_string(),
            exp: 0,
            iat: 0,
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
            refreshable: false,
            sub: "936da01f-9abd-4d9d-80c7-02af85c822a8".to_string(),
            exp: 0,
            iat: 0,
        };
        let user_admin = service.verify_session(&claims_admin).await.unwrap();
        assert_eq!(user_admin.id, "936da01f-9abd-4d9d-80c7-02af85c822a8");

        // Test unverified superuser session verification
        let claims_unverified = Claims {
            token_type: "auth".to_string(),
            id: "f47ac10b-58cc-4372-a567-0e02b2c3d479".to_string(),
            collection_id: "_superusers".to_string(),
            refreshable: false,
            sub: "f47ac10b-58cc-4372-a567-0e02b2c3d479".to_string(),
            exp: 0,
            iat: 0,
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
            refreshable: false,
            sub: "ba8f95c5-cc1a-4fa6-a70e-f0bcfd96c9e0".to_string(),
            exp: 0,
            iat: 0,
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
            refreshable: false,
            sub: "user_id_1".to_string(),
            exp: 0,
            iat: 0,
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
            refreshable: false,
            sub: "user_id_2".to_string(),
            exp: 0,
            iat: 0,
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
            refreshable: false,
            sub: "nonexistent_user".to_string(),
            exp: 0,
            iat: 0,
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

        // 1. Test login fail - admin collection not in _collections
        let err_not_found_col = service
            .authenticate("admin", "admin@example.com", password)
            .await
            .unwrap_err();
        assert!(
            matches!(err_not_found_col, APIError::NotFound { ref resource } if resource == "admin")
        );

        // Setup "admin" in _collections table so collection ID can be queried
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
        )
        .bind("admin_col_id")
        .bind(1)
        .bind("auth")
        .bind("admin")
        .bind("[]")
        .bind("{\"authToken\": {\"secret\": \"super-secret-key\"}}")
        .execute(&pool)
        .await
        .unwrap();

        // 2. Test login success (returns token)
        let tokens = service
            .authenticate("admin", "admin@example.com", password)
            .await
            .unwrap();
        let claims = verify_token(&tokens.access_token, "super-secret-key", "token").unwrap();
        assert_eq!(claims.id, "936da01f-9abd-4d9d-80c7-02af85c822a8");
        assert_eq!(claims.collection_id, "admin_col_id");

        // 3. Test login fail - wrong password
        let err_unauthorized = service
            .authenticate("admin", "admin@example.com", "wrong_pass")
            .await
            .unwrap_err();
        assert!(matches!(err_unauthorized, APIError::Unauthorized));

        // 4. Test login fail - non-existent superuser
        let err_not_found = service
            .authenticate("admin", "nonexistent@example.com", password)
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
        let claims = verify_token(&tokens.access_token, "super-secret-key", "token").unwrap();
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
}
