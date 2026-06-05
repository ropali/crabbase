use crabbase_core::errors::APIError;
use crabbase_db::repositories::auth::{AuthRepository, AuthUser};

use crate::auth::{Claims, TokenType, create_token, verify_password};

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
    ) -> Result<String, APIError> {
        let user_opt = if collection == "admin" {
            self.repo.get_user_by_email("_superusers", email).await?
        } else {
            self.repo.get_user_by_email(collection, email).await?
        };

        let user = user_opt.ok_or(APIError::NotFound {
            resource: email.to_string(),
        })?;

        let is_valid =
            verify_password(password, &user.password_hash).map_err(|_| APIError::Unauthorized)?;

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

        let token = create_token(
            &user.id,
            &col.id,
            col_token,
            &user.token_key,
            TokenType::Auth,
        )
        .map_err(|_| APIError::Unauthorized)?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Claims, hash_password, verify_token};
    use crabbase_core::errors::APIError;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_service() -> (AuthService, sqlx::Pool<sqlx::Sqlite>) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
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
        let (service, pool) = setup_service().await;

        // 1. Setup a verified superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("admin_id_1")
        .bind("admin1@example.com")
        .bind("hash")
        .bind("token")
        .bind(1) // verified
        .execute(&pool)
        .await
        .unwrap();

        // 2. Setup an unverified superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("admin_id_2")
        .bind("admin2@example.com")
        .bind("hash")
        .bind("token")
        .bind(0) // unverified
        .execute(&pool)
        .await
        .unwrap();

        // Test verified superuser session verification
        let claims_verified = Claims {
            token_type: "auth".to_string(),
            id: "admin_id_1".to_string(),
            collection_id: "_superusers".to_string(),
            refreashable: Some(false),
            sub: "admin_id_1".to_string(),
            exp: 0,
            iat: 0,
        };
        let user = service.verify_session(&claims_verified).await.unwrap();
        assert_eq!(user.id, "admin_id_1");
        assert_eq!(user.email, "admin1@example.com");
        assert!(user.verified);

        // Test with collection_id as "admin"
        let claims_admin = Claims {
            token_type: "auth".to_string(),
            id: "admin_id_1".to_string(),
            collection_id: "admin".to_string(),
            refreashable: Some(false),
            sub: "admin_id_1".to_string(),
            exp: 0,
            iat: 0,
        };
        let user_admin = service.verify_session(&claims_admin).await.unwrap();
        assert_eq!(user_admin.id, "admin_id_1");

        // Test unverified superuser session verification
        let claims_unverified = Claims {
            token_type: "auth".to_string(),
            id: "admin_id_2".to_string(),
            collection_id: "_superusers".to_string(),
            refreashable: Some(false),
            sub: "admin_id_2".to_string(),
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
            id: "nonexistent_admin".to_string(),
            collection_id: "_superusers".to_string(),
            refreashable: Some(false),
            sub: "nonexistent_admin".to_string(),
            exp: 0,
            iat: 0,
        };
        let err_unauthorized = service
            .verify_session(&claims_nonexistent)
            .await
            .unwrap_err();
        assert!(matches!(err_unauthorized, APIError::Unauthorized));
    }

    async fn create_users_table(pool: &sqlx::Pool<sqlx::Sqlite>) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id             TEXT PRIMARY KEY NOT NULL,
                email          TEXT UNIQUE NOT NULL,
                password_hash  TEXT NOT NULL,
                token_key      TEXT NOT NULL,
                email_visible  INTEGER NOT NULL DEFAULT 0,
                verified       INTEGER NOT NULL DEFAULT 0,
                created        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
                updated        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
            );
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_verify_session_regular_user() {
        let (service, pool) = setup_service().await;
        create_users_table(&pool).await;

        // 1. Setup a verified user
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_1")
        .bind("user1@example.com")
        .bind("hash")
        .bind("token")
        .bind(1) // verified
        .execute(&pool)
        .await
        .unwrap();

        // 2. Setup an unverified user
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_2")
        .bind("user2@example.com")
        .bind("hash")
        .bind("token")
        .bind(0) // unverified
        .execute(&pool)
        .await
        .unwrap();

        // Test verified user session verification
        let claims_verified = Claims {
            token_type: "auth".to_string(),
            id: "user_id_1".to_string(),
            collection_id: "users".to_string(),
            refreashable: Some(false),
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
            refreashable: Some(false),
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
            refreashable: Some(false),
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
        let (service, pool) = setup_service().await;

        let password = "admin_secure_password";
        let hash = hash_password(password).unwrap();

        // Setup a superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("admin_id_1")
        .bind("admin@example.com")
        .bind(hash)
        .bind("token")
        .bind(1)
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
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5, $6)"
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
        let token = service
            .authenticate("admin", "admin@example.com", password)
            .await
            .unwrap();
        let claims = verify_token(&token, "super-secret-key", "token").unwrap();
        assert_eq!(claims.id, "admin_id_1");
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
        let (service, pool) = setup_service().await;
        create_users_table(&pool).await;

        let password = "user_secure_password";
        let hash = hash_password(password).unwrap();

        // Setup a user
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_1")
        .bind("user@example.com")
        .bind(hash)
        .bind("token")
        .bind(1)
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
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5, $6)"
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
        let token = service
            .authenticate("users", "user@example.com", password)
            .await
            .unwrap();
        let claims = verify_token(&token, "super-secret-key", "token").unwrap();
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
