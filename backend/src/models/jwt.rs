use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};

use super::user::{Role, UserEntity};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub email: String,
    pub role: Role,
    pub id: i64,
    pub exp: i64,
    pub token_type: TokenType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TokenType {
    AccessToken,
    RefreshToken,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokeRefresh {
    pub refresh_token: String,
}

impl Claims {
    pub fn from(user: &UserEntity, token_type: TokenType) -> Self {
        let iat = Utc::now();
        let exp = iat + Duration::hours(24);
        Self {
            email: user.email.clone(),
            role: user.role.clone(),
            id: user.id,
            exp: exp.timestamp(),
            token_type,
        }
    }
}

#[derive(Clone)]
pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}
