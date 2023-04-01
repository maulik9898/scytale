use axum::extract::ws::Message;
use std::{collections::HashMap, fmt};

use argon2::Config;
use jsonwebtoken::{DecodingKey, EncodingKey};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, sqlx::FromRow, sqlx::Type)]
pub struct UserEntity {
    pub id: i64,
    pub email: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct UserCreate {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Option<Role>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct UserLogin {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, sqlx::Type)]
pub enum Role {
    ADMIN,
    #[default]
    USER,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Client {
    pub id: String,
    pub user_id: i64,
    pub name: String,
    pub pub_key: Option<String>,
}

impl Client {
    pub fn new(id: String, user_id: i64, name: String, pub_key: Option<String>) -> Self {
        Self {
            id,
            user_id,
            name,
            pub_key,
        }
    }
}

impl From<String> for Role {
    fn from(role: String) -> Self {
        match role.as_str() {
            "ADMIN" => Self::ADMIN,
            _ => Self::USER,
        }
    }
}

impl From<UserCreate> for UserEntity {
    fn from(request: UserCreate) -> Self {
        Self {
            id: 0,
            email: request.email,
            name: request.name,
            password: request.password,
            role: request.role.unwrap_or_default(),
        }
    }
}

impl UserCreate {
    pub fn hash_password(&mut self) -> Result<(), AppError> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        self.password = argon2::hash_encoded(self.password.as_bytes(), &salt, &config)
            .map_err(|e| AppError::InternalServerError)?;

        Ok(())
    }
}

impl UserEntity {
    pub fn verify_password(&self, password: &[u8]) -> Result<bool, AppError> {
        argon2::verify_encoded(&self.password, password).map_err(|e| AppError::WrongCredential)
    }
}
