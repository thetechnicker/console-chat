use super::client::*;
use super::data_model::{messages, user::*};
use super::encryption;
use super::error::*;
use crate::action::Action;
use color_eyre::Result;
use futures::StreamExt;
use reqwest;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};
use url::Url;

pub const LISTEN_TIMEOUT: u64 = 30;

pub async fn listen(cancellation_token: CancellationToken) -> Result<(), NetworkError> {
    if let Some(client_lock) = CLIENT.get() {
        let client = client_lock.lock().await.clone();
        let room = client.room.clone().ok_or(NetworkError::NoRoom)?.clone();
        let token = client.token.clone().ok_or(NetworkError::MissingAuthToken)?;
        let (msg_tx, msg_rx) = unbounded_channel();
        let task_msg_tx = msg_tx.clone();
        let task_cancellation_token = cancellation_token.clone();
        let msg_handler = tokio::spawn(async move {
            handle_messages_async(task_cancellation_token, msg_rx, task_msg_tx).await
        });

        loop {
            trace!("sending listen Request");
            let responce = send_listen_request(
                client.client.clone(),
                client.url.join(&format!("room/{room}"))?,
                token.clone(),
            )
            .await?;
            trace!("got responce, starting stream");
            let mut stream = responce.bytes_stream();
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    debug!("listen worker cancelled");
                    break;
                }
                Some(chunk)=stream.next()=>{
                    debug!("Received Chunk {chunk:?}");
                    let chunk = match chunk {
                        Err(e) => {
                            error!("Error Receiving chunk: {e:#?}");
                            continue;
                        }
                        Ok(data) => data,
                    };

                    let s = str::from_utf8(&chunk)?;

                    debug!("chunk as string: {s}");

                    if s == "END" {
                        continue;
                    }

                    let msg = match serde_json::from_str::<messages::ServerMessage>(s) {
                        Ok(msg) => Ok(msg),
                        Err(e) => Err(NetworkError::from((e, s))),
                    };
                    match msg{
                        Ok(msg)=>{
                            debug!("Got message: {msg:#?}\n{}", if msg.base.is_mine(){"is mine"}else{"from someone else"});
                            client_lock.lock().await.action_tx.send(Action::ReceivedMessage(msg));
                            //handle_message(client_lock, msg).await?;
                            //let _=msg_tx.send(msg);
                        }
                        Err(e)=>{error!("{e}");continue},
                    }
                }
            }
        }
        msg_handler.await?;
    }
    Ok(())
}

async fn handle_messages_async(
    cancellation_token: CancellationToken,
    mut msg_rx: UnboundedReceiver<messages::ServerMessage>,
    msg_tx: UnboundedSender<messages::ServerMessage>,
) {
    if let Some(client_lock) = CLIENT.get() {
        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    debug!("listen worker cancelled");
                    break;
                }
                Some(msg) = msg_rx.recv()=>{
                    handle_message_intermediat(&client_lock, msg).await;
                }
            }
        }
    }
}

async fn handle_message_intermediat(client: &Mutex<Client>, msg: messages::ServerMessage) {
    if let Err(e) = handle_message(client, msg).await {
        let _ = client.lock().await.action_tx.send(Action::Error(e.into()));
    }
}

async fn request_key(client_lock: &Mutex<Client>) -> Result<(), NetworkError> {
    let client = client_lock.lock().await;
    let room = client.room.clone().ok_or(NetworkError::NoRoom)?;
    let url = client.url.join(&format!("room/{room}"))?;

    let key_guard = client.asymetric_key.lock().await;
    let msg = messages::ClientMessage::key_request(key_guard.public_key());
    let body = serde_json::json!(msg);

    let resp = client
        .post(url)
        .json(&body)
        .bearer_auth(
            client
                .token
                .clone()
                .ok_or(NetworkError::MissingAuthToken)?
                .token,
        )
        .send()
        .await?;

    let message: messages::ServerMessage = handle_errors_json(resp).await?;
    debug!("{:?}", message);
    Ok(())
}

async fn set_new_sym_key(client_lock: &Mutex<Client>) -> Result<(), NetworkError> {
    let client = client_lock.lock().await;
    let mut key_guard = client.symetric_key.lock().await;
    *key_guard = Some(encryption::get_new_symetric_key()?);
    Ok(())
}

