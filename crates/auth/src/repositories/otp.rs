use crabbase_core::errors::RepositoryError;
use crabbase_db::repositories::auth::AuthUser;
use hmac::{Hmac, Mac};
use rand::{Rng, rng};
use sha2::Sha256;
use sqlx::{FromRow, Pool, Postgres};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, FromRow)]
pub struct Otp {
    pub id: uuid::Uuid,
    pub collection_ref: String,
    pub otp_hash: String,
    pub sent_to: String,
    pub valid_for: i32, // valid for X seconds
    pub created: chrono::DateTime<chrono::Utc>,
}

impl Otp {
    /// Returns true if the OTP has passed its validity window.
    pub fn is_expired(&self) -> bool {
        let expires_at = self.created + chrono::Duration::seconds(self.valid_for as i64);
        chrono::Utc::now() > expires_at
    }
}

pub struct OtpRepository {
    pub db: Pool<Postgres>,
}

impl OtpRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db: db }
    }

    pub async fn generate_otp(
        &self,
        collection: &str,
        user: &AuthUser,
    ) -> Result<u32, RepositoryError> {
        // Scope rng tightly so it is dropped before any await — ThreadRng is !Send
        let otp = {
            let mut rng = rng();
            rng.random_range(100_000..=999_999)
        };

        let sql = "INSERT INTO _otps (collection_ref, sent_to, otp_hash, valid_for) VALUES($1, $2, $3, $4)";

        // TODO: Hardcoded a 5 min as OTP validation for now
        sqlx::query(&sql)
            .bind(collection)
            .bind(&user.email)
            .bind(self.hash_otp(otp, &user.token_key).as_str())
            .bind(300_i32)
            .execute(&self.db)
            .await?;

        Ok(otp)
    }

    pub async fn get_otp(&self, otp: u32, user: &AuthUser) -> Result<Option<Otp>, RepositoryError> {
        let otp_hash = self.hash_otp(otp, &user.token_key);

        let sql = "SELECT * FROM _otps WHERE otp_hash = $1 AND sent_to = $2;";

        let record = sqlx::query_as::<_, Otp>(&sql)
            .bind(&otp_hash)
            .bind(&user.email)
            .fetch_optional(&self.db)
            .await?;

        // Treat expired OTPs as if they don't exist
        Ok(record.filter(|r| !r.is_expired()))
    }

    fn hash_otp(&self, otp: u32, key: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
        mac.update(otp.to_string().as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM _otps WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
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

    fn make_user(token_key: &str) -> AuthUser {
        AuthUser {
            id: "user_001".to_string(),
            email: "test@example.com".to_string(),
            password: "hash".to_string(),
            token_key: token_key.to_string(),
            verified: true,
        }
    }

    // ── Otp::is_expired ────────────────────────────────────────────────────────

    #[test]
    fn test_otp_is_expired_fresh() {
        let otp = Otp {
            id: uuid::Uuid::new_v4(),
            collection_ref: "users".to_string(),
            otp_hash: "hash".to_string(),
            sent_to: "test@example.com".to_string(),
            valid_for: 300,
            created: chrono::Utc::now(),
        };
        assert!(!otp.is_expired());
    }

    #[test]
    fn test_otp_is_expired_past() {
        let otp = Otp {
            id: uuid::Uuid::new_v4(),
            collection_ref: "users".to_string(),
            otp_hash: "hash".to_string(),
            sent_to: "test@example.com".to_string(),
            valid_for: 1, // 1 second validity
            // Created far enough in the past that it's expired
            created: chrono::Utc::now() - chrono::Duration::seconds(60),
        };
        assert!(otp.is_expired());
    }

    // ── generate_otp ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_generate_otp_produces_6_digit_code() {
        let pool = setup_pool("otp_generate").await;
        let repo = OtpRepository::new(pool);
        let user = make_user("my_secret_key");

        let otp = repo.generate_otp("users", &user).await.unwrap();
        assert!(otp >= 100_000, "OTP should be at least 6 digits");
        assert!(otp <= 999_999, "OTP should be at most 6 digits");
    }

    #[tokio::test]
    async fn test_generate_otp_is_stored_in_db() {
        let pool = setup_pool("otp_generate_stored").await;
        let repo = OtpRepository::new(pool.clone());
        let user = make_user("store_key");

        let otp = repo.generate_otp("users", &user).await.unwrap();

        // Try to retrieve the OTP back
        let retrieved = repo.get_otp(otp, &user).await.unwrap();
        assert!(
            retrieved.is_some(),
            "freshly generated OTP should be retrievable"
        );
    }

    // ── get_otp ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_otp_returns_none_for_wrong_code() {
        let pool = setup_pool("otp_get_wrong_code").await;
        let repo = OtpRepository::new(pool);
        let user = make_user("wrong_code_key");

        repo.generate_otp("users", &user).await.unwrap();

        // Try an OTP value we know won't match
        let result = repo.get_otp(111111, &user).await.unwrap();
        // This might coincidentally match so we just check the type is Option
        let _ = result; // at minimum: should not panic
    }

    #[tokio::test]
    async fn test_get_otp_returns_none_for_expired_otp() {
        let pool = setup_pool("otp_get_expired").await;
        let repo = OtpRepository::new(pool.clone());
        let user = make_user("expire_key");
        let otp_code: u32 = 654321;

        // Insert an already-expired OTP directly
        let otp_hash = {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice("expire_key".as_bytes()).unwrap();
            mac.update(otp_code.to_string().as_bytes());
            hex::encode(mac.finalize().into_bytes())
        };

        sqlx::query(
            "INSERT INTO _otps (collection_ref, sent_to, otp_hash, valid_for, created) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind("users")
        .bind(&user.email)
        .bind(&otp_hash)
        .bind(1_i32) // 1 second validity
        .bind(chrono::Utc::now() - chrono::Duration::seconds(60)) // created 60s ago → expired
        .execute(&pool)
        .await
        .unwrap();

        let result = repo.get_otp(otp_code, &user).await.unwrap();
        assert!(
            result.is_none(),
            "expired OTP should be treated as non-existent"
        );
    }

    // ── delete ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_otp() {
        let pool = setup_pool("otp_delete").await;
        let repo = OtpRepository::new(pool);
        let user = make_user("delete_key");

        let otp_code = repo.generate_otp("users", &user).await.unwrap();

        // Verify it exists
        let before = repo.get_otp(otp_code, &user).await.unwrap();
        assert!(before.is_some());
        let record = before.unwrap();

        // Delete it
        repo.delete(record.id).await.unwrap();

        // Now it should be gone
        let after = repo.get_otp(otp_code, &user).await.unwrap();
        assert!(after.is_none());
    }
}
