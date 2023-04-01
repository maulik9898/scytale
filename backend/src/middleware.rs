use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRef, FromRequest, FromRequestParts, State},
    headers::{authorization::Bearer, Authorization},
    http::{self, request::Parts, Request, StatusCode},
    middleware::Next,
    response::Response,
    Extension, RequestPartsExt, TypedHeader,
};
use jsonwebtoken::{decode, Validation};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::{
    error::AppError,
    models::{
        jwt::{Claims, TokenType},
        state::AppStateType,
        user::{Role, UserEntity},
    },
    service::user::get_user_by_id,
    utils::decode_token,
    AppState,
};

#[async_trait]
impl<S> FromRequestParts<S> for UserEntity
where
    AppStateType: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::MissingToken)?;

        let state = AppStateType::from_ref(&state);
        let state = state.lock().await;

        let claims = decode_token(bearer.token(), &state.keys).await?;

        match claims.token_type {
            TokenType::AccessToken => {
                let id = claims.id;
                let email = claims.email;
                let user = get_user_by_id(&state.pool, id).await?;
                Ok(user)
            }
            TokenType::RefreshToken => {
                return Err(AppError::NotAccessToken);
            }
        }
    }
}
pub async fn is_admin<B>(
    user: UserEntity,
    State(state): State<Arc<Mutex<AppState>>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, AppError> {
    match user.role {
        Role::ADMIN => Ok(next.run(req).await),
        Role::USER => Err(AppError::InsufficientPermission),
    }
}
