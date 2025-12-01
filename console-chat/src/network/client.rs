use super::data_model::{messages, user::*};
use super::encryption;
use super::error::*;
use super::listen::*;
use crate::action::Action;
use color_eyre::Result;
use color_eyre::eyre::OptionExt;
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
use tracing::instrument;
use tracing::{debug, error, trace};
use url::Url;

lazy_static! {
    static ref LISTEN_WORKER: Mutex<Option<ListenHandler>> = Mutex::new(None);
}

pub static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Client {
    pub url: Url,
    pub client: reqwest::Client,
    pub token: Arc<Mutex<Option<Token>>>,
    pub action_tx: UnboundedSender<Action>,
    pub room: Arc<Mutex<Option<String>>>,
    pub symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
    pub asymetric_key: Arc<Mutex<encryption::KeyPair>>,
}
impl Client {
    pub async fn init<T>(url: T, action_tx: UnboundedSender<Action>) -> Result<()>
    where
        T: TryInto<Url> + std::fmt::Debug,
        <T as TryInto<url::Url>>::Error: Sync + Send + std::error::Error + 'static,
    {
        debug!("Initializing network client");
        let url = url.try_into()?;
        //let key = Keys::new()?;
        let is_localhost = url
            .host_str()
            .map(|host| host == "localhost" || host == "127.0.0.1" || host == "::1")
            .unwrap_or(false);

        let client_builder = reqwest::Client::builder();

        let client_builder = if is_localhost {
            client_builder.danger_accept_invalid_certs(true)
        } else {
            client_builder
        };
        let client = client_builder.build()?;
        let asymetric_key = Arc::new(Mutex::new(encryption::get_asym_key_pair()?));
        CLIENT.set(Arc::new(Client {
            url,
            client: client,
            token: Arc::new(Mutex::new(None)),
            action_tx,
            room: Arc::new(Mutex::new(None)),
            symetric_key: Arc::new(Mutex::new(None)),
            asymetric_key,
        }));
        auth().await?;
        debug!("Initializing done");
        Ok(())
    }

