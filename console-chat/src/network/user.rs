use serde::{self, Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MessageType {
    Text,
    Join,
    Leave,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub text: String,
    pub data: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl ClientMessage {
    pub fn new(msg: &str) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::Text,
                text: msg.to_string(),
                data: None,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub user: Option<PublicUser>,
}
impl ServerMessage {
    pub fn new(msg: &str) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::Text,
                text: msg.to_string(),
                data: None,
            },
            user: None,
        }
    }
}
