use crate::{
    event,
    network::{ApiError, NetworkEvent, ResponseErrorData, encryption, messages, user},
};
use base64::{Engine as _, engine::general_purpose};
use reqwest::{StatusCode, Url};
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
//use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_stream::{self, StreamExt};

type NoResTokioHandles = JoinHandle<Result<(), ApiError>>;

#[derive(Debug)]
pub struct ApiClient {
    base_url: Url,
    client: reqwest::Client,
    _max_api_failure_count: u32,
    _api_failure_count: u32,
    event_sender: event::NetworkEventSender,

    //api_data: Arc<Mutex<ApiData>>,
    api_key: Option<String>,
    bearer_token: Option<String>,
    current_room: Option<String>,
    // Main encryption Key
    listen_task: Option<NoResTokioHandles>,
    handle_server_messages: Option<NoResTokioHandles>,
    msg_queue_sender: tokio::sync::mpsc::UnboundedSender<messages::ServerMessage>,
    msg_queue_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<messages::ServerMessage>>,

    symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
    //asymetric_key: Arc<Mutex<Option<encryption::KeyType>>>,
}

///Magic numbers
const LISTEN_TIMEOUT: u64 = 30;

impl ApiClient {
    pub fn new(base_url: &str, event_sender: event::NetworkEventSender) -> Result<Self, ApiError> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let (msg_queue_sender, msg_queue_receiver) = tokio::sync::mpsc::unbounded_channel();
        Ok(ApiClient {
            base_url: Url::parse(base_url)?,
            client,

            _max_api_failure_count: 0,
            _api_failure_count: 0,
            event_sender: event_sender.clone(),

            api_key: None,
            bearer_token: None,
            current_room: None,

            listen_task: None,
            handle_server_messages: None,
            msg_queue_sender,
            msg_queue_receiver: Some(msg_queue_receiver),

            symetric_key: Arc::new(Mutex::new(None)),
            //asymetric_key: Arc::new(Mutex::new(None)),
        })
    }

    pub fn reset(&mut self) {
        self.api_key = None;
        self.bearer_token = None;
        self.current_room = None;
        if let Some(t) = self.listen_task.as_mut() {
            t.abort();
        }
        if let Some(t) = self.handle_server_messages.as_mut() {
            t.abort();
        }
        self.listen_task = None;
        self.handle_server_messages = None;
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
            _ => {}
        }
        Ok(())
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
        let room = self.current_room.as_ref().map_or_else(
            || {
                Err(ApiError::GenericError(
                    "You haven't joined a room yet".to_owned(),
                ))
            },
            Ok,
        )?;
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
        if self.msg_queue_receiver.is_none() {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            self.msg_queue_receiver = Some(rx);
            self.msg_queue_sender = tx;
        }
        self.manage_msgs().await?;
        self.listen_internal(room).await
    }
    pub async fn listen_reconnect(&mut self) -> Result<(), ApiError> {
        if let Some(room) = self.current_room.as_ref() {
            let r = room.clone();
            self.listen_internal(&r).await
        } else {
            Err(ApiError::GenericError(
                "You havent Joined a room yet".to_string(),
            ))
        }
    }

    async fn listen_internal(&mut self, room: &str) -> Result<(), ApiError> {
        let local_sender = self.event_sender.clone();
        let msg_sender = self.msg_queue_sender.clone();

        let url = self.base_url.join(&format!("room/{room}"))?;
        let token = self.bearer_token.clone().expect("No Token Given");
        let client = self.client.clone();

        if let Some(task) = self.listen_task.as_mut() {
            task.abort()
        }

        self.listen_task = Some(tokio::spawn(async move {
            let resp = client
                .get(url)
                .query(&[("listen_seconds", &LISTEN_TIMEOUT.to_string())])
                .timeout(std::time::Duration::from_secs(LISTEN_TIMEOUT))
                .bearer_auth(token)
                .send()
                .await?;

            let resp = handle_errors_raw(resp).await?;
            let mut stream = resp.bytes_stream();

            while let Some(chunk) = stream.next().await {
                log::debug!("Received Chunk {chunk:?}");
                let chunk = match chunk {
                    Err(e) => {
                        local_sender.send(NetworkEvent::Error(e.into()));
                        continue;
                    }
                    Ok(data) => data,
                };

                let s = match str::from_utf8(&chunk) {
                    Ok(s) => s,
                    Err(e) => {
                        local_sender.send(NetworkEvent::Error(e.into()));
                        continue;
                    }
                };

                log::debug!("chunk as string: {s}");

                if s == "END" {
                    break;
                }

                let msg = match serde_json::from_str::<messages::ServerMessage>(s) {
                    Ok(msg) => msg,
                    Err(e) => {
                        local_sender.send(NetworkEvent::Error((e, s).into()));
                        continue;
                    }
                };
                let _ = msg_sender.send(msg);
            }
            local_sender.send(NetworkEvent::RequestReconnect);
            Ok(())
        }));
        Ok(())
    }
    async fn manage_msgs(&mut self) -> Result<(), ApiError> {
        let local_sender = self.event_sender.clone();
        let msg_sender = self.msg_queue_sender.clone();
        let sym_key = self.symetric_key.clone();
        if let Some(task) = self.handle_server_messages.as_mut() {
            task.abort()
        }

        if let Some(mut queue) = self.msg_queue_receiver.take() {
            self.handle_server_messages = Some(tokio::spawn(async move {
                while let Some(mut msg) = queue.recv().await {
                    log::debug!("Received Message: {msg:#?}");
                    match msg.base.message_type {
                        messages::MessageType::System => {
                            if let Some(data) = msg.base.data {
                                log::debug!("Received Message: {data:#?}");
                                if data.contains_key("online") {
                                    if let Some(online) = data.get("online").unwrap().as_number() {
                                        if let Some(num_online) = online.as_u64() {
                                            if num_online == 1 {
                                                //panic!();
                                                local_sender.send(NetworkEvent::CreateKey);
                                                continue;
                                            }
                                        }
                                    }
                                    local_sender.send(NetworkEvent::RequestKeyExchange);
                                }
                            }
                        }
                        messages::MessageType::Key => {}
                        messages::MessageType::Join => {
                            // TODO: Send Key Sync
                            local_sender.send(NetworkEvent::Message(msg));
                        }
                        messages::MessageType::EncryptedText => {
                            match sym_key.lock().unwrap().as_ref() {
                                None => {
                                    local_sender
                                        .send(NetworkEvent::Error(ApiError::from("No Key")));
                                    local_sender.send(NetworkEvent::RequestKeyExchange);
                                    let _ = msg_sender.send(msg);
                                }
                                Some(ref key) => {
                                    let text = msg.base.text.clone();
                                    // TODO: maybe send nonce as base64 str and not array
                                    if let Some(nonce_json) =
                                        msg.base.data.as_ref().map_or(None, |h| h.get("nonce"))
                                    {
                                        let mut nonce: encryption::Nonce =
                                            encryption::Nonce::default();
                                        let nonce_ref_v = general_purpose::STANDARD
                                            .decode(nonce_json.as_str().unwrap())?;
                                        for i in 0..nonce.len() {
                                            nonce[i] = nonce_ref_v[i];
                                        }
                                        match encryption::decrypt((text, nonce), key) {
                                            Err(e) => {
                                                local_sender.send(NetworkEvent::Error(e.into()));
                                                local_sender.send(NetworkEvent::RequestKeyExchange);
                                                if msg.base.data.is_none() {
                                                    msg.base.data =
                                                        Some(std::collections::HashMap::new());
                                                }
                                                let retries = msg
                                                    .base
                                                    .data
                                                    .as_mut()
                                                    .map(|h| {
                                                        let base = if let Some(x) = h.get("retry") {
                                                            x.as_u64().unwrap_or(0) + 1
                                                        } else {
                                                            0
                                                        };
                                                        (*h).insert(
                                                            "retry".to_owned(),
                                                            serde_json::Value::from(base),
                                                        );
                                                        base
                                                    })
                                                    .unwrap_or(0);

                                                if retries < 10 {
                                                    let _ = msg_sender.send(msg);
                                                }
                                            }
                                            Ok(txt) => {
                                                msg.base.text = txt;
                                                local_sender.send(NetworkEvent::Message(msg));
                                            }
                                        }
                                    } else {
                                        local_sender
                                            .send(NetworkEvent::Error("This Is BAD".into()));
                                    }
                                }
                            }
                        }
                        _ => {
                            local_sender.send(NetworkEvent::Message(msg));
                        }
                    }
                }
                Ok(())
            }));
        }
        Ok(())
    }
}

#[inline]
async fn handle_errors_raw(resp: reqwest::Response) -> Result<reqwest::Response, ApiError> {
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
async fn handle_errors_json<'a, T>(resp: reqwest::Response) -> Result<T, ApiError>
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