    pub fn get() -> Result<Arc<Client>> {
        Ok(CLIENT
            .get()
            .ok_or_eyre("Client hasnt been initialized")?
            .clone())
    }
}
impl Deref for Client {
    type Target = reqwest::Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[derive(Debug, Clone)]
pub struct Keys {
    pub symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
    pub asymetric_key: Arc<Mutex<encryption::KeyPair>>,
}

impl Keys {
    fn new() -> Result<Self, NetworkError> {
        Ok(Self {
            symetric_key: Arc::new(Mutex::new(None)),
            asymetric_key: Arc::new(Mutex::new(encryption::get_asym_key_pair()?)),
        })
    }
}

pub struct ListenHandler {
    pub listen_task: JoinHandle<Result<(), NetworkError>>,
    pub cancellation_token: CancellationToken,
}

impl Deref for ListenHandler {
    type Target = JoinHandle<Result<(), NetworkError>>;
    fn deref(&self) -> &Self::Target {
        &self.listen_task
    }
}

impl<F, Oputput> From<F> for ListenHandler
where
    F: FnOnce(CancellationToken) -> Oputput + 'static + Sync + Send,
    Oputput: Future<Output = Result<(), NetworkError>> + 'static + Sync + Send,
{
    fn from(value: F) -> ListenHandler {
        let cancellation_token = CancellationToken::new();
        let cancell_token = cancellation_token.clone();
        ListenHandler {
            listen_task: tokio::spawn(async move { (value)(cancell_token).await }),
            cancellation_token,
        }
    }
}

pub fn handle_network(action: Action) -> Result<Option<Action>> {
    Ok(match action {
        Action::PerformJoin(room) => {
            run_async_sync(join_room, room)?;
            Some(Action::OpenChat)
        }
        Action::PerformLogin(username, password) => {
            run_async_sync(login, (username, password))?;
            Some(Action::OpenHome)
            //None
        }
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

pub fn run_async_sync<F, T, R>(func: F, data: T) -> Result<()>
where
    F: FnOnce(T) -> R + 'static + Sync + Send,
    R: Future<Output = Result<Option<Action>, NetworkError>> + Sync + Send + 'static,
    T: Sync + Send + 'static,
{
    let _: JoinHandle<Result<()>> = tokio::spawn(async {
        let res = (func)(data).await;
        let client = Client::get()?;
        let _ = match res {
            Ok(Some(action)) => client.action_tx.send(action),
            Err(error) => {
                error!("{error}");
                client.action_tx.send(Action::Error(error.into()))
            }
            Ok(None) => Ok(()),
        };
        Ok(())
    });
    Ok(())
}

async fn auth() -> Result<()> {
    trace!("Getting client lock");
    let client = Client::get()?;
    trace!("sending auth request");
    let mut token_guard = client.token.lock().await;
    let token: Token = match token_guard.as_ref() {
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
        None => handle_errors_json(client.post(client.url.join("/auth")?).send().await?).await?,
    };
    debug!("got auth result: {:#?}", token);
    keep_token_alive_auth(token.ttl.clone());
    *token_guard = Some(token);
    Ok(())
}

fn keep_token_alive_auth(ttl: std::time::Duration) {
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

async fn leave(_: ()) -> Result<Option<Action>, NetworkError> {
    let mut listen_worker_guard = LISTEN_WORKER.lock().await;
    if let Some(listen_worker) = listen_worker_guard.take() {
        debug!("cancelling listen_worker");
        listen_worker.cancellation_token.cancel();
        listen_worker.listen_task.await??;
        let client = Client::get()?;
        let mut room = client.room.lock().await;
        *room = None;
    }
    Ok(None)
}

async fn join_room(room: String) -> Result<Option<Action>, NetworkError> {
    let client = Client::get()?;
    let mut room_guard = client.room.lock().await;
    match *room_guard {
        Some(_) => return Err(NetworkError::GenericError("Already in a room".to_string())),
        None => {
            *room_guard = Some(room);
            let mut listen_worker_mutex = LISTEN_WORKER.lock().await;
            match listen_worker_mutex.take() {
                Some(listen_worker) => listen_worker.abort(),
                None => {
                    *listen_worker_mutex = Some(ListenHandler::from(listen));
                    debug!("Started Listen Worker");
                }
            }
        }
    }
    Ok(None)
}

pub async fn login(credentials: (String, String)) -> Result<Option<Action>, NetworkError> {
    let client = Client::get()?;

    use std::collections::HashMap;
    let (username, password) = credentials;

    let body = HashMap::from([("username", username), ("password", password)]);

    let mut request = client.post(client.url.join("auth")?).json(&body);

    let mut token_guard = client.token.lock().await;
    if let Some(token) = token_guard.as_ref() {
        request = request.bearer_auth(token.token.clone());
    }
    let responce: Token = handle_errors_json(request.send().await?).await?;

    debug!("{responce:#?}");
    *token_guard = Some(responce);
    Ok(None)
}

pub async fn send_txt(msg: String) -> Result<Option<Action>, NetworkError> {
    let client = Client::get()?;
    if let Some(room) = client.room.lock().await.as_ref() {
        trace!("Sending Message...");
        let msg: Result<messages::ClientMessage, NetworkError> = {
            let key_guard = client.symetric_key.lock().await;
            key_guard
                .as_ref()
                .map_or(Ok(messages::ClientMessage::new(&msg)), |key| {
                    trace!("{key:#?}");
                    let encrypted = encryption::encrypt(&msg, &key)?;
                    Ok(messages::ClientMessage::encrypted(encrypted))
                })
        };
        let url = client.url.join(&format!("room/{room}"))?;
        let token_guard = client.token.lock().await;
        let token = token_guard.as_ref().ok_or(NetworkError::MissingAuthToken)?;

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
    Ok(None)
}

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
