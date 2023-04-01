#![allow(dead_code)]
use std::borrow::{Borrow, Cow};

use axum::{http::StatusCode, response::IntoResponse, Json};
use bcrypt::BcryptError;
use serde_json::json;
use sqlx::{error::DatabaseError, sqlite::SqliteError};

#[derive(Debug)]
pub enum AppError {
    InvalidToken,
    MissingToken,
    NotRefreshToken,
    NotAccessToken,
    InsufficientPermission,
    WrongCredential,
    MissingCredential,
    TokenCreation,
    InternalServerError,
    UserDoesNotExist,
    UserAlreadyExits,
    AlreadyConnected,
    DatabaseError(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "an internal server error occured".to_string(),
            ),
            Self::InvalidToken => (StatusCode::BAD_REQUEST, "invalid token".to_string()),
            Self::MissingToken => (StatusCode::BAD_REQUEST, "missing token".to_string()),
            Self::MissingCredential => (StatusCode::BAD_REQUEST, "missing credential".to_string()),
            Self::TokenCreation => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create token".to_string(),
            ),
            Self::WrongCredential => (StatusCode::UNAUTHORIZED, "wrong credentials".to_string()),
            Self::UserDoesNotExist => (StatusCode::UNAUTHORIZED, "User does not exist".to_string()),
            Self::UserAlreadyExits => (StatusCode::BAD_REQUEST, "User already exists".to_string()),
            Self::DatabaseError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Self::InsufficientPermission => {
                (StatusCode::FORBIDDEN, "insufficient permission".to_string())
            }
            Self::NotAccessToken => (StatusCode::BAD_REQUEST, "not an access token".to_string()),
            Self::NotRefreshToken => (StatusCode::BAD_REQUEST, "not a refresh token".to_string()),
            Self::AlreadyConnected => (StatusCode::BAD_REQUEST, "already connected".to_string()),
        };
        (status, Json(json!({ "error": err_msg }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::DatabaseError(error)
    }
}

impl From<BcryptError> for AppError {
    fn from(error: BcryptError) -> Self {
        Self::InternalServerError
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for AppError {
    fn from(error: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::InternalServerError
    }
}
