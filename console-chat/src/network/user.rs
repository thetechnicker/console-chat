use base64::{Engine as _, engine::general_purpose};
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
    Key,
}

fn encrypt_text<S>(text: &str, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let bytes: &[u8] = text.as_bytes();
    let mut bytes: Vec<u8> = Vec::from(bytes);
    // TODO: Replace with actual encryption
    for b in bytes.iter_mut() {
        *b = *b ^ 0xff;
    }
    let encoded = general_purpose::STANDARD.encode(bytes);
    s.serialize_str(&encoded)
}

fn decrypt_bytes<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    match general_purpose::STANDARD.decode(s) {
        Err(e) => Err(serde::de::Error::custom(e)),
        Ok(mut bytes) => {
            // TODO: Replace with actual decryption
            for b in bytes.iter_mut() {
                *b = *b ^ 0xff;
            }
            match str::from_utf8(&bytes) {
                Ok(string) => Ok(String::from(string)),
                Err(e) => Err(serde::de::Error::custom(e)),
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    #[serde(serialize_with = "encrypt_text", deserialize_with = "decrypt_bytes")]
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
