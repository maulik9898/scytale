use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WsParam {
    pub id: String,
    pub name: String,
    pub token: String,
}
