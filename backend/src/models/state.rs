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
        if let Some(user) = self.users.get_mut(&uid) {
            user.insert(client.id.clone(), (client, tx));
        } else {
            let mut user = HashMap::new();
            user.insert(client.id.clone(), (client, tx));
            self.users.insert(uid, user);
        }
    }

    pub async fn delete_client(&mut self, uid: &i64, client_id: &str) {
        self.users.get_mut(uid).map_or_else(
            || {
                tracing::debug!("User not found");
            },
            |user| {
                user.remove(client_id);
            },
        );
    }
}
