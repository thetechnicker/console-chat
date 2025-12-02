use super::messages::*;
use crate::network::encryption;
use crate::network::error::NetworkError;
use lazy_static::lazy_static;
use serde::{self, Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub const ASYM_KEY_CHECK: &str = "sending-key";
pub const ID_FIELD: &str = "current-id";

lazy_static! {
    pub static ref ID: Uuid = Uuid::new_v4();
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
                message_type: MessageType::PlainText,
                text: String::from(msg),
                data: Some(HashMap::from([(
                    ID_FIELD.to_string(),
                    serde_json::json!(ID.clone()),
                )])),
            },
        }
    }
    pub fn encrypted(msg: encryption::EncryptedMessage) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::EncryptedText,
                text: msg.0,
                data: Some(HashMap::from([
                    (
                        "nonce".to_owned(),
                        serde_json::Value::from(encryption::to_base64(&msg.1)),
                    ),
                    (ID_FIELD.to_string(), serde_json::json!(ID.clone())),
                ])),
            },
        }
    }
    pub fn key_request(publickey: encryption::PublicKey) -> Self {
        Self {
            base: BaseMessage {
                message_type: MessageType::KeyRequest,
                text: "requesting Key".to_owned(),
                data: Some(HashMap::from([
                    (
                        "key".to_owned(),
                        serde_json::Value::from(encryption::to_base64(&publickey)),
                    ),
                    (ID_FIELD.to_string(), serde_json::json!(ID.clone())),
                ])),
            },
        }
    }

    pub fn send_key(
        symetric_key: &encryption::SymetricKey,
        key_pair: &encryption::KeyPair,
        public_key: encryption::PublicKey,
    ) -> Result<Self, NetworkError> {
        let key_str = encryption::to_base64(&symetric_key.to_vec());
        let self_public_key = encryption::to_base64(&key_pair.public_key());
        let encrypted_key = encryption::encrypt_asym(&key_str, key_pair, public_key)?;
        let check_msg = encryption::encrypt_asym(ASYM_KEY_CHECK, key_pair, public_key)?;
        Ok(Self {
            base: BaseMessage {
                message_type: MessageType::Key,
                text: check_msg.0,
                data: Some(HashMap::from([
                    (
                        "nonce".to_owned(),
                        serde_json::Value::from(encryption::to_base64(&check_msg.1)),
                    ),
                    (
                        "public_key".to_owned(),
                        serde_json::Value::from(self_public_key),
                    ),
                    ("key".to_owned(), serde_json::Value::from(encrypted_key.0)),
                    (
                        "key_nonce".to_owned(),
                        serde_json::Value::from(encryption::to_base64(&encrypted_key.1)),
                    ),
                    (ID_FIELD.to_string(), serde_json::json!(ID.clone())),
                ])),
            },
        })
    }
}
