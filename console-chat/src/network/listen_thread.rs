use super::Keys;
use super::Message;
use super::error::NetworkError;
use super::from_base64;
use super::send_message_from_content;
use super::to_base64;
use crate::action::Action;
use crate::network::Result;
use alkali::asymmetric::cipher::{self, PUBLIC_KEY_LENGTH, PublicKey};
use alkali::mem::ReadOnly;
use alkali::symmetric::cipher::{self as symetric_cipher, Key, NONCE_LENGTH};
use chrono::{DateTime, Utc};
use futures_util::stream::StreamExt;
use openapi::apis::configuration::Configuration;
use openapi::apis::rooms_api;
use openapi::models::Content;
use openapi::models::KeyRequest;
use openapi::models::KeyResponse;
use openapi::models::MessagePublic;
use openapi::models::UserPrivate;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch::Sender;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;

#[derive(Debug)]
pub struct ListenThreadData {
    is_static: bool,
    room: String,
    keys: Arc<Keys>, // Shared ownership via Arc
    //used_key: Option<Key<ReadOnly>>,
    me: Arc<Mutex<Option<UserPrivate>>>,
    signal: Arc<Notify>,
    room_tx: Sender<String>,

    conf: Arc<RwLock<Configuration>>, // Use Arc<Mutex> for shared access
    sender: UnboundedSender<Action>,  // Thread-local; no protection needed
}

impl ListenThreadData {
    pub fn new(
        is_static: bool,
        room: String,
        keys: Arc<Keys>,
        signal: Arc<Notify>,
        room_tx: Sender<String>,
        me: Arc<Mutex<Option<UserPrivate>>>,
        conf: Arc<RwLock<Configuration>>, // Use Arc<Mutex> for shared access
        sender: UnboundedSender<Action>,  // Thread-local; no protection needed
    ) -> Self {
        Self {
            is_static,
            room,
            keys,
            room_tx,
            //used_key: None,
            me,
            signal,
            conf,
            sender,
        }
    }
    #[tracing::instrument]
    pub async fn run(mut self, cancel_token: CancellationToken) -> Result<()> {
        let mut stream = {
            let conf = self.conf.read().await;
            if self.is_static {
                rooms_api::rooms_listen_static(&conf, &self.room).await?
            } else {
                rooms_api::rooms_listen(&conf, &self.room).await?
            }
        };
        let _ = self.sender.send(Action::OpenChat);
        self.room_tx.send_replace(self.room.clone());

        debug!("Starting listening on room: {}", self.room);

        while let Some(Ok(msg)) = stream.next().await {
            debug!("Received message: {:#?}", msg);

            if let reqwest_eventsource::Event::Message(event) = msg {
                match serde_json::from_str::<MessagePublic>(&event.data) {
                    Ok(message) => {
                        debug!("Parsed Content: {:#?}", message);
                        let mut received_message = Message {
                            user: message.sender,
                            is_me: false,
                            send_at: message
                                .send_at
                                .and_then(|send_at| DateTime::<Utc>::from_str(&send_at).ok()),
                            content: Default::default(),
                        };
                        match message.content {
                            Some(content) => match self.handle_content(content).await {
                                Err(err) => {
                                    error!("Failed to handle content: {}", err);
                                    let _ = self.sender.send(Action::Error(err.into()));
                                }
                                Ok(Some(content)) => {
                                    received_message.content = content;
                                    let _ =
                                        self.sender.send(Action::ReceivedMessage(received_message));
                                }
                                Ok(_) => {}
                            },
                            None => {
                                error!("Received message with no content",);
                            }
                        }
                    }
                    Err(e) => {
                        let err: NetworkError = e.into();
                        error!("Failed to parse incoming message: {}", err);
                        let _ = self.sender.send(Action::Error(err.into()));
                    }
                }
            } else {
                error!("Unexpected message type received: {:#?}", msg);
            }
        }
        Ok(())
    }

