#![allow(dead_code)]
use chrono::Utc;
use sqlx::SqlitePool;

use crate::{
    error::AppError,
    models::user::{Role, UserCreate, UserEntity},
};
use sqlx::{query, query_as};

pub async fn create_user(
    pool: &SqlitePool,
    user: &mut UserCreate,
    role: Role,
) -> Result<UserEntity, AppError> {
    user.hash_password()?;
    let email = &user.email;
    let name = &user.name;
    let password = &user.password;
    let record = query_as!(
        UserEntity,
        r#"INSERT INTO users (email, name, password, role) 
            VALUES (?, ?, ?, ?) 
            RETURNING id as "id!: i64", name as "name!:String", email as "email!: String", password as "password!: String", role as "role!: Role" "#,
        email,
        name,
        password,
        role
    ).fetch_one(pool)
    .await?;

    Ok(record)
}

pub async fn get_user_by_id(pool: &SqlitePool, id: i64) -> Result<UserEntity, AppError> {
    let user = query_as!(
        UserEntity,
        r#"SELECT id, name, email, password , role as "role!: Role" FROM users WHERE id = ?"#,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(|err| AppError::UserDoesNotExist)?;

    Ok(user)
}

pub async fn get_user_by_id_email(
    pool: &SqlitePool,
    id: i64,
    email: &str,
) -> Result<UserEntity, AppError> {
    let user = query_as!(
        UserEntity,
        r#"SELECT id, name, email, password, role as "role!: Role" FROM users WHERE id = ? AND email = ?"#,
        id, email
    )
    .fetch_one(pool)
    .await
    .map_err(|err| AppError::UserDoesNotExist)?;

    Ok(user)
}

pub async fn get_user_by_email(
    pool: &SqlitePool,
    email: &str,
) -> Result<Option<UserEntity>, sqlx::Error> {
    let user = query_as!(
        UserEntity,
        r#"SELECT 
            id as "id!: i64", 
            name as "name!:String", 
            email as "email!: String", 
            password as "password!: String", 
            role as "role!: Role"
        FROM users WHERE email = ?"#,
        email
    )
    .fetch_optional(pool)
    .await?;
    Ok(user)
}
