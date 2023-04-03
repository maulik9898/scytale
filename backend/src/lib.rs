#![allow(unused_variables)]
#![allow(unused_imports)]
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::Message;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::routing::get_service;
use axum::routing::{get, post, Route};
use axum::Router;

use error::AppError;
use models::jwt::Keys;
use models::user::{Client, Role, UserCreate};
use service::user::{chech_or_add_admin, create_user, get_user_by_email};
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};

mod controllers;
mod error;
mod middleware;
mod models;
mod service;
mod utils;

use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utils::{get_default_router, get_state, setup_db};

use crate::controllers::admin::AdminController;
use crate::controllers::auth::AuthController;
use crate::controllers::token::TokenController;
use crate::controllers::user::UserController;
use crate::controllers::websocket::WebsocketController;
use crate::middleware::is_admin;
use crate::models::state::AppState;

pub struct Scytale {
    pub addr: SocketAddr,
    pub db_url: String,
    pub jwt_secret: String,
    pub admin_email: String,
    pub admin_password: String,
    pub admin_name: String,
}

impl Scytale {
    pub fn new(
        addr: SocketAddr,
        db_url: String,
        jwt_secret: String,
        admin_email: String,
        admin_password: String,
        admin_name: String,
    ) -> Self {
        Self {
            addr,
            db_url,
            jwt_secret,
            admin_email,
            admin_password,
            admin_name,
        }
    }

    pub async fn start(&mut self) {
        tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
        let pool = setup_db(&self.db_url).await;
        chech_or_add_admin(
            &pool,
            &self.admin_email,
            &self.admin_password,
            &self.admin_name,
        )
        .await;
        let state = get_state(pool, &self.jwt_secret);

        let app = get_default_router(state.clone());

        tracing::info!("Starting server at {}", self.addr);

        axum::Server::bind(&self.addr)
            .serve(app.into_make_service())
            .await
            .expect("Failed to start server");
    }
}

#[cfg(test)]
mod routes {
    use crate::models::{
        auth::LoginResponse,
        user::{Role, UserCreate, UserLogin},
    };

    use super::*;
    use axum::{http::StatusCode, Json};
    use axum_test_helper::{RequestBuilder, TestClient};
    use sqlx::SqlitePool;

    const ADMIN_EMAIL: &str = "maulikp";
    const ADMIN_PASSWORD: &str = "password";
    const ADMIN_NAME: &str = "Maulik Patel";

    async fn setup_db() -> SqlitePool {
        let db_url = "sqlite::memory:".to_string();

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

        pool
    }

    async fn setup_client(pool: SqlitePool) -> TestClient {
        let app_state = AppState::new(pool, "secret");

        let app_state = Arc::new(Mutex::new(app_state));

        let router = get_default_router(app_state);
        let client = TestClient::new(router);
        client
    }

    async fn admin_login(client: &TestClient) -> String {
        let user = UserLogin {
            email: ADMIN_EMAIL.to_string(),
            password: ADMIN_PASSWORD.to_string(),
        };
        let res = client.post("/api/login").json(&user).send().await;
        assert_eq!(res.status(), StatusCode::OK);
        let access_token = res.json::<LoginResponse>().await.access_token;
        let h = format!("Bearer {}", access_token);
        h
    }

    //TODO: support for custom Role
    async fn create_user() -> UserCreate {
        let create_user = UserCreate {
            email: "maulikp1".to_string(),
            password: "password".to_string(),
            name: "Maulik Patel".to_string(),
            role: Role::USER,
        };
        create_user
    }

    #[tokio::test]
    async fn test_unauthenticated() {
        let pool = setup_db().await;
        let client = setup_client(pool).await;
        let res = client.get("/api/authenticated").send().await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn test_authenticated() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = UserCreate {
            email: "maulikp1".to_string(),
            password: "password".to_string(),
            name: "Maulik Patel".to_string(),
            role: Role::USER,
        };
        let res = client
            .post("/api/admin/register")
            .header("Authorization", h)
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let access_token = res.json::<LoginResponse>().await.access_token;
        let h = format!("Bearer {}", access_token);
        let res = client
            .get("/api/authenticated")
            .header("Authorization", h)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_login() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = create_user().await;
        let res = client
            .post("/api/admin/register")
            .json(&create_user)
            .header("Authorization", h)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let user = UserLogin {
            email: create_user.email.to_string(),
            password: create_user.password.to_string(),
        };
        let res = client.post("/api/login").json(&user).send().await;
        assert_eq!(res.status(), StatusCode::OK);
        let access_token = res.json::<LoginResponse>().await.access_token;
        let h = format!("Bearer {}", access_token);
        let res = client
            .get("/api/authenticated")
            .header("Authorization", h)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_register() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = create_user().await;
        let res = client
            .post("/api/admin/register")
            .json(&create_user)
            .header("Authorization", h)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_register_no_admin() {
        let pool = setup_db().await;
        let client = setup_client(pool).await;
        let create_user = create_user().await;
        let res = client
            .post("/api/admin/register")
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_register_no_password() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = UserCreate {
            email: "maulikp1".to_string(),
            password: "".to_string(),
            name: "Maulik Patel".to_string(),
            role: Role::USER,
        };
        let res = client
            .post("/api/admin/register")
            .header("Authorization", &h)
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_register_no_email() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = UserCreate {
            email: "".to_string(),
            password: "password".to_string(),
            name: "Maulik Patel".to_string(),
            role: Role::USER,
        };
        let res = client
            .post("/api/admin/register")
            .header("Authorization", &h)
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_register_no_name() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = UserCreate {
            email: "maulikp1".to_string(),
            password: "password".to_string(),
            name: "".to_string(),
            role: Role::USER,
        };
        let res = client
            .post("/api/admin/register")
            .header("Authorization", &h)
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_register_duplicate() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;
        let h = admin_login(&client).await;
        let create_user = create_user().await;
        let res = client
            .post("/api/admin/register")
            .header("Authorization", &h)
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let res = client
            .post("/api/admin/register")
            .header("Authorization", &h)
            .json(&create_user)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_admin() {
        let pool = setup_db().await;
        chech_or_add_admin(&pool, ADMIN_EMAIL, ADMIN_PASSWORD, ADMIN_NAME).await;
        let client = setup_client(pool).await;

        let h = admin_login(&client).await;
        let res = client
            .get("/api/admin")
            .header("Authorization", h)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}
