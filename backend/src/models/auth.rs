use serde::{Deserialize, Serialize};

use super::user::Role;
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub id: i64,
    pub message: String,
    pub role: Role,
    pub access_token: String,
    pub refresh_token: String,
}
