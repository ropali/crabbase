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
        let user_opt = if claims.collection_id == "_superusers" || claims.collection_id == "admin" {
            self.repo
                .get_superuser_by_id(&claims.id)
                .await
                .map_err(|e| APIError::Internal {
                    message: "Database query failed".to_string(),
                    details: serde_json::json!(e.to_string()),
                })?
        } else {
            self.repo
                .get_user_by_id(&claims.collection_id, &claims.id)
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

        let token_collection = if collection == "admin" {
            "_superusers"
        } else {
            collection
        };

        let token = create_token(&user.id, token_collection, TokenType::Auth)
            .map_err(|_| APIError::Unauthorized)?;

        Ok(token)
    }
}
