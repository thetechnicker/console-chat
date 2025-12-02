use super::client_messages::{ID, ID_FIELD};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub text: String,
    pub data: Option<HashMap<String, serde_json::Value>>,
}

impl PartialEq for BaseMessage {
    /// Always asume two messages are different
    fn eq(&self, _: &BaseMessage) -> bool {
        false
    }
}
impl Eq for BaseMessage {}

impl BaseMessage {
    pub fn get_data_str(&self, key: impl Into<String>) -> Option<String> {
        if let Some(ref data) = self.data {
            match data.get(&key.into()) {
                None => None,
                Some(elem) => {
                    elem.as_str().map(|str| str.to_string())
                }
            }
        } else {
            None
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
}