pub async fn send_listen_request(
    client: reqwest::Client,
    url: Url,
    token: Token,
) -> Result<reqwest::Response, NetworkError> {
    let resp = client
        .get(url)
        .query(&[("listen_seconds", &LISTEN_TIMEOUT.to_string())])
        .timeout(std::time::Duration::from_secs(LISTEN_TIMEOUT))
        .bearer_auth(token.token)
        .send()
        .await?;

    Ok(handle_errors_raw(resp).await?)
}

pub async fn handle_message(
    client_lock: &Mutex<Client>,
    mut msg: messages::ServerMessage,
) -> Result<(), NetworkError> {
    debug!("Received Message: {msg:#?}");
    match msg.base.message_type {
        messages::MessageType::System => {
            if let Some(data) = msg.base.data {
                if data.contains_key("online") {
                    if let Some(online) = data.get("online").unwrap().as_number() {
                        if let Some(num_online) = online.as_u64() {
                            if num_online == 1 {
                                set_new_sym_key(client_lock).await?;
                            }
                        }
                    }
                    request_key(client_lock).await?;
                }
            }
        }
        messages::MessageType::KeyRequest => {
            if let Some(data) = msg.base.data {
                if data.contains_key("key") {
                    if let Some(key) = data.get("key").unwrap().as_str() {
                        let received_key = encryption::from_base64(key)?;
                        let mut key = encryption::PublicKey::default();
                        for i in 0..key.len() {
                            key[i] = received_key[i];
                        }
                        return Ok(());
                    }
                }
            }
            return Err("No Data given".into());
        }
        messages::MessageType::Key => {
            let client = client_lock.lock().await;
            if client.symetric_key.lock().await.as_ref().is_some() {
                return Ok(());
            }
            let (public_key, nonce, sym_key, key_nonce) = msg.get_key_exchange_data()?;
            let ref asym_key_guard = client.asymetric_key.lock().await;
            if public_key == asym_key_guard.public_key() {
                return Ok(());
            }
            let text =
                encryption::decrypt_asym((msg.base.text, nonce), asym_key_guard, public_key)?;

            if text == messages::ASYM_KEY_CHECK {
                let decoded_key =
                    encryption::decrypt_asym((sym_key, key_nonce), asym_key_guard, public_key)?;

                let decrypted_key = encryption::from_base64(&decoded_key)?;

                let mut new_sym_key: encryption::SymetricKey =
                    encryption::SymetricKey::new_empty()?;

                for i in 0..new_sym_key.len() {
                    new_sym_key[i] = decrypted_key[i];
                }

                let mut sym_key_guard = client.symetric_key.lock().await;
                *sym_key_guard = Some(new_sym_key);
            } else {
                return Err(
                    "The decoded text of the message differs from the expected value".into(),
                );
            }
            return Ok(());
        }
        messages::MessageType::Join => {
            let _ = client_lock
                .lock()
                .await
                .action_tx
                .send(Action::ReceivedMessage(msg));
        }
        messages::MessageType::EncryptedText => {
            match client_lock.lock().await.symetric_key.lock().await.as_ref() {
                None => {
                    request_key(client_lock).await?;
                }
                Some(ref key) => {
                    let text = msg.base.text.clone();
                    if let Some(nonce_json) =
                        msg.base.data.as_ref().map_or(None, |h| h.get("nonce"))
                    {
                        let mut nonce: encryption::Nonce = encryption::Nonce::default();
                        let nonce_ref_v = encryption::from_base64(nonce_json.as_str().unwrap())?;
                        for i in 0..nonce.len() {
                            nonce[i] = nonce_ref_v[i];
                        }
                        match encryption::decrypt((text, nonce), key) {
                            Err(_) => {
                                let client = client_lock.lock().await;
                                let mut sym_key_guard = client.symetric_key.lock().await;
                                *sym_key_guard = None;
                                request_key(client_lock).await?;
                            }
                            Ok(txt) => {
                                msg.base.text = txt;
                                let _ = client_lock
                                    .lock()
                                    .await
                                    .action_tx
                                    .send(Action::ReceivedMessage(msg));
                            }
                        }
                    } else {
                        return Err("Can't decode Message".into());
                    }
                }
            }
        }
        _ => {
            let _ = client_lock
                .lock()
                .await
                .action_tx
                .send(Action::ReceivedMessage(msg));
        }
    }
    Ok(())
}
