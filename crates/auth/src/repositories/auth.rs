use crabbase_core::errors::RepositoryError;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};

pub fn hash_token_jti(jti: &str) -> String {
    let mut hasher = Sha256::new();

    hasher.update(jti.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRecord {
    pub id: uuid::Uuid,
    pub family_id: uuid::Uuid,
    pub collection_ref: String,
    pub record_ref: String,
    pub token_hash: String,
    pub parent_id: Option<uuid::Uuid>,
    pub used: bool,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked: bool,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created: chrono::DateTime<chrono::Utc>,
    pub updated: chrono::DateTime<chrono::Utc>,
}

pub struct AuthRepository {
    pub db: Pool<Postgres>,
}

impl AuthRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        AuthRepository { db }
    }

    pub async fn create_refresh_token(
        &self,
        family_id: uuid::Uuid,
        collection_ref: &str,
        record_ref: &str,
        jti: &str,
        parent_id: Option<uuid::Uuid>,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<RefreshTokenRecord, RepositoryError> {
        let token_hash = hash_token_jti(jti);

        let sql = r#"
            INSERT INTO _refresh_tokens (family_id, collection_ref, record_ref, token_hash, parent_id, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
        "#;

        let record = sqlx::query_as::<_, RefreshTokenRecord>(sql)
            .bind(family_id)
            .bind(collection_ref)
            .bind(record_ref)
            .bind(token_hash)
            .bind(parent_id)
            .bind(expires_at)
            .fetch_one(&self.db)
            .await?;

        Ok(record)
    }

    pub async fn get_refresh_token_by_jti(
        &self,
        jti: &str,
    ) -> Result<Option<RefreshTokenRecord>, RepositoryError> {
        let token_hash = hash_token_jti(jti);

        let sql = "SELECT * FROM _refresh_tokens WHERE token_hash = $1";

        let record = sqlx::query_as::<_, RefreshTokenRecord>(sql)
            .bind(token_hash)
            .fetch_optional(&self.db)
            .await?;

        Ok(record)
    }

    // OPTIMIZED ATOMIC ROTATION CTE: Consumes old token & issues new token in 1 DB roundtrip
    pub async fn rotate_refreh_token(
        &self,
        old_jti: &str,
        new_jti: &str,
        new_expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<RefreshTokenRecord>, RepositoryError> {
        let old_token_hash = hash_token_jti(old_jti);
        let new_token_hash = hash_token_jti(new_jti);

        let sql = r#"
            WITH consumed AS (
                UPDATE _refresh_tokens
                SET used = TRUE, used_at = NOW(), updated = NOW()
                WHERE token_hash = $1 AND used = FALSE AND revoked = FALSE AND expires_at > NOW()
                RETURNING id, family_id, collection_ref, record_ref
            )
            INSERT INTO _refresh_tokens (family_id, collection_ref, record_ref, token_hash, parent_id, expires_at)
            SELECT family_id, collection_ref, record_ref, $2, id, $3
            FROM consumed
            RETURNING *;
        "#;

        let new_record = sqlx::query_as::<_, RefreshTokenRecord>(sql)
            .bind(old_token_hash)
            .bind(new_token_hash)
            .bind(new_expires_at)
            .fetch_optional(&self.db)
            .await?;

        Ok(new_record)
    }

    pub async fn get_child_refresh_token(
        &self,
        parent_id: uuid::Uuid,
    ) -> Result<Option<RefreshTokenRecord>, RepositoryError> {
        let sql =
            "SELECT * FROM _refresh_tokens WHERE parent_id = $1 ORDER BY created DESC LIMIT 1";

        let record = sqlx::query_as::<_, RefreshTokenRecord>(sql)
            .bind(parent_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(record)
    }

    pub async fn revoke_token_family(&self, family_id: uuid::Uuid) -> Result<(), RepositoryError> {
        let sql = "UPDATE _refresh_tokens SET revoked = TRUE, updated = NOW() WHERE family_id = $1 AND revoked = FALSE";
        sqlx::query(sql).bind(family_id).execute(&self.db).await?;
        Ok(())
    }

    // Log security breach into _logs table
    pub async fn log_security_event(
        &self,
        message: &str,
        data: serde_json::Value,
    ) -> Result<(), RepositoryError> {
        // let id = format!("log_{}", uuid::Uuid::new_v4().simple());
        // let sql = "INSERT INTO _logs (id, level, message, data) VALUES ($1, 2, $2, $3)";
        // sqlx::query(sql)
        //     .bind(id)
        //     .bind(message)
        //     .bind(data.to_string())
        //     .execute(&self.db)
        //     .await?;
        // Ok(())
        //
        todo!()
    }

    // Optimized batch cleanup of expired tokens (Chunked to prevent table lock)
    pub async fn purge_expired_refresh_tokens(&self) -> Result<u64, RepositoryError> {
        let sql = r#"
            DELETE FROM _refresh_tokens
            WHERE id IN (
                SELECT id FROM _refresh_tokens
                WHERE expires_at < NOW()
                LIMIT 1000
            )
        "#;
        let result = sqlx::query(sql).execute(&self.db).await?;
        Ok(result.rows_affected())
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
        pool
    }

    fn make_repo(pool: sqlx::Pool<sqlx::Postgres>) -> AuthRepository {
        AuthRepository::new(pool)
    }

    fn future_expires_at() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + chrono::Duration::days(7)
    }

    fn past_expires_at() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() - chrono::Duration::seconds(1)
    }

    // ── hash_token_jti ────────────────────────────────────────────────────────

    #[test]
    fn test_hash_token_jti_deterministic() {
        let jti = "some-token-id-123";
        let h1 = hash_token_jti(jti);
        let h2 = hash_token_jti(jti);
        assert_eq!(h1, h2);
        assert!(!h1.is_empty());
    }

    #[test]
    fn test_hash_token_jti_different_inputs_produce_different_hashes() {
        let h1 = hash_token_jti("token-a");
        let h2 = hash_token_jti("token-b");
        assert_ne!(h1, h2);
    }

    // ── create_refresh_token ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_refresh_token() {
        let pool = setup_pool("auth_repo_create_rt").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let jti = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        let record = repo
            .create_refresh_token(family_id, "users", "user_123", &jti, None, expires_at)
            .await
            .unwrap();

        assert_eq!(record.family_id, family_id);
        assert_eq!(record.collection_ref, "users");
        assert_eq!(record.record_ref, "user_123");
        assert!(!record.used);
        assert!(!record.revoked);
        assert!(record.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_create_refresh_token_with_parent() {
        let pool = setup_pool("auth_repo_create_rt_parent").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let jti1 = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        let parent = repo
            .create_refresh_token(family_id, "users", "user_abc", &jti1, None, expires_at)
            .await
            .unwrap();

        let jti2 = uuid::Uuid::new_v4().to_string();
        let child = repo
            .create_refresh_token(
                family_id,
                "users",
                "user_abc",
                &jti2,
                Some(parent.id),
                expires_at,
            )
            .await
            .unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
        assert_eq!(child.family_id, family_id);
    }

    // ── get_refresh_token_by_jti ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_refresh_token_by_jti_found() {
        let pool = setup_pool("auth_repo_get_rt_jti").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let jti = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        repo.create_refresh_token(family_id, "col", "rec", &jti, None, expires_at)
            .await
            .unwrap();

        let found = repo.get_refresh_token_by_jti(&jti).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().family_id, family_id);
    }

    #[tokio::test]
    async fn test_get_refresh_token_by_jti_not_found() {
        let pool = setup_pool("auth_repo_get_rt_jti_missing").await;
        let repo = make_repo(pool);

        let result = repo
            .get_refresh_token_by_jti("nonexistent-jti")
            .await
            .unwrap();

        assert!(result.is_none());
    }

    // ── rotate_refresh_token ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_rotate_refresh_token_success() {
        let pool = setup_pool("auth_repo_rotate_rt").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let old_jti = uuid::Uuid::new_v4().to_string();
        let new_jti = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        // Create the initial token
        repo.create_refresh_token(family_id, "col", "rec", &old_jti, None, expires_at)
            .await
            .unwrap();

        // Rotate it
        let new_record = repo
            .rotate_refreh_token(&old_jti, &new_jti, expires_at)
            .await
            .unwrap();

        assert!(new_record.is_some(), "rotation should succeed");
        let new_rec = new_record.unwrap();
        assert_eq!(new_rec.family_id, family_id);
        assert!(!new_rec.used);
        assert!(!new_rec.revoked);

        // Old token should now be marked used
        let old_rec = repo
            .get_refresh_token_by_jti(&old_jti)
            .await
            .unwrap()
            .unwrap();
        assert!(old_rec.used);
        assert!(old_rec.used_at.is_some());
    }

    #[tokio::test]
    async fn test_rotate_refresh_token_already_used_returns_none() {
        let pool = setup_pool("auth_repo_rotate_rt_used").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let old_jti = uuid::Uuid::new_v4().to_string();
        let new_jti1 = uuid::Uuid::new_v4().to_string();
        let new_jti2 = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        repo.create_refresh_token(family_id, "col", "rec", &old_jti, None, expires_at)
            .await
            .unwrap();

        // First rotation: should succeed
        repo.rotate_refreh_token(&old_jti, &new_jti1, expires_at)
            .await
            .unwrap();

        // Second rotation of same old token: already used → None
        let second = repo
            .rotate_refreh_token(&old_jti, &new_jti2, expires_at)
            .await
            .unwrap();

        assert!(
            second.is_none(),
            "second rotation of an already-used token should return None"
        );
    }

    #[tokio::test]
    async fn test_rotate_expired_token_returns_none() {
        let pool = setup_pool("auth_repo_rotate_rt_expired").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let old_jti = uuid::Uuid::new_v4().to_string();
        let new_jti = uuid::Uuid::new_v4().to_string();

        // Create a token that is already expired
        repo.create_refresh_token(family_id, "col", "rec", &old_jti, None, past_expires_at())
            .await
            .unwrap();

        let result = repo
            .rotate_refreh_token(&old_jti, &new_jti, future_expires_at())
            .await
            .unwrap();

        assert!(
            result.is_none(),
            "rotating an expired token should return None"
        );
    }

    // ── get_child_refresh_token ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_child_refresh_token() {
        let pool = setup_pool("auth_repo_child_rt").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let old_jti = uuid::Uuid::new_v4().to_string();
        let new_jti = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        // Create parent token
        let parent = repo
            .create_refresh_token(family_id, "col", "rec", &old_jti, None, expires_at)
            .await
            .unwrap();

        // Rotate to create a child
        let child_record = repo
            .rotate_refreh_token(&old_jti, &new_jti, expires_at)
            .await
            .unwrap()
            .unwrap();

        // Retrieve child by parent id
        let found_child = repo.get_child_refresh_token(parent.id).await.unwrap();

        assert!(found_child.is_some());
        assert_eq!(found_child.unwrap().id, child_record.id);
    }

    #[tokio::test]
    async fn test_get_child_refresh_token_no_child() {
        let pool = setup_pool("auth_repo_child_rt_none").await;
        let repo = make_repo(pool);

        let nonexistent_parent = uuid::Uuid::new_v4();
        let result = repo
            .get_child_refresh_token(nonexistent_parent)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    // ── revoke_token_family ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_revoke_token_family() {
        let pool = setup_pool("auth_repo_revoke_family").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let jti1 = uuid::Uuid::new_v4().to_string();
        let jti2 = uuid::Uuid::new_v4().to_string();
        let expires_at = future_expires_at();

        // Create two tokens in the same family
        repo.create_refresh_token(family_id, "col", "rec", &jti1, None, expires_at)
            .await
            .unwrap();
        repo.create_refresh_token(family_id, "col", "rec", &jti2, None, expires_at)
            .await
            .unwrap();

        // Revoke the whole family
        repo.revoke_token_family(family_id).await.unwrap();

        // Both tokens should now be revoked
        let t1 = repo.get_refresh_token_by_jti(&jti1).await.unwrap().unwrap();
        let t2 = repo.get_refresh_token_by_jti(&jti2).await.unwrap().unwrap();

        assert!(t1.revoked);
        assert!(t2.revoked);
    }

    // ── purge_expired_refresh_tokens ──────────────────────────────────────────

    #[tokio::test]
    async fn test_purge_expired_refresh_tokens() {
        let pool = setup_pool("auth_repo_purge_rt").await;
        let repo = make_repo(pool);

        let family_id = uuid::Uuid::new_v4();
        let expired_jti = uuid::Uuid::new_v4().to_string();
        let valid_jti = uuid::Uuid::new_v4().to_string();

        // Create one expired token and one valid token
        repo.create_refresh_token(
            family_id,
            "col",
            "rec",
            &expired_jti,
            None,
            past_expires_at(),
        )
        .await
        .unwrap();

        repo.create_refresh_token(
            family_id,
            "col",
            "rec",
            &valid_jti,
            None,
            future_expires_at(),
        )
        .await
        .unwrap();

        let deleted = repo.purge_expired_refresh_tokens().await.unwrap();
        assert!(deleted >= 1, "at least the expired token should be purged");

        // Expired token should be gone
        let expired = repo.get_refresh_token_by_jti(&expired_jti).await.unwrap();
        assert!(expired.is_none());

        // Valid token should still exist
        let valid = repo.get_refresh_token_by_jti(&valid_jti).await.unwrap();
        assert!(valid.is_some());
    }
}
