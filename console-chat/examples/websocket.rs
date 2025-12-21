use chrono::{DateTime, Utc};
use color_eyre::Result;
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use openapi_custom::apis::{configuration, users_api};
use openapi_custom::models::*;
use std::time::SystemTime;
use tokio_tungstenite::tungstenite::{client::ClientRequestBuilder, protocol::Message};
use tokio_tungstenite::{Connector, connect_async_tls_with_config};

#[tokio::main]
async fn main() -> Result<()> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let token = users_api::online_users_online_get(&conf, None).await?;
    println!("Token: {:#?}", token);

    let url = "wss://localhost/ws/room/abcd";

    // Build a TLS connector
    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let request = ClientRequestBuilder::new(url.parse()?)
        .with_header("Authorization", format!("Bearer {}", token.token.token));
    println!("Request: {:#?}", request);

    let (websocket, r) = connect_async_tls_with_config(
        request,
        None,
        false,
        Some(Connector::NativeTls(tls_connector)),
    )
    .await?;
    println!("Responce: {:#?}", r);

    // Split the websocket into a sender and receiver
    let (mut sink, mut stream) = websocket.split();
    let send_at = DateTime::<Utc>::from(SystemTime::now());
    let msg = MessageSend {
        content: MessageType::Plaintext(Plaintext::new("Hello World".to_owned())),
        send_at,
        data: None,
    };

    // Sending a message
    sink.send(Message::Text(serde_json::to_string(&msg)?.into()))
        .await
        .expect("Failed to send message");

    // Receiving messages
    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => println!("Received: {:?}", msg),
            Err(e) => eprintln!("Error while receiving message: {:?}", e),
        }
    }
    Ok(())
}
