use crate::network::listen;
use crate::{
    event,
    network::{ApiError, NetworkEvent, ResponseErrorData, encryption, messages, user},
};
use reqwest::{StatusCode, Url};
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub type NoResTokioHandles = JoinHandle<Result<(), ApiError>>;

#[derive(Debug)]
pub struct ApiClient {
    base_url: Url,
    client: reqwest::Client,
    event_sender: event::NetworkEventSender,

    //api_data: Arc<Mutex<ApiData>>,
    api_key: Option<String>,
    bearer_token: Option<String>,
    current_room: Option<String>,

    listen_task: Option<listen::ListenTask>,
    listen_data: Option<listen::ListenData>,

    listen_stop_flag: watch::Sender<bool>,

    handle_messages_task: Option<listen::HandleMessagesTask>,

    symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
    asymetric_key: Arc<Mutex<encryption::KeyPair>>,
}

///Magic numbers
pub const LISTEN_TIMEOUT: u64 = 30;

impl Drop for ApiClient {
    fn drop(&mut self) {
        self.handle_messages_task.as_ref().map(|h| h.abort());
    }
}

impl ApiClient {
    pub fn new(base_url: &str, event_sender: event::NetworkEventSender) -> Result<Self, ApiError> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let url = Url::parse(base_url)?;
        let (msg_queue_sender, msg_queue_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (tx, rx) = watch::channel(false);
        let listen_data = listen::ListenData::new(
            client.clone(),
            url.clone(),
            String::new(),
            rx,
            msg_queue_sender.clone(),
            event_sender.clone().into(),
        );

        let asym_key = encryption::get_asym_key_pair()?;
        let symetric_key = Arc::new(Mutex::new(None));
        let asymetric_key = Arc::new(Mutex::new(asym_key));
        let handle_messages_data = listen::HandleMessagesData::new(
            symetric_key.clone(),
            asymetric_key.clone(),
            event_sender.clone(),
            msg_queue_sender.clone(),
            msg_queue_receiver,
        );

        Ok(ApiClient {
            base_url: url,
            client,

            event_sender: event_sender.clone(),

            api_key: None,
            bearer_token: None,
            current_room: None,

            listen_task: None,
            listen_data: Some(listen_data),

            //handle_messages_data: None,
            handle_messages_task: Some(handle_messages_data.run()),

            listen_stop_flag: tx,

            symetric_key,
            asymetric_key,
        })
    }

    async fn handled_listen_task_results(&mut self) {
        if let Some(task) = self.listen_task.as_mut().take() {
            let _ = self.listen_stop_flag.send(true);
            if let Ok(task_result) = task.await {
                match task_result {
                    Ok(data) => self.listen_data = Some(data),
                    Err(e) => self.event_sender.send(e.into()),
                }
            }
        }
    }

    pub async fn reset(&mut self) {
        self.api_key = None;
        self.bearer_token = None;
        self.current_room = None;
        self.handled_listen_task_results().await;
    }

    pub fn set_api_key(&mut self, key: String) {
        self.api_key = Some(key);
    }

    pub fn set_bearer_token(&mut self, token: String) {
        self.bearer_token = Some(token);
    }

    pub async fn auth(&mut self, args: Option<serde_json::Value>) -> Result<(), ApiError> {
        let url = self.base_url.join("auth")?;
        let resp = if let Some(body) = args {
            self.client.post(url).json(&body)
        } else {
            self.client.post(url)
        }
        .send()
        .await?;

        let res: user::UserStatus = handle_errors_json(resp).await?;
        self.bearer_token = Some(res.token);
        Ok(())
    }

