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

impl BaseMessage {
    pub fn get_data_str(&self, key: impl Into<String>) -> Option<String> {
        if let Some(ref data) = self.data {
            match data.get(&key.into()).clone() {
                None => None,
                Some(elem) => {
                    if let Some(str) = elem.as_str() {
                        Some(str.to_string())
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        }
    }
}
