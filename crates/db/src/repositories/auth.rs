use crabbase_core::{errors::RepositoryError, utils::string_utils::quote_ident};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

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
        let row = sqlx::query(sql).bind(name).fetch_optional(&self.db).await?;

        Ok(row.is_some())
    }

    pub async fn get_superuser_by_id(&self, id: &str) -> Result<Option<AuthUser>, RepositoryError> {
        self.get_user_by_id("_superusers", id)
            .await
            .map_err(|_| RepositoryError::NotFound(id.to_string()))
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

    pub async fn get_user_by_email(
        &self,
        collection: &str,
        email: &str,
    ) -> Result<Option<AuthUser>, RepositoryError> {
        let escpated_table = quote_ident(collection);

        let sql = format!("SELECT * FROM {escpated_table} WHERE email = $1");

        let user = sqlx::query_as::<_, AuthUser>(&sql)
            .bind(email)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }

    pub async fn get_collection_id_by_name(
        &self,
        name: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let sql = "SELECT id FROM _collections WHERE name = $1 LIMIT 1";

        let id = sqlx::query_scalar(sql)
            .bind(name)
            .fetch_optional(&self.db)
            .await?;

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> sqlx::Pool<sqlx::Sqlite> {
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
        pool
    }

    #[tokio::test]
    async fn test_is_auth_collection() {
        let pool = setup_pool().await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a collection with type = 'auth'
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind("auth_col_id")
        .bind(0)
        .bind("auth")
        .bind("users")
        .bind("[]")
        .bind("{}")
        .execute(&pool)
        .await
        .unwrap();

        // Insert a collection with type = 'base'
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind("base_col_id")
        .bind(0)
        .bind("base")
        .bind("posts")
        .bind("[]")
        .bind("{}")
        .execute(&pool)
        .await
        .unwrap();

        assert!(repo.is_auth_collection("users").await.unwrap());
        assert!(!repo.is_auth_collection("posts").await.unwrap());
        assert!(!repo.is_auth_collection("nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_get_superuser_by_id() {
        let pool = setup_pool().await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("admin_id")
        .bind("admin@example.com")
        .bind("hash123")
        .bind("token123")
        .bind(1)
        .execute(&pool)
        .await
        .unwrap();

        let superuser = repo.get_superuser_by_id("admin_id").await.unwrap().unwrap();
        assert_eq!(superuser.id, "admin_id");
        assert_eq!(superuser.email, "admin@example.com");
        assert_eq!(superuser.password_hash, "hash123");
        assert_eq!(superuser.token_key, "token123");
        assert!(superuser.verified);

        let none_superuser = repo.get_superuser_by_id("nonexistent_id").await.unwrap();
        assert!(none_superuser.is_none());
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let pool = setup_pool().await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a user in the `users` table
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_1")
        .bind("user1@example.com")
        .bind("hash456")
        .bind("token456")
        .bind(0)
        .execute(&pool)
        .await
        .unwrap();

        let user = repo
            .get_user_by_id("users", "user_id_1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.id, "user_id_1");
        assert_eq!(user.email, "user1@example.com");
        assert_eq!(user.password_hash, "hash456");
        assert_eq!(user.token_key, "token456");
        assert!(!user.verified);

        let none_user = repo
            .get_user_by_id("users", "nonexistent_user")
            .await
            .unwrap();
        assert!(none_user.is_none());
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let pool = setup_pool().await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a user in the `users` table
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_2")
        .bind("user2@example.com")
        .bind("hash789")
        .bind("token789")
        .bind(1)
        .execute(&pool)
        .await
        .unwrap();

        let user = repo
            .get_user_by_email("users", "user2@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.id, "user_id_2");
        assert_eq!(user.email, "user2@example.com");
        assert_eq!(user.password_hash, "hash789");
        assert_eq!(user.token_key, "token789");
        assert!(user.verified);

        let none_user = repo
            .get_user_by_email("users", "nonexistent@example.com")
            .await
            .unwrap();
        assert!(none_user.is_none());
    }

    #[tokio::test]
    async fn test_get_collection_id_by_name() {
        let pool = setup_pool().await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a collection
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind("col_id_xyz")
        .bind(0)
        .bind("auth")
        .bind("members")
        .bind("[]")
        .bind("{}")
        .execute(&pool)
        .await
        .unwrap();

        let col_id = repo
            .get_collection_id_by_name("members")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(col_id, "col_id_xyz");

        let none_id = repo
            .get_collection_id_by_name("nonexistent_col")
            .await
            .unwrap();
        assert!(none_id.is_none());
    }
}
