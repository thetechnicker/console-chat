use alkali::asymmetric::cipher::{self};
use alkali::mem::ReadOnly;
use alkali::symmetric::cipher as symetric_cipher;
use alkali::symmetric::cipher::Key;
use base64::{Engine as _, engine::general_purpose};
use derive_deref::{Deref, DerefMut};
use openapi::apis::configuration::Configuration;
use openapi::apis::rooms_api;
use openapi::models::Content;
use openapi::models::Encrypted;
use openapi::models::MessageSend;
use openapi::models::MessageType;
use openapi::models::Plaintext;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::debug;

type Result<T, E = error::NetworkError> = std::result::Result<T, E>;

#[derive(Deref, DerefMut)]
pub(crate) struct Keypair(pub cipher::Keypair);
impl From<cipher::Keypair> for Keypair {
    fn from(c: cipher::Keypair) -> Keypair {
        Keypair(c)
    }
}

impl std::fmt::Debug for Keypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keypair")
            .field("private_key", &"*".repeat(self.private_key.len()))
            .field("public_key", &self.public_key)
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct Keys {
    pub symetric_keys: RwLock<HashMap<String, Key<ReadOnly>>>, // Protect with Mutex
    pub asymetric_keys: Option<Keypair>,
}

 fn to_base64(arg: &[u8]) -> String {
    general_purpose::STANDARD.encode(arg)
}

 fn from_base64(arg: &str) -> Result<Vec<u8>> {
    Ok(general_purpose::STANDARD.decode(arg)?)
}

#[tracing::instrument]
async fn send_message_from_content(
    conf: &Configuration,
    room: &str,
    is_static: bool,
    message_content: Content,
) -> Result<()> {
    let r#type = match message_content {
        Content::Encrypted(_) => MessageType::Encrypted,
        Content::KeyResponse(_) => MessageType::KeyResponse,
        Content::KeyRequest(_) => MessageType::KeyRequest,
        Content::Plaintext(_) => MessageType::Plaintext,
        Content::System(_) => MessageType::System,
    };
    let now: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(std::time::SystemTime::now());
    let msg = MessageSend {
        r#type: Some(r#type),
        content: Some(message_content),
        send_at: Some(now.to_rfc3339()),
        data: Some(None),
    };
    if is_static {
        rooms_api::rooms_send_static(conf, room, msg).await?;
    } else {
        rooms_api::rooms_send(conf, room, msg).await?;
    }
    Ok(())
}

async fn send_message(
    conf: &Configuration,
    room: &str,
    is_static: bool,
    message_content: &str,
    key: Option<&Key<impl alkali::mem::MprotectReadable>>,
) -> Result<()> {
    if let Some(key) = key {
        debug!("Sending encrypted Message");
        let plaintext = message_content.as_bytes();
        let mut ciphertext = vec![0u8; plaintext.len() + cipher::MAC_LENGTH];
        let (_, nonce) = symetric_cipher::encrypt(plaintext, key, None, &mut ciphertext)?;

        let message = Content::Encrypted(Encrypted::new(to_base64(&ciphertext), to_base64(&nonce)));
        send_message_from_content(conf, room, is_static, message).await?;
        Ok(())
    } else {
        debug!("Sending plaintext Message");
        let message = Content::Plaintext(Plaintext::new(message_content.to_owned()));
        send_message_from_content(conf, room, is_static, message).await?;
        Ok(())
    }
}

pub(crate) mod error;
pub(crate) mod listen_thread;
pub(crate) mod message;
pub(crate) mod misc_thread;
pub(crate) mod network;

pub(crate) use message::*;
