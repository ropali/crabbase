use chrono::Utc;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const SECRET: &[u8] = b"my-secret-key";

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
        collection_id: "".to_string(),
        refreashable: Some(false),
        sub: user_id.to_string(),
        exp: now + 3600,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET),
    )
}

pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET),
        &Validation::default(),
    )?;

    Ok(data.claims)
}
