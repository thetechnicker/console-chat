use super::messages::*;
use crate::network::encryption;
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl ClientMessage {
    pub fn new(msg: &str) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::EncryptedText,
                text: encryption::encrypt_text(msg, &encryption::KEY),
                data: None,
            },
        }
    }
}
