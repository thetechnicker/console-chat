use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub text: String,
    pub data: Option<HashMap<String, serde_json::Value>>,
}
