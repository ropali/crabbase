use crabbase_core::errors::RepositoryError;
use crabbase_db::repositories::auth::{AuthUser, UserRepository};
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
