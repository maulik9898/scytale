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
        jwt::{Claims, TokenType},
        user::{Role, UserCreate, UserEntity, UserLogin},
    },
    service::user::{create_user, get_user_by_email},
    utils::encode_token,
    AppState,
};

pub struct AuthController {}

impl AuthController {
    pub async fn register(
        State(state): State<Arc<Mutex<AppState>>>,
        Json(payload): Json<UserCreate>,
    ) -> Result<Json<Value>, AppError> {
        let state = state.lock().await;
        let user = get_user_by_email(&state.pool, payload.email.as_str()).await?;

        if let Some(data) = user {
            return Err(AppError::UserAlreadyExits);
        }

        let mut user = payload.clone();

        let user = create_user(&state.pool, &mut user, Role::USER).await?;

        let access_token = encode_token(&user, &state.keys, TokenType::AccessToken).await?;
        let refresh_token = encode_token(&user, &state.keys, TokenType::RefreshToken).await?;

        Ok(Json(json!({
            "message": "User created successfully",
            "id": user.id,
            "role": user.role,
            "access_token": access_token,
            "refresh_token": refresh_token
        })))
    }

    pub async fn login(
        State(state): State<Arc<Mutex<AppState>>>,
        Json(payload): Json<UserLogin>,
    ) -> Result<Json<Value>, AppError> {
        let state = state.lock().await;
        let user = get_user_by_email(&state.pool, payload.email.as_str()).await?;

        if let Some(user) = user {
            let is_verified = user.verify_password(payload.password.as_bytes())?;

            if is_verified {
                let access_token = encode_token(&user, &state.keys, TokenType::AccessToken).await?;
                let refresh_token =
                    encode_token(&user, &state.keys, TokenType::RefreshToken).await?;

                return Ok(Json(json!({
                    "message": "User logged in successfully",
                    "id": user.id,
                    "role": user.role,
                    "access_token": access_token,
                    "refresh_token": refresh_token
                })));
            }

            return Err(AppError::WrongCredential);
        }

        Err(AppError::UserDoesNotExist)
    }

    pub async fn authenticated(
        user: UserEntity,
        State(state): State<Arc<Mutex<AppState>>>,
    ) -> Result<Json<Value>, AppError> {
        Ok(Json(json!(user)))
    }
}
