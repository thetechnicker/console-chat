use chrono::{DateTime, Utc};
use openapi::models::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Message {
    pub content: String,
    pub user: Option<UserPublic>,
    pub send_at: Option<DateTime<Utc>>,
    pub is_me: bool,
}
