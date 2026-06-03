use crabbase_core::errors::APIError;
use crabbase_db::repositories::auth::{AuthRepository, AuthUser};

use crate::auth::Claims;

pub struct AuthService {
    repo: AuthRepository,
}

impl AuthService {
    pub fn new(repo: AuthRepository) -> Self {
        Self { repo }
    }

    // Verifies auth user session from claims
    pub async fn verify_session(&self, claims: &Claims) -> Result<AuthUser, APIError> {
        let user_opt = if claims.collection_id == "_superusers" {
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
}
