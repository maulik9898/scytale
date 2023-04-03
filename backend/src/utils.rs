use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use bcrypt::{BcryptError, DEFAULT_COST};
use jsonwebtoken::{Header, Validation};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

#[cfg(feature = "webapp")]
use crate::controllers::spa::SpaController;

use crate::{
    controllers::{
        admin::AdminController, auth::AuthController, token::TokenController, user::UserController,
        websocket::WebsocketController,
    },
    error::AppError,
    middleware::is_admin,
    models::{
        jwt::{Claims, Keys, TokenType},
        state::AppState,
        user::UserEntity,
    },
};

// consume password value to make it unusable
pub async fn encode_token(
    user: &UserEntity,
    keys: &Keys,
    token_type: TokenType,
) -> Result<String, AppError> {
    let token = jsonwebtoken::encode::<Claims>(
        &Header::default(),
        &Claims::from(user, token_type),
        &keys.encoding,
    )
    .map_err(|err| {
        tracing::error!("Error encoding token: {:?}", err);
        AppError::InternalServerError
    })?;

    Ok(token)
}

pub async fn decode_token(token: &str, keys: &Keys) -> Result<Claims, AppError> {
    let claims = jsonwebtoken::decode::<Claims>(token, &keys.decoding, &Validation::default())
        .map_err(|err| {
            tracing::error!("Error decoding token: {:?}", err);
            AppError::InvalidToken
        })?;

    Ok(claims.claims)
}

pub async fn setup_db(db_url: &str) -> SqlitePool {
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

pub fn get_state(pool: SqlitePool, secret: &str) -> Arc<Mutex<AppState>> {
    let app_state = AppState::new(pool, secret);
    let app_state = Arc::new(Mutex::new(app_state));
    app_state
}

pub fn get_default_router(state: Arc<Mutex<AppState>>) -> Router {
    let token_routes = Router::new()
        .route("/token/authenticate", get(TokenController::authenticated))
        .route("/token", post(TokenController::refresh));

    let admin_routes = Router::new()
        .route("/admin", get(AdminController::admin))
        .route("/admin/register", post(AdminController::register))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            is_admin,
        ));

    let user_route = Router::new().route("/user/client", get(UserController::get_clients));

    let auth_routes = Router::new()
        .route("/authenticated", get(AuthController::authenticated))
        .route("/login", post(AuthController::login));

    let websocket_routes = Router::new().route("/ws", get(WebsocketController::ws_handler));

    let app_router = Router::new()
        .merge(admin_routes)
        .merge(auth_routes)
        .merge(token_routes)
        .merge(websocket_routes)
        .merge(user_route);

    #[cfg(feature = "webapp")]
    let app = Router::new()
        .nest("/api", app_router)
        .route("/index.html", get(SpaController::index_handler))
        .route("/*path", get(SpaController::static_handler))
        .fallback_service(get(SpaController::index_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state.clone());

    #[cfg(not(feature = "webapp"))]
    let app = Router::new()
        .nest("/api", app_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state.clone());

    app
}
