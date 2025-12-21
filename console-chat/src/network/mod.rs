use crate::action::Action;
use crate::cli::Cli;
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use openapi::apis::Error as ApiError;
use openapi::apis::configuration::Configuration;
use openapi::apis::{experimental_api, users_api};
use openapi::models::*;
use std::sync::Arc;
//use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tracing::debug;

pub(crate) mod error;
 type Result<T, E = error::NetworkError> = std::result::Result<T, E>;

pub struct ListenData {
    pub thread: JoinHandle<Result<()>>,
    pub room: Arc<String>,
}

lazy_static! {
    pub static ref CONFIGURATION: Arc<RwLock<Configuration>> =
        Arc::new(RwLock::new(Configuration::new()));
    pub static ref USER: Arc<RwLock<Option<UserPrivate>>> = Arc::new(RwLock::new(None));
    pub static ref LISTEN_TASK: Arc<RwLock<Option<ListenData>>> = Arc::new(RwLock::new(None));
    pub static ref ACTION_TX: Arc<RwLock<UnboundedSender<Action>>> =
        Arc::new(RwLock::new(unbounded_channel().0));
}

pub async fn init(config: Cli, action_tx: UnboundedSender<Action>) -> Result<()> {
    *ACTION_TX.write().await = action_tx;
    let mut client = CONFIGURATION.write().await;
    if config.accept_invalid_certificate {
        client.client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;
    }
    let response = users_api::users_online(&client, None).await?;
    client.bearer_access_token = Some(response.token.token);
    let mut user = USER.write().await;
    *user = Some(users_api::users_get_me(&client).await?);
    debug!("{:#?}", user);
    Ok(())
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

async fn listen(room: Arc<String>) -> Result<()> {
    let conf = CONFIGURATION.read().await.clone();
    let mut stream = experimental_api::experimental_listen(&conf, &room).await?;
    let action_tx = ACTION_TX.read().await.clone();
    let _ = action_tx.send(Action::OpenChat);
    while let Some(Ok(msg)) = stream.next().await {
        debug!("got message: {:#?}", msg);
        if let reqwest_eventsource::Event::Message(event) = msg {
            match serde_json::from_str::<MessagePublic>(&event.data) {
                Ok(message) => {
                    let _ = action_tx.send(Action::ReceivedMessage(message));
                }
                Err(e) => {
                    let e: error::NetworkError = e.into();
                    let _ = action_tx.send(Action::Error(e.into()));
                }
            }
        }
    }
    Ok(())
}
async fn send_message(message_content: &str) -> Result<()> {
    let listen_task = LISTEN_TASK.read().await;
    if let Some(task) = listen_task.as_ref() {
        let conf = CONFIGURATION.read().await;
        let now: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::from(std::time::SystemTime::now());
        let message = MessageSend {
            r#type: Some(MessageType::Plaintext),
            content: Some(Content::Plaintext(Plaintext::new(
                message_content.to_owned(),
            ))),
            send_at: Some(now.to_rfc3339()),
            data: Some(None),
        };

        experimental_api::experimental_send(&conf, &task.room, message).await?;
    }
    Ok(())
}

async fn login(username: &str, password: &str) -> Result<()> {
    let mut conf = CONFIGURATION.write().await;
    let login = LoginData {
        username: username.to_owned(),
        password: password.to_owned(),
    };
    match users_api::users_login(&conf, login.clone()).await {
        Ok(response) => {
            conf.bearer_access_token = Some(response.token.token);

            let mut user = USER.write().await;
            *user = Some(users_api::users_get_me(&conf).await?);
            debug!("{:#?}", user);
            Ok(())
        }
        Err(e) => {
            // TODO: is it a good idea to register if login fails?
            if let ApiError::ResponseError(ref e) = e
                && let Some(users_api::UsersLoginError::Status401(_)) = e.entity {
                    if let Ok(string) = serde_json::to_string(&login) {
                        debug!("{}", string);
                    }
                    let response = users_api::users_register(&conf, login).await?;
                    conf.bearer_access_token = Some(response.token.token);

                    let mut user = USER.write().await;
                    *user = Some(users_api::users_get_me(&conf).await?);
                    debug!("{:#?}", user);
                }
            Err(e.into())
        }
    }
}
