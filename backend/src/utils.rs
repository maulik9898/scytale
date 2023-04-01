use bcrypt::{BcryptError, DEFAULT_COST};
use jsonwebtoken::{Header, Validation};

use crate::{
    error::AppError,
    models::{
        jwt::{Claims, Keys, TokenType},
        user::UserEntity,
    },
};

// consume password value to make it unusable
pub async fn encode_token(
    user: &UserEntity,
    keys: &Keys,
    token_type: TokenType,
) -> Result<String, AppError> {
    let token = jsonwebtoken::encode::<Claims>(
        &Header::default(),
        &Claims::from(user, token_type),
        &keys.encoding,
    )
    .map_err(|err| {
        tracing::error!("Error encoding token: {:?}", err);
        AppError::InternalServerError
    })?;

    Ok(token)
}

pub async fn decode_token(token: &str, keys: &Keys) -> Result<Claims, AppError> {
    let claims = jsonwebtoken::decode::<Claims>(token, &keys.decoding, &Validation::default())
        .map_err(|err| {
            tracing::error!("Error decoding token: {:?}", err);
            AppError::InvalidToken
        })?;

    Ok(claims.claims)
}