    pub async fn handle_event(&mut self, event: NetworkEvent) -> Result<(), ApiError> {
        match event {
            NetworkEvent::CreateKey => {
                let mut key_guard = self.symetric_key.lock().unwrap();
                *key_guard = Some(encryption::get_new_symetric_key()?);
            }
            NetworkEvent::RequestKeyExchange => {
                let room = self.get_room()?;
                let url = self.base_url.join(&format!("room/{room}"))?;

                let key_guard = self.asymetric_key.lock().unwrap();
                let msg = messages::ClientMessage::key_request(key_guard.public_key());
                let body = serde_json::json!(msg);

                let resp = self
                    .client
                    .post(url)
                    .json(&body)
                    .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
                    .send()
                    .await?;

                let message: messages::ServerMessage = handle_errors_json(resp).await?;
                log::debug!("{:?}", message);
            }
            NetworkEvent::SendKey(pub_key) => {
                let room = self.get_room()?;
                let url = self.base_url.join(&format!("room/{room}"))?;

                let asym_key_guard = self.asymetric_key.lock().unwrap();
                let sym_key_guard = self.symetric_key.lock().unwrap();
                match *sym_key_guard {
                    None => return Err(ApiError::from("No Symetic key")),
                    Some(ref key) => {
                        let msg = messages::ClientMessage::send_key(key, &asym_key_guard, pub_key)?;

                        let body = serde_json::json!(msg);
                        let resp = self
                            .client
                            .post(url)
                            .json(&body)
                            .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
                            .send()
                            .await?;

                        let message: messages::ServerMessage = handle_errors_json(resp).await?;
                        log::debug!("{:?}", message);
                    }
                }
            }
            NetworkEvent::Leaf => {
                let _ = self.listen_stop_flag.send(true);
                //self.handled_listen_task_results().await;
            }
            _ => {}
        }
        Ok(())
    }

    fn get_room(&self) -> Result<String, ApiError> {
        self.current_room
            .as_ref()
            .map_or_else(
                || {
                    Err(ApiError::GenericError(
                        "You haven't joined a room yet".to_owned(),
                    ))
                },
                Ok,
            )
            .cloned()
    }

    pub async fn send_txt(&mut self, msg: &str) -> Result<(), ApiError> {
        if self.symetric_key.is_poisoned() {
            let mut lock = self.symetric_key.lock().unwrap_or_else(|e| e.into_inner());
            *lock = None;
        }
        let key_guard = self.symetric_key.lock().unwrap();
        let args = match *key_guard {
            Some(ref key) => messages::ClientMessage::encrypted(encryption::encrypt(msg, key)?),
            None => messages::ClientMessage::new(msg),
        };
        log::trace!("Sending Message...");
        let room = self.get_room()?;
        let url = self.base_url.join(&format!("room/{room}"))?;
        let body = serde_json::json!(args);
        log::debug!("Sending: {body}");
        let resp = self
            .client
            .post(url)
            .json(&body)
            .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
            .send()
            .await?;
        //log::debug!("{}", resp.text().await?);
        let message: messages::ServerMessage = handle_errors_json(resp).await?;
        log::debug!("{:?}", message);
        Ok(())
    }

    pub async fn listen(&mut self, room: &str) -> Result<(), ApiError> {
        self.current_room = Some(room.to_string());
        let url = self.base_url.join(&format!("room/{room}"))?;
        let token = self.bearer_token.clone().expect("No Token Given");

        if self.listen_task.is_some() {
            //return Err("Already Joined a room".into());
            self.handled_listen_task_results().await;
        }

        match self.listen_data.take() {
            None => return Err("This is very BAD".into()),
            Some(mut data) => {
                data.update(url, token);
                self.listen_task = Some(data.run());
            }
        }
        Ok(())
    }
}

#[inline]
pub async fn handle_errors_raw(resp: reqwest::Response) -> Result<reqwest::Response, ApiError> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let url = resp.url().to_owned();
    let msg = resp
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read error message.".to_string());

    let error_data = ResponseErrorData { msg, status, url };

    match status {
        StatusCode::NOT_FOUND => Err(ApiError::NotFound(error_data)),
        StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized(error_data)),
        status if status.is_client_error() => Err(ApiError::ClientError(error_data)),
        status if status.is_server_error() => Err(ApiError::ServerError(error_data)),
        _ => Err(format!("Unexpected status: {}", status).into()),
    }
}

#[allow(unused_lifetimes)]
#[inline]
pub async fn handle_errors_json<'a, T>(resp: reqwest::Response) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    if resp.status().is_success() {
        let data = resp.json::<T>().await?;
        return Ok(data);
    }
    let status = resp.status();
    let url = resp.url().to_owned();
    let msg = resp
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read error message.".to_string());

    let error_data = ResponseErrorData { msg, status, url };

    match status {
        StatusCode::NOT_FOUND => Err(ApiError::NotFound(error_data)),
        StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized(error_data)),
        status if status.is_client_error() => Err(ApiError::ClientError(error_data)),
        status if status.is_server_error() => Err(ApiError::ServerError(error_data)),
        _ => Err(format!("Unexpected status: {}", status).into()),
    }
}
