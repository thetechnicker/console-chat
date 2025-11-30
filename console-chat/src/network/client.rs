use super::data_model::{messages, user::*};
use super::encryption;
use super::error::*;
use crate::action::Action;
use color_eyre::Result;
use futures::StreamExt;
use lazy_static::lazy_static;
use reqwest;
use reqwest::StatusCode;
use std::convert::TryInto;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, instrument, trace};
use url::Url;

const LISTEN_TIMEOUT: u64 = 30;

#[derive(Debug, Clone)]
pub struct Client {
    url: Url,
    client: reqwest::Client,
    token: Option<Token>,
    action_tx: UnboundedSender<Action>,
    room: Option<String>,
    key: Keys,
}

#[derive(Debug, Clone)]
pub struct Keys {
    symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
    asymetric_key: Arc<Mutex<encryption::KeyPair>>,
}

impl Keys {
    fn new() -> Result<Self, NetworkError> {
        Ok(Self {
            symetric_key: Arc::new(Mutex::new(None)),
            asymetric_key: Arc::new(Mutex::new(encryption::get_asym_key_pair()?)),
        })
    }
}

pub fn run_async_sync<F, T, R>(func: F, data: T) -> Result<()>
where
    F: FnOnce(T) -> R + 'static + Sync + Send,
    R: Future<Output = Result<Option<Action>, NetworkError>> + Sync + Send + 'static,
    T: Sync + Send + 'static,
{
    let _ = tokio::spawn(async {
        let res = (func)(data).await;
        if let Some(client_lock) = CLIENT.get() {
            let client = client_lock.lock().await;
            let _ = match res {
                Ok(Some(action)) => client.action_tx.send(action),
                Err(error) => {
                    error!("{error}");
                    client.action_tx.send(Action::Error(error.into()))
                }
                Ok(None) => Ok(()),
            };
        }
    });
    Ok(())
}

