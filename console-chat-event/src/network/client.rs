use crate::{
    event,
    network::{ApiError, NetworkEvent, ResponseErrorData, user::UserStatus},
};
//use bytes::Bytes;
use reqwest::{StatusCode, Url};
use std::str;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_stream::{self, StreamExt};

//pub type ApiClientType = Arc<Mutex<ApiClient>>;

#[derive(Debug)]
pub struct ApiClient {
    base_url: Url,
    client: Arc<reqwest::Client>,
    api_key: Option<String>,
    bearer_token: Option<String>,

    _max_api_failure_count: u32,
    _api_failure_count: u32,
    event_sender: event::NetworkEventSender,
    listen_task: Option<JoinHandle<Result<(), ApiError>>>,
}

impl ApiClient {
    pub fn new(base_url: &str, event_sender: event::NetworkEventSender) -> Result<Self, ApiError> {
        let client = Arc::new(reqwest::Client::new());
        Ok(ApiClient {
            base_url: Url::parse(base_url)?,
            client,
            api_key: None,
            bearer_token: None,

            _max_api_failure_count: 0,
            _api_failure_count: 0,
            event_sender,

            listen_task: None,
        })
    }

    pub fn reset(&mut self) {
        self.api_key = None;
        self.bearer_token = None;
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

        let res: UserStatus = handle_errors_json(resp).await?;
        self.bearer_token = Some(res.token);
        Ok(())
    }

    /*
    #[allow(unreachable_code, unused_variables)]
    pub async fn login(&self, username: &str, password: &str) -> Result<UserStatus, ApiError> {
        todo!();
        let url = self.base_url.join("auth")?;
        let body = serde_json::json!({ "username": username, "password": password });
        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await?
            .json::<UserStatus>()
            .await?;
        Ok(resp)
    }
    pub async fn get_user_status(&self) -> Result<BetterUser, ApiError> {
        let url = self.base_url.join("users/status")?;
        let req = self.client.get(url);
        let req = if let Some(token) = &self.bearer_token {
            req.bearer_auth(token)
        } else {
            req
        };
        let resp = req.send().await?.json::<BetterUser>().await?;
        Ok(resp)
    }
    */

    pub async fn listen(&mut self, room: &str) -> Result<(), ApiError> {
        let url = self.base_url.join("auth")?.join(room)?;
        let resp = self
            .client
            .get(url)
            .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
            .send()
            .await?;

        let resp = handle_errors_raw(resp).await?;
        let local_sender = self.event_sender.clone();
        self.listen_task = Some(tokio::spawn(async move {
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                chunk?;
                local_sender.send(NetworkEvent::Message);
            }
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
