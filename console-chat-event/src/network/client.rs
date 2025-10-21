use crate::network::{
    ApiError,
    user::{BetterUser, UserStatus},
};
use reqwest::{StatusCode, Url};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ApiClientType = Arc<Mutex<ApiClient>>;

#[derive(Debug)]
pub struct ApiClient {
    base_url: Url,
    client: Arc<reqwest::Client>,
    api_key: Option<String>,
    bearer_token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Result<Arc<Mutex<Self>>, ApiError> {
        let client = Arc::new(reqwest::Client::new());
        Ok(Arc::new(Mutex::new(ApiClient {
            base_url: Url::parse(base_url)?,
            client,
            api_key: None,
            bearer_token: None,
        })))
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

    pub async fn auth(&mut self, args: Option<serde_json::Value>) -> Result<UserStatus, ApiError> {
        let url = self.base_url.join("auth")?;
        let resp = if let Some(body) = args {
            self.client.post(url).json(&body)
        } else {
            self.client.post(url)
        }
        .send()
        .await?;

        match resp.status() {
            StatusCode::OK => {
                let body = resp.json::<UserStatus>().await?;
                self.bearer_token = Some(body.token.clone());
                Ok(body)
            }
            StatusCode::NOT_FOUND => Err(ApiError::NotFound(resp.text().await.ok())),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized(resp.text().await.ok())),
            status if status.is_client_error() => Err(ApiError::ClientError(status)),
            status if status.is_server_error() => Err(ApiError::ServerError(status)),
            _ => Err(format!("Unexpected status: {}", resp.status()).into()),
        }
    }

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

    pub async fn listen(&self, room: &str) -> Result<(), ApiError> {
        let url = self.base_url.join("auth")?.join(room)?;
        let _resp = self
            .client
            .get(url)
            .bearer_auth(self.bearer_token.clone().expect("No Token Given"))
            .send()
            .await?;

        Ok(())
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
}