impl Deref for Client {
    type Target = reqwest::Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

pub struct ListenHandler {
    pub task: JoinHandle<Result<(), NetworkError>>,
    pub cancellation_token: CancellationToken,
}

impl Deref for ListenHandler {
    type Target = JoinHandle<Result<(), NetworkError>>;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl<F, Oputput> From<(F, Client)> for ListenHandler
where
    F: FnOnce(Client, CancellationToken) -> Oputput + 'static + Sync + Send,
    Oputput: Future<Output = Result<(), NetworkError>> + 'static + Sync + Send,
{
    fn from(value: (F, Client)) -> ListenHandler {
        let cancellation_token = CancellationToken::new();
        let cancell_token = cancellation_token.clone();
        let task = value.0;
        let client = value.1;
        ListenHandler {
            task: tokio::spawn(async move { (task)(client, cancell_token).await }),
            cancellation_token,
        }
    }
}

static CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();
lazy_static! {
    static ref LISTEN_WORKER: Mutex<Option<ListenHandler>> = Mutex::new(None);
}

//#[instrument]
pub async fn init<T>(url: T, action_tx: UnboundedSender<Action>) -> Result<()>
where
    T: TryInto<Url> + std::fmt::Debug,
    <T as TryInto<url::Url>>::Error: Sync + Send + std::error::Error + 'static,
{
    debug!("Initializing network client");
    let url = url.try_into()?;
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let key = Keys::new()?;
    CLIENT.get_or_init(|| {
        Mutex::new(Client {
            url,
            client: client,
            token: None,
            action_tx,
            room: None,
            key,
        })
    });
    auth().await?;
    debug!("Initializing done");
    Ok(())
}

//#[instrument]
async fn auth() -> Result<()> {
    trace!("Getting client lock");
    if let Some(client_lock) = CLIENT.get() {
        let mut client = client_lock.lock().await;
        trace!("sending auth request");
        let token: Token = match client.token.as_ref() {
            Some(token) => {
                handle_errors_json(
                    client
                        .post(client.url.join("/auth")?)
                        .bearer_auth(&token.token)
                        .send()
                        .await?,
                )
                .await?
            }
            None => {
                handle_errors_json(client.post(client.url.join("/auth")?).send().await?).await?
            }
        };
        debug!("got auth result: {:#?}", token);
        register_auth(token.ttl.clone());
        client.token = Some(token);
    }
    Ok(())
}

fn register_auth(ttl: std::time::Duration) {
    let timeout = ttl - std::time::Duration::from_secs(3);
    debug!("Scheduling Auth to run againin in: {timeout:?}");
    tokio::spawn(async move {
        trace!("Sleeping");
        tokio::time::sleep(timeout).await;
        trace!("calling auth");
        match auth().await {
            Ok(_) => debug!("Successfully reauthed"),
            Err(e) => error!("got error: {e}"),
        }
    });
}

pub fn handle_network(action: Action) -> Result<Option<Action>> {
    Ok(match action {
        Action::PerformJoin(room) => {
            run_async_sync(join_room, room)?;
            Some(Action::OpenChat)
        }
        Action::PerformLogin(_, _) => Some(Action::OpenHome),
        Action::SendMessage(message) => {
            run_async_sync(send_txt, message)?;
            None
        }
        Action::Leave => {
            run_async_sync(leave, ())?;
            None
        }
        _ => None,
    })
}

async fn leave(_: ()) -> Result<Option<Action>, NetworkError> {
    let mut listen_worker_guard = LISTEN_WORKER.lock().await;
    if let Some(listen_worker) = listen_worker_guard.take() {
        debug!("cancelling listen_worker");
        listen_worker.cancellation_token.cancel();
    }
    Ok(None)
}

//#[instrument]
async fn listen(client: Client, cancellation_token: CancellationToken) -> Result<(), NetworkError> {
    let room = client.room.ok_or(NetworkError::NoRoom)?;
    let token = client.token.ok_or(NetworkError::MissingAuthToken)?;

    loop {
        let responce = send_listen_request(
            client.client.clone(),
            client.url.join(&format!("room/{room}"))?,
            token.clone(),
        )
        .await?;
        let mut stream = responce.bytes_stream();
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("listen worker cancelled");
                break;
            }
            Some(chunk)=stream.next()=>{
                debug!("Received Chunk {chunk:?}");
                let chunk = match chunk {
                    Err(e) => {
                        debug!("Error Receiving chunk: {e:#?}");
                        break;
                    }
                    Ok(data) => data,
                };

                let s = str::from_utf8(&chunk)?;

                debug!("chunk as string: {s}");

                if s == "END" {
                    continue;
                }

                let msg = match serde_json::from_str::<messages::ServerMessage>(s) {
                    Ok(msg) => Ok(msg),
                    Err(e) => Err(NetworkError::from((e, s))),
                }?;
                debug!("Got message: {msg:#?}\n{}", if msg.base.is_mine(){"is mine"}else{"from someone else"});
                let _ = client.action_tx.send(Action::ReceivedMessage(msg));
            }
        }
    }
    Ok(())
}

//#[instrument]
async fn send_listen_request(
    client: reqwest::Client,
    url: Url,
    token: Token,
) -> Result<reqwest::Response, NetworkError> {
    let resp = client
        .get(url)
        .query(&[("listen_seconds", &LISTEN_TIMEOUT.to_string())])
        .timeout(std::time::Duration::from_secs(LISTEN_TIMEOUT))
        .bearer_auth(token.token)
        .send()
        .await?;

    Ok(handle_errors_raw(resp).await?)
}

async fn join_room(room: String) -> Result<Option<Action>, NetworkError> {
    if let Some(client_lock) = CLIENT.get() {
        let mut client = client_lock.lock().await;
        match client.room {
            Some(_) => return Err(NetworkError::GenericError("Already in a room".to_string())),
            None => {
                client.room = Some(room);
                let mut listen_worker_mutex = LISTEN_WORKER.lock().await;
                match listen_worker_mutex.take() {
                    Some(listen_worker) => listen_worker.abort(),
                    None => {
                        let worker_client = client.clone();
                        *listen_worker_mutex = Some(ListenHandler::from((listen, worker_client)));
                        debug!("Started Listen Worker");
                    }
                }
            }
        }
    }
    Ok(None)
}

