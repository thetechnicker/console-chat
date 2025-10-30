use super::messages::*;
use crate::network::encryption;
use base64::{Engine as _, engine::general_purpose};
use serde::{self, Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl ClientMessage {
    pub fn new(msg: &str) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::PlainText,
                text: String::from(msg),
                data: None,
            },
        }
    }
    pub fn encrypted(msg: encryption::EncryptedMessage) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::EncryptedText,
                text: String::from(msg.0),
                data: Some(HashMap::from([(
                    "nonce".to_owned(),
                    serde_json::Value::from(general_purpose::STANDARD.encode(msg.1)),
                )])),
            },
        }
    }
}
