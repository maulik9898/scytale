use std::{collections::HashMap, sync::Arc};

use axum::extract::ws::Message;
use tokio::sync::{mpsc::UnboundedSender, Mutex};

use super::{jwt::Keys, user::Client};

pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub keys: Keys,
    pub users: HashMap<i64, HashMap<String, (Client, UnboundedSender<Message>)>>,
}

pub type AppStateType = Arc<Mutex<AppState>>;

impl AppState {
    pub fn new(pool: sqlx::SqlitePool, secret: &str) -> Self {
        Self {
            pool,
            keys: Keys::new(secret),
            users: HashMap::new(),
        }
    }

    pub async fn get_client(
        &mut self,
        uid: &i64,
        client_id: &str,
    ) -> Option<&mut (Client, UnboundedSender<Message>)> {
        if let Some(user) = self.users.get_mut(uid) {
            user.get_mut(client_id)
        } else {
            None
        }
    }

    pub async fn add_client(&mut self, uid: i64, client: Client, tx: UnboundedSender<Message>) {
        match self.users.get_mut(&uid) {
            Some(user) => {
                user.insert(client.id.clone(), (client, tx));
            }
            None => {
                let mut user = HashMap::new();
                user.insert(client.id.clone(), (client, tx));
                self.users.insert(uid, user);
            }
        };
    }

    pub async fn delete_client(&mut self, uid: &i64, client_id: &str) {
        if let Some(user) = self.users.get_mut(uid) {
            user.remove(client_id);
        } else {
            tracing::error!("User not found");
        }
    }
}
