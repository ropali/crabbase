use crabbase_core::{
    errors::RepositoryError, models::Collection, utils::string_utils::quote_ident,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

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
    db: Pool<Postgres>,
}

impl AuthRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
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

    pub async fn get_collection_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Collection>, RepositoryError> {
        let sql = "SELECT * FROM _collections WHERE name = $1 LIMIT 1";

        let col = sqlx::query_as::<_, Collection>(sql)
            .bind(name)
            .fetch_optional(&self.db)
            .await?;

        Ok(col)
    }

    pub async fn get_collection_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Collection>, RepositoryError> {
        let sql = "SELECT * FROM _collections WHERE id = $1 LIMIT 1";

        let col = sqlx::query_as::<_, Collection>(sql)
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn setup_pool(schema: &str) -> sqlx::Pool<sqlx::Postgres> {
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
        pool
    }

    #[tokio::test]
    async fn test_is_auth_collection() {
        let pool = setup_pool("db_auth_is_auth_collection").await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a collection with type = 'auth'
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
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
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
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
        let pool = setup_pool("db_auth_get_superuser_by_id").await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a superuser
        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("admin_id")
        .bind("admin@example.com")
        .bind("hash123")
        .bind("token123")
        .bind(true)
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

    async fn create_users_table(pool: &sqlx::Pool<sqlx::Postgres>) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id             TEXT PRIMARY KEY NOT NULL,
                email          TEXT UNIQUE NOT NULL,
                password_hash  TEXT NOT NULL,
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
    async fn test_get_user_by_id() {
        let pool = setup_pool("db_auth_get_user_by_id").await;
        create_users_table(&pool).await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a user in the `users` table
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_1")
        .bind("user1@example.com")
        .bind("hash456")
        .bind("token456")
        .bind(false)
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
        let pool = setup_pool("db_auth_get_user_by_email").await;
        create_users_table(&pool).await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a user in the `users` table
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("user_id_2")
        .bind("user2@example.com")
        .bind("hash789")
        .bind("token789")
        .bind(true)
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
        let pool = setup_pool("db_auth_get_collection_id_by_name").await;
        let repo = AuthRepository::new(pool.clone());

        // Insert a collection
        sqlx::query(
            "INSERT INTO _collections (id, system, type, name, fields, options) VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)"
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
            .get_collection_by_name("members")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(col_id.id, "col_id_xyz");

        let none_id = repo
            .get_collection_by_name("nonexistent_col")
            .await
            .unwrap();
        assert!(none_id.is_none());
    }
}
