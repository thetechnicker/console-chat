use crate::network::user::{BetterUser, UserStatus};
use reqwest::{Client, Error, Response, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub struct ApiClient {
    base_url: Url,
    client: Arc<reqwest::Client>,
    api_key: Option<String>,
    bearer_token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Result<Self, reqwest::Error> {
        let client = Arc::new(reqwest::Client::new());
        Ok(ApiClient {
            base_url: Url::parse(base_url)?,
            client,
            api_key: None,
            bearer_token: None,
        })
    }

    pub fn set_api_key(&mut self, key: String) {
        self.api_key = Some(key);
    }

    pub fn set_bearer_token(&mut self, token: String) {
        self.bearer_token = Some(token);
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserStatus, reqwest::Error> {
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

    pub async fn get_user_status(&self) -> Result<BetterUser, reqwest::Error> {
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

    // Additional methods for other endpoints...
}
