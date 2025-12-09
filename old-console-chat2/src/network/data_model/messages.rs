use super::message_data::*;
use super::user::*;
use crate::network::Result;
use crate::network::encryption::*;
use crate::network::error::*;
//use color_eyre::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub const ASYM_KEY_CHECK: &str = "sending-key";
pub const ID_FIELD: &str = "current-id";
pub const ENCRYPTION_DATA_FIELD: &str = "ENCRYPTION_DATA_FIELD";

lazy_static! {
    pub static ref ID: Uuid = Uuid::new_v4();
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum MessageType {
    #[serde(rename = "PLAIN-TEXT")]
    PlainText,
    #[serde(rename = "ENCRYPTED-TEXT")]
    EncryptedText,
    Join,
    Leave,
    System,
    Key,
    #[serde(rename = "KEY-REQUEST")]
    KeyRequest,
}

#[derive(Debug)]
pub enum DecryptedMessage {
    Message(Message),
    NoKey(Message),
    KeyRequest(PublicKey, Message),
    KeyResponce(SymetricKey, Message),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub text: String,
    pub data: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<PublicUser>,
}

impl PartialEq for Message {
    /// Always asume two messages are different
    fn eq(&self, _: &Message) -> bool {
        false
    }
}
impl Eq for Message {}

impl Message {
    pub fn new(msg: &str) -> Self {
        Self {
            message_type: MessageType::PlainText,
            text: String::from(msg),
            data: Some(HashMap::from([(
                ID_FIELD.to_string(),
                serde_json::json!(ID.clone()),
            )])),
            user: None,
        }
    }

    pub fn get_data<T>(&self, key: impl Into<String>) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(if let Some(ref data) = self.data {
            match data.get(&key.into()) {
                None => None,
                Some(elem) => Some(serde_json::from_value(elem.to_owned())?),
            }
        } else {
            None
        })
    }
    pub fn is_mine(&self) -> bool {
        if let Ok(Some(id)) = self.get_data::<uuid::Uuid>(ID_FIELD) {
            id == *ID
        } else {
            false
        }
    }

    pub fn encrypted(msg: EncryptedMessageBase64) -> Self {
        Self {
            message_type: MessageType::EncryptedText,
            text: msg.0,
            data: Some(HashMap::from([
                (
                    ENCRYPTION_DATA_FIELD.to_string(),
                    serde_json::json!(EncryptedMessageData::new(msg.1)),
                ),
                (ID_FIELD.to_string(), serde_json::json!(ID.clone())),
            ])),
            user: None,
        }
    }
    pub fn key_request(publickey: PublicKey) -> Self {
        Self {
            message_type: MessageType::KeyRequest,
            text: "requesting Key".to_owned(),
            data: Some(HashMap::from([
                (
                    ENCRYPTION_DATA_FIELD.to_string(),
                    serde_json::json!(HexVec::from(publickey)),
                ),
                (ID_FIELD.to_string(), serde_json::json!(ID.clone())),
            ])),
            user: None,
        }
    }

    pub fn send_key(
        symetric_key: &SymetricKey,
        key_pair: &KeyPair,
        public_key: PublicKey,
    ) -> Result<Self, NetworkError> {
        Ok(Self {
            message_type: MessageType::Key,
            text: "Public Key".to_string(),
            data: Some(HashMap::from([
                (
                    ENCRYPTION_DATA_FIELD.to_string(),
                    serde_json::json!(EncryptedKeyMessageData::new(
                        ASYM_KEY_CHECK,
                        symetric_key,
                        key_pair,
                        public_key,
                    )?),
                ),
                (ID_FIELD.to_string(), serde_json::json!(ID.clone())),
            ])),
            user: None,
        })
    }

    pub fn decrypt(
        mut self,
        key_pair: &KeyPair,
        symetric_key: Option<&SymetricKey>,
    ) -> Result<DecryptedMessage> {
        Ok(match self.message_type {
            MessageType::EncryptedText => match symetric_key {
                Some(symetric_key) => {
                    let data: EncryptedMessageData = self
                        .get_data(ENCRYPTION_DATA_FIELD)?
                        .ok_or(NetworkError::MissingEncryptionData)?;
                    let msg = data.decode(&self.text, symetric_key)?;
                    self.text = msg;
                    DecryptedMessage::Message(self)
                }
                None => DecryptedMessage::NoKey(self),
            },
            MessageType::Key => {
                let data: EncryptedKeyMessageData = self
                    .get_data(ENCRYPTION_DATA_FIELD)?
                    .ok_or(NetworkError::MissingEncryptionData)?;
                let key = data.get_key(ASYM_KEY_CHECK, key_pair)?;
                DecryptedMessage::KeyResponce(key, self)
            }
            MessageType::KeyRequest => {
                let data: HexVec = self
                    .get_data(ENCRYPTION_DATA_FIELD)?
                    .ok_or(NetworkError::MissingEncryptionData)?;
                DecryptedMessage::KeyRequest(data.into(), self)
            }
            _ => DecryptedMessage::Message(self),
        })
    }
}
