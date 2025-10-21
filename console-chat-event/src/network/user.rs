use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserStatus {
    pub is_new: bool,
    pub ttl: u32,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BetterUser;
