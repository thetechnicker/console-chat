use chrono::{DateTime, Utc};
use color_eyre::Result;
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use openapi_custom::apis::{self, configuration, users_api};
use openapi_custom::models::*;
use std::time::SystemTime;
use tokio_tungstenite::tungstenite::{client::ClientRequestBuilder, protocol::Message};
use tokio_tungstenite::{connect_async_tls_with_config, Connector};

#[tokio::main]
async fn main() -> Result<()> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let token = users_api::online_users_online_get(&conf, None).await?;
    println!("Token: {:#?}", token);
    conf.bearer_access_token = Some(token.token.token.clone());

    let uri_str = format!("{}/rooms/room", conf.base_path);
    let mut req_builder = conf.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref user_agent) = conf.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = conf.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = conf.client.execute(req).await?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");
    let content_type = apis::ContentType::from(content_type);

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        println!("{:#?}", content);
        match content_type {
            apis::ContentType::Json => todo!(),
            apis::ContentType::Text => todo!(),
            apis::ContentType::Unsupported(unknown_type) => todo!(),
        }
    } else {
        let content = resp.text().await?;
        println!("{:#?}", content);
    };

    Ok(())
}
