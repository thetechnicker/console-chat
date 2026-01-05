use crate::action::Action;
use crate::cli::Cli;
//use crate::error::print_recursive_error;
use alkali::asymmetric::cipher::{self, Keypair, PUBLIC_KEY_LENGTH, PublicKey};
use alkali::mem::FullAccess;
use alkali::symmetric::cipher::{self as symetric_cipher, Key, NONCE_LENGTH};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use color_eyre::eyre::OptionExt;
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use openapi::apis::Error as ApiError;
use openapi::apis::configuration::Configuration;
use openapi::apis::{experimental_api, users_api};
use openapi::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tracing::{debug, error};
pub(crate) mod error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct Message {
    pub content: String,
    pub user: Option<UserPublic>,
    pub send_at: Option<DateTime<Utc>>,
}

type Result<T, E = error::NetworkError> = std::result::Result<T, E>;

pub struct ListenData {
    pub thread: JoinHandle<Result<()>>,
    pub room: Arc<String>,
}

pub struct KeyData {
    pub symetric_key: RwLock<Option<Key<FullAccess>>>,
    pub first: std::sync::atomic::AtomicBool,
    pub key_map: RwLock<HashMap<String, Key<FullAccess>>>,
    pub asymetric_key: RwLock<Keypair>,
}

impl KeyData {
    pub fn new() -> Result<Self> {
        Ok(Self {
            symetric_key: Default::default(),
            asymetric_key: RwLock::new(Keypair::generate()?),
            first: Default::default(),
            key_map: Default::default(),
        })
    }
}

lazy_static! {
    pub static ref CONFIGURATION: Arc<RwLock<Configuration>> =
        Arc::new(RwLock::new(Configuration::new()));
    pub static ref USER: Arc<RwLock<Option<UserPrivate>>> = Arc::new(RwLock::new(None));
    pub static ref USERNAME: Arc<std::sync::RwLock<Option<String>>> =
        Arc::new(std::sync::RwLock::new(None));
    pub static ref LISTEN_TASK: Arc<RwLock<Option<ListenData>>> = Arc::new(RwLock::new(None));
    pub static ref ACTION_TX: Arc<RwLock<UnboundedSender<Action>>> =
        Arc::new(RwLock::new(unbounded_channel().0));
    pub static ref KEYS: Arc<KeyData> = Arc::new(
        KeyData::new()
            .expect("Cannot create Keys for encryption, there is no way to disable this crash.")
    );
}

#[tracing::instrument]
pub async fn init(config: Cli, action_tx: UnboundedSender<Action>) -> Result<()> {
    KEYS.first.store(false, Ordering::Relaxed);
    *ACTION_TX.write().await = action_tx;
    let mut client = CONFIGURATION.write().await;
    if config.accept_invalid_certificate {
        client.client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;
    }
    let response = users_api::users_online(&client, None).await?;
    client.bearer_access_token = Some(response.token.token);
    let user = users_api::users_get_me(&client).await?;
    debug!("{:#?}", user);
    update_user(user).await;
    Ok(())
}

#[tracing::instrument]
pub async fn update_user(new_user: UserPrivate) {
    let mut user = USER.write().await;
    *user = Some(new_user.clone());

    if let Ok(mut user) = USERNAME.write() {
        *user = new_user.username.clone();
    }
}

pub async fn handle_actions(event: Action) -> Result<Option<Action>> {
    match event {
        Action::Leave => {
            let mut task = LISTEN_TASK.write().await;
            if let Some(task) = task.take() {
                task.thread.abort();
            }
        }
        Action::OpenLogin => {
            let me = USER.read().await;
            if let Some(me) = me.as_ref() {
                return Ok(Some(Action::Me(me.clone())));
            }
        }
        Action::PerformLogin(username, password) => {
            login(&username, &password).await?;
            return Ok(Some(Action::OpenHome));
        }
        Action::PerformJoin(room) => {
            join(&room).await?;
        }
        Action::SendMessage(msg) => {
            send_message(&msg).await?;
        }
        _ => {}
    }
    Ok(None)
}

