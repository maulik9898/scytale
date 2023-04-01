use std::sync::Arc;

use axum::{extract::State, Json};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::{error::AppError, models::user::UserEntity, AppState};

pub struct AdminController {}

impl AdminController {
    pub async fn admin(user: UserEntity) -> Result<Json<Value>, AppError> {
        Ok(Json(json!(user)))
    }
}
