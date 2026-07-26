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
