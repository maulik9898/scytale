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
        auth::LoginResponse,
        jwt::{Claims, TokenType},
        user::{Role, UserCreate, UserEntity, UserLogin},
    },
    service::user::{create_user, get_user_by_email},
    utils::encode_token,
    AppState,
};

pub struct AuthController {}

impl AuthController {

    pub async fn login(
        State(state): State<Arc<Mutex<AppState>>>,
        Json(payload): Json<UserLogin>,
    ) -> Result<Json<LoginResponse>, AppError> {
        let state = state.lock().await;
        let user = get_user_by_email(&state.pool, payload.email.as_str()).await?;

        if let Some(user) = user {
            let is_verified = user.verify_password(payload.password.as_bytes())?;

            if is_verified {
                let access_token = encode_token(&user, &state.keys, TokenType::AccessToken).await?;
                let refresh_token =
                    encode_token(&user, &state.keys, TokenType::RefreshToken).await?;

                let response = LoginResponse {
                    message: "User created successfully".to_string(),
                    id: user.id,
                    role: user.role,
                    access_token,
                    refresh_token,
                };

                return Ok(Json(response));
            }

            return Err(AppError::WrongCredential);
        }

        Err(AppError::UserDoesNotExist)
    }

    pub async fn authenticated(
        user: UserEntity,
        State(state): State<Arc<Mutex<AppState>>>,
    ) -> Result<Json<UserEntity>, AppError> {
        Ok(Json(user))
    }
}