#[tracing::instrument]
async fn join(room: &str) -> Result<()> {
    let mut listen_task = LISTEN_TASK.write().await;
    if listen_task.is_none() {
        let room = Arc::new(room.to_owned());
        let thread_room = room.clone();
        let task = ListenData {
            thread: tokio::task::spawn(async { listen(thread_room).await }),
            room,
        };
        *listen_task = Some(task);
    }
    Ok(())
}

#[tracing::instrument]
async fn listen(room: Arc<String>) -> Result<()> {
    let conf = CONFIGURATION.read().await.clone();
    let mut stream = experimental_api::experimental_listen(&conf, &room).await?;
    let action_tx = ACTION_TX.read().await.clone();
    let _ = action_tx.send(Action::OpenChat);
    debug!("Starting listening on room: {}", room);
    {
        let key_map = KEYS.key_map.read().await;
        let key = key_map.get(&*room);
        let key2 = key.and_then(|key| Key::try_from(key.as_ref()).ok());
        if key2.is_some() {
            let mut key_write = KEYS.symetric_key.write().await;
            *key_write = key2;
        }
    }
    while let Some(Ok(msg)) = stream.next().await {
        debug!("Received message: {:#?}", msg);

        if let reqwest_eventsource::Event::Message(event) = msg {
            match serde_json::from_str::<MessagePublic>(&event.data) {
                Ok(message) => {
                    debug!("Parsed Content: {:#?}", message);
                    let mut received_message = Message {
                        user: message.sender,
                        send_at: message
                            .send_at.and_then(|send_at| DateTime::<Utc>::from_str(&send_at).ok()),
                        content: Default::default(),
                    };
                    match message.content {
                        Some(content) => match handle_content(&room, content).await {
                            Err(err) => {
                                error!("Failed to handle content: {}", err);
                                let _ = action_tx.send(Action::Error(err.into()));
                            }
                            Ok(Some(content)) => {
                                received_message.content = content;
                                let _ = action_tx.send(Action::ReceivedMessage(received_message));
                            }
                            Ok(_) => {}
                        },
                        None => {
                            error!("Received message with no content",);
                        }
                    }
                }
                Err(e) => {
                    let err: error::NetworkError = e.into();
                    error!("Failed to parse incoming message: {}", err);
                    let _ = action_tx.send(Action::Error(err.into()));
                }
            }
        } else {
            error!("Unexpected message type received: {:#?}", msg);
        }
    }
    Ok(())
}