    async fn handle_content(&mut self, content: Content) -> Result<Option<String>> {
        let key_map = self.keys.symetric_keys.read().await;
        let symetric_key = key_map.get(&self.room);

        match content {
            Content::Encrypted(encrypted) => {
                match symetric_key {
                    Some(key) => {
                        let mut nonce = [0u8; NONCE_LENGTH];
                        let nonce_vec = from_base64(&encrypted.nonce)?;
                        let msg_vec = from_base64(&encrypted.content_base64)?;
                        let mut x = vec![0u8; msg_vec.len() - symetric_cipher::MAC_LENGTH];
                        nonce.copy_from_slice(&nonce_vec);
                        match symetric_cipher::decrypt(&msg_vec, key, &nonce, &mut x) {
                            Ok(_) => {
                                let msg = str::from_utf8(&x)?;
                                debug!("Decrypted message content: {}", msg);
                                return Ok(Some(msg.to_owned()));
                            }
                            Err(e) => {
                                error!("{e}");
                                let _ = self
                                    .sender
                                    .send(Action::Error(NetworkError::from(e).into()));
                                //if KEYS.first.load(Ordering::Relaxed) {
                                //    //   let mut key = KEYS.symetric_key.write().await;
                                //    //   if key.is_none() {
                                //    //       *key = Some(Key::generate()?);
                                //    //   }
                                //} else {
                                //    let key_pair = KEYS.asymetric_key.read().await;
                                //    let msg = KeyRequest::new(to_base64(&key_pair.public_key));
                                //    send_message_from_content(Content::KeyRequest(msg)).await?;
                                //}
                            }
                        }
                    }
                    None => {
                        error!("Symmetric key not found for decryption.");
                    }
                }
            }
            Content::Plaintext(plaintext) => {
                debug!("Received plaintext message: {}", plaintext.content);
                return Ok(Some(plaintext.content));
            }
            Content::System(system_message) => {
                if let Some(asymmetric_key) = self.keys.asymetric_keys.as_ref() {
                    debug!("Received system message: {}", system_message.content);
                    if system_message.online_users >= 1 {
                        debug!("Last to join,requesting Key");
                        let msg = KeyRequest::new(to_base64(&asymmetric_key.public_key));
                        send_message_from_content(
                            &*self.conf.read().await,
                            &self.room,
                            self.is_static,
                            Content::KeyRequest(msg),
                        )
                        .await?;
                    } else {
                        debug!("First to join, generating Key");
                        if symetric_key.is_none() {
                            // TODO: Get KEY
                        }
                    }
                }
                return Ok(Some(system_message.content));
            }
            Content::KeyResponse(_) => {
                //let _key_pair = KEY_PAIR.read().await;
                //let mut _key = SYMETRIC_KEY.write().await;
            }
            Content::KeyRequest(request_content) => {
                if let Some(asymmetric_key) = self.keys.asymetric_keys.as_ref() {
                    if let Some(key) = symetric_key {
                        let mut public_key: PublicKey = [0u8; PUBLIC_KEY_LENGTH];
                        let public_key_vec = from_base64(&request_content.public_key)?;
                        public_key.copy_from_slice(public_key_vec.as_slice());
                        let mut ciphertext = vec![0u8; key.len() + cipher::MAC_LENGTH];
                        let (_, nonce) = asymmetric_key.encrypt(
                            key.as_slice(),
                            &public_key,
                            None,
                            &mut ciphertext,
                        )?;
                        let encrypted_key_str = to_base64(&ciphertext);
                        let my_public_key_str = to_base64(&asymmetric_key.public_key);
                        let nonse_str = to_base64(&nonce);
                        let mut ciphertext = vec![0u8; key.len() + cipher::MAC_LENGTH];
                        symetric_cipher::encrypt("TEST".as_bytes(), key, None, &mut ciphertext)?;
                        let test_msg = to_base64(&ciphertext);
                        let key_response = KeyResponse::new(
                            encrypted_key_str,
                            test_msg,
                            my_public_key_str,
                            nonse_str,
                        );
                        send_message_from_content(
                            &*self.conf.read().await,
                            &self.room,
                            self.is_static,
                            Content::KeyResponse(key_response),
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(None)
    }
}
