use std::sync::Arc;

use axum::{extract::State, Json};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::{
    error::AppError,
    models::user::{Client, UserEntity},
    AppState,
};

pub struct UserController {}

impl UserController {
    pub async fn get_clients(
        State(state): State<Arc<Mutex<AppState>>>,
        user: UserEntity,
    ) -> Result<Json<Vec<Client>>, AppError> {
        let state = state.lock().await;
        let clients = state.users.get(&user.id);
        if let Some(clients) = clients {
            let c = clients
                .clone()
                .iter()
                .map(|(_, (client, _))| client.clone())
                .collect::<Vec<_>>();
            return Ok(Json(c));
        }
        Ok(Json([].to_vec()))
    }
}
