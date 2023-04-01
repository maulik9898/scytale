#![allow(unused_variables)]
#![allow(unused_imports)]
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::Message;
use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;

use models::jwt::Keys;
use models::user::Client;
use sqlx::migrate::MigrateDatabase;
use sqlx::Sqlite;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};

mod controllers;
mod error;
mod middleware;
mod models;
mod service;
mod utils;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::controllers::admin::AdminController;
use crate::controllers::auth::AuthController;
use crate::controllers::token::TokenController;
use crate::controllers::user::UserController;
use crate::controllers::websocket::WebsocketController;
use crate::middleware::is_admin;
use crate::models::state::AppState;


pub async fn serve(addr: SocketAddr, db_url: &str, secret: &str) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = db_url.to_string();

    if Sqlite::database_exists(&db_url)
        .await
        .expect("unable to check if database exists")
    {
        tracing::debug!("Database exists");
    } else {
        tracing::debug!("Database does not exist");
        Sqlite::create_database(&db_url)
            .await
            .expect("unable to create database");
    }

    let pool = sqlx::SqlitePool::connect(&db_url)
        .await
        .expect("unable to connect to database");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("unable to run migrations");

    let cors = CorsLayer::new().allow_origin(Any);

    let app_state = AppState::new(pool, secret);

    let app_state = Arc::new(Mutex::new(app_state));

    let token_routes = Router::new()
        .route("/token/authenticate", get(TokenController::authenticated))
        .route("/token", post(TokenController::refresh));

    let admin_routes = Router::new()
        .route("/admin", get(AdminController::admin))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            is_admin,
        ));

    let user_route = Router::new().route("/user/client", get(UserController::get_clients));

    let auth_routes = Router::new()
        .route("/register", post(AuthController::register))
        .route("/authenticated", get(AuthController::authenticated))
        .route("/login", post(AuthController::login));

    let websocket_routes = Router::new().route("/ws", get(WebsocketController::ws_handler));

    let app = Router::new()
        .merge(admin_routes)
        .merge(auth_routes)
        .merge(token_routes)
        .merge(websocket_routes)
        .merge(user_route)
        .layer(CorsLayer::permissive())
        // .layer(TraceLayer::new_for_http()
        .with_state(app_state.clone());

    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}

#[cfg(test)]
mod tests {
    use crate::models::state::AppStateType;

    use super::*;
    use sqlx::{Connection, Executor, SqliteConnection};
    use tokio::sync::mpsc;

    async fn create_test_app_state() -> AppStateType {
        let db_url = "sqlite::memory:";
        let pool = sqlx::SqlitePool::connect(&db_url)
            .await
            .expect("unable to connect to in-memory database");
        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("unable to run migrations");

        let secret = "test_secret";
        let app_state = AppState::new(pool, secret);
        Arc::new(Mutex::new(app_state))
    }

    #[tokio::test]
    async fn test_add_delete_client() {
        let app_state = create_test_app_state().await;
        let mut app_state = app_state.lock().await;

        let (tx, _) = mpsc::unbounded_channel::<Message>();
        let uid = 1;
        let client_id = "client_id".to_string();
        let client_name = "client_name".to_string();
        let client = Client::new(client_id.clone(), uid, client_name.clone(), None);

        app_state.add_client(uid, client, tx.clone()).await;

        assert_eq!(app_state.users.len(), 1);
        assert_eq!(app_state.users.get(&uid).unwrap().len(), 1);

        let stored_client = app_state.get_client(&uid, &client_id).await.unwrap();
        assert_eq!(stored_client.0.id, client_id);
        assert_eq!(stored_client.0.name, client_name);

        app_state.delete_client(&uid, &client_id).await;
        assert_eq!(app_state.users.len(), 1);
        assert_eq!(app_state.users.get(&uid).unwrap().len(), 0);
    }
}
