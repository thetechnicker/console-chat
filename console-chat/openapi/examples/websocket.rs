use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use openapi::apis::{configuration, users_api};
use openapi::models::*;
use std::sync::Arc;
use std::time::SystemTime;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async, tungstenite::Error as WsError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let token = users_api::online_users_online_get(&conf, None).await?;
    println!("{:#?}", token);

    let url = "wss://localhost:8443/ws/room/abc";

    // Build a TLS connector
    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let (websocket, _) = connect_async(url, Some(Arc::new(tls_connector)))
        .await
        .map_err(|e| WsError::Io(e.into()))?;

    // Split the websocket into a sender and receiver
    let (mut sink, mut stream) = websocket.split();
    let send_at = DateTime::<Utc>::from(SystemTime::now());
    let msg = MessageSend {
        content: MessageType::Plaintext(Plaintext::new("Hello World".to_owned())),
        send_at,
        data: None,
    };

    // Sending a message
    sink.send(Message::Text(serde_json::to_string(&msg)?))
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
