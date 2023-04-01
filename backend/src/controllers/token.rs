use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use jsonwebtoken::Header;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::{
    error::AppError,
    models::{
        self,
        jwt::{Claims, TokeRefresh, TokenType},
        user::{Role, UserCreate, UserEntity, UserLogin},
    },
    service::user::{create_user, get_user_by_email, get_user_by_id_email},
    utils::{decode_token, encode_token},
    AppState,
};
pub struct TokenController {}

impl TokenController {
    pub async fn authenticated(user: UserEntity) -> Result<Json<Value>, AppError> {
        Ok(Json(json!(user)))
    }

    pub async fn refresh(
        State(state): State<Arc<Mutex<AppState>>>,
        Json(payload): Json<TokeRefresh>,
    ) -> Result<Json<Value>, AppError> {
        let state = state.lock().await;
        let claims = decode_token(&payload.refresh_token.as_str(), &state.keys).await?;
        match claims.token_type {
            TokenType::AccessToken => {
                return Err(AppError::NotRefreshToken);
            }
            TokenType::RefreshToken => {
                let user =
                    get_user_by_id_email(&state.pool, claims.id, claims.email.as_str()).await?;

                let access_token = encode_token(&user, &state.keys, TokenType::AccessToken).await?;
                let refresh_token =
                    encode_token(&user, &state.keys, TokenType::RefreshToken).await?;

                return Ok(Json(json!({
                    "id": user.id,
                    "role": user.role,
                    "access_token": access_token,
                    "refresh_token": refresh_token
                })));
            }
        }
    }
}