async fn handle_content(room: &str, content: Content) -> Result<Option<String>> {
    match content {
        Content::Encrypted(encrypted) => {
            let key = KEYS.symetric_key.read().await;
            match key.as_ref() {
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
                            let _ = ACTION_TX
                                .read()
                                .await
                                .clone()
                                .send(Action::Error(error::NetworkError::from(e).into()));
                            if KEYS.first.load(Ordering::Relaxed) {
                                //   let mut key = KEYS.symetric_key.write().await;
                                //   if key.is_none() {
                                //       *key = Some(Key::generate()?);
                                //   }
                            } else {
                                let key_pair = KEYS.asymetric_key.read().await;
                                let msg = KeyRequest::new(to_base64(&key_pair.public_key));
                                send_message_from_content(Content::KeyRequest(msg)).await?;
                            }
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
            debug!("Received system message: {}", system_message.content);
            if system_message.online_users >= 1 {
                debug!("Last to join,requesting Key");
                let key_pair = KEYS.asymetric_key.read().await;
                let msg = KeyRequest::new(to_base64(&key_pair.public_key));
                send_message_from_content(Content::KeyRequest(msg)).await?;
                KEYS.first.store(false, Ordering::Relaxed);
            } else {
                debug!("First to join, generating Key");
                KEYS.first.store(true, Ordering::Relaxed);
                let mut key = KEYS.symetric_key.write().await;
                if key.is_none() {
                    let new_key = Key::generate()?;
                    *key = Some((&new_key.clone()).try_into()?);
                    let mut key_map = KEYS.key_map.write().await;
                    key_map.insert(room.to_string(), new_key);
                }
            }
            return Ok(Some(system_message.content));
        }
        Content::KeyResponse(_) => {
            //let _key_pair = KEY_PAIR.read().await;
            //let mut _key = SYMETRIC_KEY.write().await;
        }
        Content::KeyRequest(request_content) => {
            let key = KEYS.symetric_key.read().await;
            if let Some(key) = key.as_ref() {
                let mut public_key: PublicKey = [0u8; PUBLIC_KEY_LENGTH];
                let public_key_vec = from_base64(&request_content.public_key)?;
                public_key.copy_from_slice(public_key_vec.as_slice());
                let key_pair = KEYS.asymetric_key.read().await;
                let mut ciphertext = vec![0u8; key.len() + cipher::MAC_LENGTH];
                let (_, nonce) =
                    key_pair.encrypt(key.as_slice(), &public_key, None, &mut ciphertext)?;
                let encrypted_key_str = to_base64(&ciphertext);
                let my_public_key_str = to_base64(&key_pair.public_key);
                let nonse_str = to_base64(&nonce);
                let mut ciphertext = vec![0u8; key.len() + cipher::MAC_LENGTH];
                symetric_cipher::encrypt("TEST".as_bytes(), key, None, &mut ciphertext)?;
                let test_msg = to_base64(&ciphertext);
                let key_response =
                    KeyResponse::new(encrypted_key_str, test_msg, my_public_key_str, nonse_str);
                send_message_from_content(Content::KeyResponse(key_response)).await?;
            }
        }
    }
    Ok(None)
}

#[tracing::instrument]
async fn send_message_from_content(message_content: Content) -> Result<()> {
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
    let listen_task = LISTEN_TASK.read().await;
    let task = listen_task
        .as_ref()
        .ok_or_eyre("You Havent Joined a room")?;
    let conf = CONFIGURATION.read().await;
    experimental_api::experimental_send(&conf, &task.room, msg).await?;
    Ok(())
}

async fn send_message(message_content: &str) -> Result<()> {
    let key = KEYS.symetric_key.read().await;
    if let Some(key) = key.as_ref() {
        debug!("Sending encrypted Message");
        let plaintext = message_content.as_bytes();
        let mut ciphertext = vec![0u8; plaintext.len() + cipher::MAC_LENGTH];
        let (_, nonce) = symetric_cipher::encrypt(plaintext, key, None, &mut ciphertext)?;

        let message = Content::Encrypted(Encrypted::new(to_base64(&ciphertext), to_base64(&nonce)));
        send_message_from_content(message).await?;
        Ok(())
    } else {
        debug!("Sending plaintext Message");
        let message = Content::Plaintext(Plaintext::new(message_content.to_owned()));
        send_message_from_content(message).await?;
        Ok(())
    }
}

#[tracing::instrument]
async fn login(username: &str, password: &str) -> Result<()> {
    let mut conf = CONFIGURATION.write().await;
    let login = LoginData {
        username: username.to_owned(),
        password: password.to_owned(),
    };
    match users_api::users_login(&conf, login.clone()).await {
        Ok(response) => {
            conf.bearer_access_token = Some(response.token.token);

            let user = users_api::users_get_me(&conf).await?;
            debug!("{:#?}", user);
            update_user(user).await;
            Ok(())
        }
        Err(e) => {
            // TODO: is it a good idea to register if login fails?
            if let ApiError::ResponseError(ref e) = e
                && let Some(users_api::UsersLoginError::Status401(_)) = e.entity
            {
                if let Ok(string) = serde_json::to_string(&login) {
                    debug!("{}", string);
                }
                let response = users_api::users_register(&conf, login).await?;
                conf.bearer_access_token = Some(response.token.token);

                let user = users_api::users_get_me(&conf).await?;
                debug!("{:#?}", user);
                update_user(user).await;

                return Ok(());
            }
            Err(e.into())
        }
    }
}
pub fn to_base64(arg: &[u8]) -> String {
    general_purpose::STANDARD.encode(arg)
}

pub fn from_base64(arg: &str) -> Result<Vec<u8>> {
    Ok(general_purpose::STANDARD.decode(arg)?)
}
