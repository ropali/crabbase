use chrono::Utc;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

pub enum TokenType {
    Auth,
    Verification,
    File,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(rename = "type")]
    pub token_type: String,
    pub id: String,

    #[serde(rename = "collectionId")]
    pub collection_id: String,
    pub refreashable: Option<bool>,
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_token(
    user_id: &str,
    collection_id: &str,
    collection_secret: &str,
    user_token: &str,
    token_type: TokenType,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;

    let token_type: String = match token_type {
        TokenType::Auth => "auth".to_string(),
        TokenType::Verification => "verification".to_string(),
        TokenType::File => "file".to_string(),
    };

    let claims = Claims {
        token_type,
        id: user_id.to_string(),
        collection_id: collection_id.to_string(),
        refreashable: Some(false),
        sub: user_id.to_string(),
        exp: now + 3600,
        iat: now,
    };

    let key = format!("{collection_secret}-{user_token}");

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(key.as_bytes()),
    )
}

pub fn verify_token(
    token: &str,
    collection_secret: &str,
    user_token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = format!("{collection_secret}-{user_token}");
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(key.as_bytes()),
        &Validation::default(),
    )?;

    Ok(data.claims)
}

pub fn extract_unverified_claims(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(&[]);

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);

    validation.insecure_disable_signature_validation();

    let token_data = decode::<Claims>(token, &key, &validation)?;

    Ok(token_data.claims)
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "my_super_secure_password";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_token_lifecycle() {
        let user_id = "test_user_123";
        let collection_id = "users_col_xyz";
        let secret = "secret";
        let user_token = "user_token";

        // Test Auth token
        let token =
            create_token(user_id, collection_id, secret, user_token, TokenType::Auth).unwrap();
        let claims = verify_token(&token, secret, user_token).unwrap();
        assert_eq!(claims.id, user_id);
        assert_eq!(claims.collection_id, collection_id);
        assert_eq!(claims.token_type, "auth");
        assert_eq!(claims.sub, user_id);

        // Test Verification token
        let token_ver = create_token(
            user_id,
            collection_id,
            secret,
            user_token,
            TokenType::Verification,
        )
        .unwrap();
        let claims_ver = verify_token(&token_ver, secret, user_token).unwrap();
        assert_eq!(claims_ver.token_type, "verification");

        // Test File token
        let token_file =
            create_token(user_id, collection_id, secret, user_token, TokenType::File).unwrap();
        let claims_file = verify_token(&token_file, secret, user_token).unwrap();
        assert_eq!(claims_file.token_type, "file");
    }

    #[test]
    fn test_verify_invalid_token() {
        let result = verify_token("invalid.token.string", "secret", "user_token");
        assert!(result.is_err());
    }
}