//#[instrument]
pub async fn send_txt(msg: String) -> Result<Option<Action>, NetworkError> {
    if let Some(client_lock) = CLIENT.get() {
        let client = client_lock.lock().await;
        if let Some(room) = client.room.as_ref() {
            trace!("Sending Message...");
            let msg: Result<messages::ClientMessage, NetworkError> = {
                let key_guard = client.key.symetric_key.lock().await;
                key_guard
                    .as_ref()
                    .map_or(Ok(messages::ClientMessage::new(&msg)), |key| {
                        let encrypted = encryption::encrypt(&msg, &key)?;
                        Ok(messages::ClientMessage::encrypted(encrypted))
                    })
            };
            let url = client.url.join(&format!("room/{room}"))?;
            let token = client.token.as_ref().expect("No Token Given");
            let body = serde_json::json!(msg?);
            let resp = client
                .post(url)
                .json(&body)
                .bearer_auth(token.token.clone())
                .send()
                .await?;
            let message: messages::ServerMessage = handle_errors_json(resp).await?;
            debug!("{:?}", message);
        }
    }
    Ok(None)
}

//#[instrument]
pub async fn handle_errors_raw(resp: reqwest::Response) -> Result<reqwest::Response, NetworkError> {
    trace!(
        "handle_errors_raw: received response with status {}",
        resp.status()
    );
    if resp.status().is_success() {
        debug!(
            "handle_errors_raw: success status {}, returning response",
            resp.status()
        );
        return Ok(resp);
    }

    let status = resp.status();
    let url = resp.url().to_owned();
    debug!(
        "handle_errors_raw: non-success status {} for URL {}",
        status, url
    );

    let msg = resp.text().await.unwrap_or_else(|_| {
        debug!("handle_errors_raw: failed to read error body, using fallback message");
        "Failed to read error message.".to_string()
    });

    debug!("handle_errors_raw: response body for error: {}", msg);

    let error_data = ResponseErrorData { msg, status, url };

    match status {
        StatusCode::NOT_FOUND => {
            debug!("handle_errors_raw: mapping to NetworkError::NotFound");
            Err(NetworkError::NotFound(error_data))
        }
        StatusCode::UNAUTHORIZED => {
            debug!("handle_errors_raw: mapping to NetworkError::Unauthorized");
            Err(NetworkError::Unauthorized(error_data))
        }
        status if status.is_client_error() => {
            debug!("handle_errors_raw: mapping to NetworkError::ClientError");
            Err(NetworkError::ClientError(error_data))
        }
        status if status.is_server_error() => {
            debug!("handle_errors_raw: mapping to NetworkError::ServerError");
            Err(NetworkError::ServerError(error_data))
        }
        _ => {
            debug!(
                "handle_errors_raw: mapping to generic NetworkError for status {}",
                status
            );
            Err(format!("Unexpected status: {}", status).into())
        }
    }
}

//#[instrument]
pub async fn handle_errors_json<'a, T>(resp: reqwest::Response) -> Result<T, NetworkError>
where
    T: serde::de::DeserializeOwned,
{
    trace!(
        "handle_errors_json: received response with status {}",
        resp.status()
    );

    if resp.status().is_success() {
        debug!(
            "handle_errors_json: success status {}, attempting to deserialize JSON",
            resp.status()
        );
        let data = resp.json::<T>().await?;
        debug!("handle_errors_json: JSON deserialization succeeded");
        return Ok(data);
    }

    let status = resp.status();
    let url = resp.url().to_owned();
    debug!(
        "handle_errors_json: non-success status {} for URL {}",
        status, url
    );

    let msg = resp.text().await.unwrap_or_else(|_| {
        debug!("handle_errors_json: failed to read error body, using fallback message");
        "Failed to read error message.".to_string()
    });

    debug!("handle_errors_json: response body for error: {}", msg);

    let error_data = ResponseErrorData { msg, status, url };

    match status {
        StatusCode::NOT_FOUND => {
            debug!("handle_errors_json: mapping to NetworkError::NotFound");
            Err(NetworkError::NotFound(error_data))
        }
        StatusCode::UNAUTHORIZED => {
            debug!("handle_errors_json: mapping to NetworkError::Unauthorized");
            Err(NetworkError::Unauthorized(error_data))
        }
        status if status.is_client_error() => {
            debug!("handle_errors_json: mapping to NetworkError::ClientError");
            Err(NetworkError::ClientError(error_data))
        }
        status if status.is_server_error() => {
            debug!("handle_errors_json: mapping to NetworkError::ServerError");
            Err(NetworkError::ServerError(error_data))
        }
        _ => {
            debug!(
                "handle_errors_json: mapping to generic NetworkError for status {}",
                status
            );
            Err(format!("Unexpected status: {}", status).into())
        }
    }
}
