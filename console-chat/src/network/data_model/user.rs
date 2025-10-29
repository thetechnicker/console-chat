use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserStatus {
    pub is_new: bool,
    pub ttl: u32,
    pub token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BetterUser {
    pub username: String,
    /// Will always be None
    pub password_hash: Option<String>,
    pub private: bool,
    pub public_data: PublicUser,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicUser {
    pub display_name: String,
}
