use chrono::{DateTime, Utc};
use color_eyre::Result;
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use openapi_custom::apis::{self, configuration, users_api};
use openapi_custom::models::*;
use reqwest_eventsource::{Event, EventSource};
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<()> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let token = users_api::online_users_online_get(&conf, None).await?;
    println!("Token: {:#?}", token);
    conf.bearer_access_token = Some(token.token.token.clone());
    loop {
        let res = listen(&conf).await;
        println!("res: {:#?}", res)
    }
}

async fn listen(conf: &configuration::Configuration) -> Result<()> {
    let mut req = conf.client.get(format!("{}/r/abc", conf.base_path));
    if let Some(token) = conf.bearer_access_token.as_ref() {
        req = req.bearer_auth(token.to_owned());
    }
    let mut es = EventSource::new(req)?;

    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => {
                println!(
                    "Message: {:#?}",
                    serde_json::from_str::<serde_json::Value>(&message.data)
                )
            }
            Err(err) => {
                println!("Error: {}", err);
                es.close();
            }
        }
    }
    Ok(())
}
