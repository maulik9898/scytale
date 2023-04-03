use std::sync::Arc;

use axum::{extract::State, Json, http::StatusCode};
use serde_json::{json, Value};
use tokio::sync::{RwLock, Mutex};

use crate::{error::AppError, models::{user::{UserEntity, UserCreate, Role}, auth::LoginResponse, jwt::TokenType}, AppState, service::user::{get_user_by_email, create_user}, utils::encode_token};

pub struct AdminController {}

impl AdminController {
    pub async fn admin(user: UserEntity) -> Result<Json<UserEntity>, AppError> {
        Ok(Json(user))
    }

    pub async fn register(
        State(state): State<Arc<Mutex<AppState>>>,
        Json(payload): Json<UserCreate>,
    ) -> Result<(StatusCode, Json<LoginResponse>), AppError> {
        if payload.password.is_empty() || payload.email.is_empty() || payload.name.is_empty() {
            return Err(AppError::EmptyPayload);
        }
        let state = state.lock().await;
        let user = get_user_by_email(&state.pool, payload.email.as_str()).await?;

        if let Some(data) = user {
            return Err(AppError::UserAlreadyExits);
        }

        let mut user = payload.clone();

        let user = create_user(&state.pool, &mut user).await?;

        let access_token = encode_token(&user, &state.keys, TokenType::AccessToken).await?;
        let refresh_token = encode_token(&user, &state.keys, TokenType::RefreshToken).await?;
        let response = LoginResponse {
            message: "User created successfully".to_string(),
            id: user.id,
            role: user.role,
            access_token,
            refresh_token,
        };

        Ok((StatusCode::CREATED, Json(response)))
    }
}
