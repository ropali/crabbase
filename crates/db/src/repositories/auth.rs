use crabbase_core::{errors::RepositoryError, utils::string_utils::quote_ident};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub token_key: String,
    pub verified: bool,
}

#[derive(Debug)]
pub struct AuthRepository {
    db: Pool<Sqlite>,
}

impl AuthRepository {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn is_auth_collection(&self, name: &str) -> Result<bool, RepositoryError> {
        let sql = "SELECT 1 FROM _collections WHERE name = $1 and type = 'auth'";
        let row = sqlx::query(&sql)
            .bind(name)
            .fetch_optional(&self.db)
            .await?;

        Ok(row.is_some())
    }

    pub async fn get_superuser_by_id(&self, id: &str) -> Result<Option<AuthUser>, RepositoryError> {
        self.get_user_by_id("_superusers", &id)
            .await
            .or_else(|_| Err(RepositoryError::NotFound(id.to_string())))
    }

    pub async fn get_user_by_id(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<AuthUser>, RepositoryError> {
        let escpated_table = quote_ident(collection);

        let sql = format!("SELECT * FROM {escpated_table} WHERE id = $1");

        let user = sqlx::query_as::<_, AuthUser>(&sql)
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }
}
