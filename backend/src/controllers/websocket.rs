use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};

use tracing::instrument::WithSubscriber;

use crate::{
    error::AppError,
    models::{
        user::{Client, UserEntity},
        websocket::WsParam,
    },
    utils::decode_token,
    AppState,
};

pub struct WebsocketController {}

impl WebsocketController {
    pub async fn ws_handler(
        Query(ws_para): Query<WsParam>,
        State(state): State<Arc<Mutex<AppState>>>,
        ws: WebSocketUpgrade,
    ) -> Result<impl IntoResponse, AppError> {
        let u_state = state.clone();
        let u_state = u_state.lock().await;
        let claim = decode_token(&ws_para.token, &u_state.keys).await?;

        let client = Client::new(ws_para.id.clone(), claim.id, ws_para.name.clone(), None);
        tracing::debug!("New WebSocket Connection: {:?}", client);
        if let Some(user) = u_state.users.get(&client.user_id) {
            if let Some((_, tx)) = user.get(&client.id) {
                let _ = tx.send(Message::Text("You are already connected".to_owned()));
                return Err(AppError::AlreadyConnected);
            }
        }
        drop(u_state);

        Ok(ws.on_upgrade(|socket| Self::handle_socket(socket, client, state)))
    }

    pub async fn handle_socket(socket: WebSocket, client: Client, state: Arc<Mutex<AppState>>) {
        tracing::debug!("New WebSocket Upgraded: {:?}", client);

        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        let (mut sender, mut receiver) = socket.split();

        let u_state = state.clone();
        let client_id = client.id.clone();
        let uid = client.user_id;
        let client_id2 = client.id.clone();

        {
            u_state.lock().await.add_client(uid, client, tx).await;
        }

        let mut send_task = tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = sender.send(msg).await {
                    tracing::error!("Error sending message: {}", e);
                    break;
                }
            }
        });

        let recv_state = state.clone();
        let mut recv_task = tokio::task::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Err(_) =
                    Self::handle_message(msg, recv_state.clone(), &uid, &client_id).await
                {
                    tracing::debug!("Error handling message");
                    break;
                }
            }
        });
        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        };

        let state = state.clone();
        let mut state = state.lock().await;
        state.delete_client(&uid, &client_id2).await;
    }

    pub async fn handle_message(
        msg: Message,
        state: Arc<Mutex<AppState>>,
        uid: &i64,
        client_id: &str,
    ) -> Result<(), ()> {
        match msg {
            Message::Ping(msg) => {
                tracing::debug!("Received ping: {:?}", msg);
                let mut state = state.lock().await;
                if let Some((client, tx)) = state.get_client(uid, client_id).await {
                    if let Err(e) = tx.send(Message::Pong(msg)) {
                        tracing::error!("Error sending message: {}", e);
                    }
                }
                Ok(())
            }
            Message::Pong(msg) => {
                tracing::debug!("Received pong: {:?}", msg);
                Ok(())
            }
            Message::Close(_) => Err(()),
            Message::Binary(_) => {
                tracing::debug!("Received binary");
                Ok(())
            }
            Message::Text(text) => {
                tracing::debug!("Received text: {}", text);
                let mut state = state.lock().await;

                let user = state.users.get_mut(uid);
                if let Some(user) = user {
                    for (_, (client, tx)) in user.iter_mut() {
                        if client.id == client_id.clone() {
                            continue;
                        }
                        if let Err(e) = tx.send(Message::Text(text.clone())) {
                            tracing::error!("Error sending message: {}", e);
                            break;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}
