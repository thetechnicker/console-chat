use crate::{
    event,
    network::{ApiError, NetworkEvent, ResponseErrorData, user},
};
//use bytes::Bytes;
use reqwest::{StatusCode, Url};
use std::str;
use tokio::task::JoinHandle;
use tokio_stream::{self, StreamExt};

//pub type ApiClientType = Arc<Mutex<ApiClient>>;

#[derive(Debug)]
pub struct ApiClient {
    base_url: Url,
    client: reqwest::Client,
    api_key: Option<String>,
    bearer_token: Option<String>,

    _max_api_failure_count: u32,
    _api_failure_count: u32,
    event_sender: event::NetworkEventSender,
    listen_task: Option<JoinHandle<Result<(), ApiError>>>,
    current_room: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: &str, event_sender: event::NetworkEventSender) -> Result<Self, ApiError> {
        let client = reqwest::Client::new();
        Ok(ApiClient {
            base_url: Url::parse(base_url)?,
            client,
            api_key: None,
            bearer_token: None,

            _max_api_failure_count: 0,
            _api_failure_count: 0,
            event_sender,

            listen_task: None,
            current_room: None,
        })
    }

    pub fn reset(&mut self) {
        self.api_key = None;
        self.bearer_token = None;
        self.current_room = None;
        if let Some(t) = self.listen_task.as_mut() {
            t.abort();
        }
        self.listen_task = None;
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

    pub async fn send(&mut self, args: user::ClientMessage) -> Result<(), ApiError> {
        log::trace!("Sending Message...");
        let room = self.current_room.as_ref().map_or_else(
            || {
                Err(ApiError::GenericError(
                    "You haven't joined a room yet".to_owned(),
                ))
            },
            |t| Ok(t),
        )?;
        let url = self.base_url.join(&format!("room/{room}"))?;
        let body = serde_json::json!(args);
        log::debug!("{body}");
        let resp = self
            .client
            .post(url)
            .json(&body)
            .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
            .send()
            .await?;

        let resp = handle_errors_raw(resp).await?;
        //self.event_sender
        //    .send(NetworkEvent::Error(ApiError::GenericError(format!(
        //        "{}",
        //        resp.text().await?
        //    ))));
        log::trace!("{}", resp.text().await?);
        Ok(())
    }

    pub async fn listen(&mut self, room: &str) -> Result<(), ApiError> {
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
        self.current_room = Some(room.to_string());
        let timeout = 30;
        let url = self.base_url.join(&format!("room/{room}"))?;

        let resp = self
            .client
            .get(url)
            .query(&[("listen_seconds", &timeout.to_string())])
            .timeout(std::time::Duration::from_secs(timeout))
            .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
            .send()
            .await?;

        let resp = handle_errors_raw(resp).await?;
        let local_sender = self.event_sender.clone();
        self.listen_task = Some(tokio::spawn(async move {
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                log::debug!("{chunk:?}");
                match chunk {
                    Err(e) => local_sender.send(NetworkEvent::Error(e.into())),
                    Ok(data) => match str::from_utf8(&data) {
                        Ok(s) => match serde_json::from_str::<user::ServerMessage>(s) {
                            Ok(msg) => local_sender.send(NetworkEvent::Message(msg)),
                            Err(_) => {} //local_sender.send(NetworkEvent::Error(e.into())),
                        },
                        Err(e) => local_sender.send(NetworkEvent::Error(e.into())),
                    },
                }
            }
            local_sender.send(NetworkEvent::RequestReconnect);
            Ok(())
        }));
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

#[inline]
pub async fn handle_errors_json<'de, T>(resp: reqwest::Response) -> Result<T, ApiError>
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
