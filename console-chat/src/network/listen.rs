use crate::{
    event,
    network::{ApiError, NetworkEvent, client::*, encryption, messages},
};
use reqwest::Url;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use tokio_stream::{self, StreamExt};

pub type ListenTask = tokio::task::JoinHandle<Result<ListenData, ApiError>>;
pub type HandleMessagesTask = tokio::task::JoinHandle<Result<HandleMessagesData, ApiError>>;

#[derive(Debug, Clone)]
pub struct ListenData {
    client: reqwest::Client,
    url: url::Url,
    token: String,
    stop_flag: watch::Receiver<bool>,
    msg_queue_sender: UnboundedSender<messages::ServerMessage>,
    event_sender: event::NetworkEventSender,
}

impl ListenData {
    pub fn update(&mut self, url: Url, token: String) {
        self.url = url;
        self.token = token
    }
    pub fn new(
        client: reqwest::Client,
        url: Url,
        token: String,
        stop_flag: watch::Receiver<bool>,
        msg_queue_sender: UnboundedSender<messages::ServerMessage>,
        event_sender: event::NetworkEventSender,
    ) -> Self {
        Self {
            client,
            url,
            token,
            stop_flag,
            msg_queue_sender,
            event_sender,
        }
    }

    pub fn run(mut self) -> ListenTask {
        tokio::spawn(async move {
            let mut stop = self.stop_flag.clone();
            loop {
                let resp = self.send_listen_request();
                tokio::select! {
                    res=resp=>{
                        match res {
                            Err(e) => {
                                self.event_sender.send(e.into());
                            }
                            Ok(resp) => {
                                let output = self.handle_stream(resp).await;
                                if let Err(e) = output {
                                    self.event_sender.send(e.into());
                                }
                            }
                        }
                    }
                    _ = stop.changed()=>{
                        if *stop.borrow(){
                            break;
                        }
                    }
                }
            }

            Ok(self)
        })
    }

    async fn send_listen_request(&mut self) -> Result<reqwest::Response, ApiError> {
        let resp = self
            .client
            .get(self.url.clone())
            .query(&[("listen_seconds", &LISTEN_TIMEOUT.to_string())])
            .timeout(std::time::Duration::from_secs(LISTEN_TIMEOUT))
            .bearer_auth(self.token.clone())
            .send()
            .await?;

        Ok(handle_errors_raw(resp).await?)
    }

    async fn handle_stream(&mut self, resp: reqwest::Response) -> Result<(), ApiError> {
        let mut stream = resp.bytes_stream();
        let mut is_end = false;

        loop {
            let chunk = stream.next();
            tokio::select! {
                Some(chunk)=chunk=>{
                    log::debug!("Received Chunk {chunk:?}");
                    let chunk = match chunk {
                        Err(e) => {
                            log::debug!("Error Receiving chunk: {e:#?}");
                            break;
                        }
                        Ok(data) => data,
                    };

                    let s = str::from_utf8(&chunk)?;

                    log::debug!("chunk as string: {s}");

                    if s == "END" {
                        is_end = true;
                    }
                    if is_end {
                        continue;
                    }

                    let msg = match serde_json::from_str::<messages::ServerMessage>(s) {
                        Ok(msg) => Ok(msg),
                        //Making composite Error to include the responce string
                        Err(e) => Err(ApiError::from((e, s))),
                    }?;
                    let res = self.msg_queue_sender.send(msg);
                    if res.is_err() {
                        break;
                    }
                }
                _ = self.stop_flag.changed()=>{
                        break;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct HandleMessagesData {
    symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
    asymetric_key: Arc<Mutex<encryption::KeyPair>>,
    event_sender: event::NetworkEventSender,
    msg_queue_sender: UnboundedSender<messages::ServerMessage>,
    msg_queue_receiver: UnboundedReceiver<messages::ServerMessage>,
}

impl HandleMessagesData {
    pub fn new(
        symetric_key: Arc<Mutex<Option<encryption::SymetricKey>>>,
        asymetric_key: Arc<Mutex<encryption::KeyPair>>,
        event_sender: event::NetworkEventSender,
        msg_queue_sender: UnboundedSender<messages::ServerMessage>,
        msg_queue_receiver: UnboundedReceiver<messages::ServerMessage>,
    ) -> Self {
        Self {
            symetric_key,
            asymetric_key,
            event_sender,
            msg_queue_sender,
            msg_queue_receiver,
        }
    }

    fn handle_system_msg(&self, msg: messages::ServerMessage) -> Result<NetworkEvent, ApiError> {
        if msg.base.message_type != messages::MessageType::System {
            return Err("".into());
        }
        if let Some(data) = msg.base.data {
            if data.contains_key("online") {
                if let Some(online) = data.get("online").unwrap().as_number() {
                    if let Some(num_online) = online.as_u64() {
                        if num_online == 1 {
                            return Ok(NetworkEvent::CreateKey);
                        }
                    }
                }
                return Ok(NetworkEvent::RequestKeyExchange);
            }
        }
        Err("No Data given".into())
    }

    fn handle_key_request(&self, msg: messages::ServerMessage) -> Result<NetworkEvent, ApiError> {
        if let Some(data) = msg.base.data {
            if data.contains_key("key") {
                if let Some(key) = data.get("key").unwrap().as_str() {
                    let received_key = encryption::from_base64(key)?;
                    let mut key = encryption::PublicKey::default();
                    for i in 0..key.len() {
                        key[i] = received_key[i];
                    }
                    return Ok(NetworkEvent::SendKey(key));
                }
            }
        }
        Err("No Data given".into())
    }

    fn handle_key_responce(&mut self, msg: messages::ServerMessage) -> Result<(), ApiError> {
        if self.symetric_key.lock().as_ref().is_ok_and(|x| x.is_some()) {
            return Ok(());
        }
        match msg.get_key_exchange_data() {
            Err(e) => self.event_sender.send(e.into()),
            Ok((public_key, nonce, sym_key, key_nonce)) => {
                let ref asym_key_guard = self.asymetric_key.lock().unwrap();
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

                    if let Ok(mut sym_key_guard) = self.symetric_key.lock() {
                        *sym_key_guard = Some(new_sym_key);
                    }
                } else {
                    return Err(
                        "The decoded text of the message differs from the expected value".into(),
                    );
                }
            }
        }
        return Ok(());
    }

    fn handle_message(&mut self, mut msg: messages::ServerMessage) -> Result<(), ApiError> {
        log::debug!("Received Message: {msg:#?}");
        match msg.base.message_type {
            messages::MessageType::System => {
                let event = self.handle_system_msg(msg)?;
                self.event_sender.send(event);
            }
            messages::MessageType::KeyRequest => {
                let event = self.handle_key_request(msg)?;
                self.event_sender.send(event);
            }
            messages::MessageType::Key => {
                self.handle_key_responce(msg)?;
            }
            messages::MessageType::Join => self.event_sender.send(NetworkEvent::Message(msg)),
            messages::MessageType::EncryptedText => {
                match self.symetric_key.lock().unwrap().as_ref() {
                    None => {
                        self.event_sender
                            .send(NetworkEvent::Error(ApiError::from("No Key")));
                        self.event_sender.send(NetworkEvent::RequestKeyExchange);
                        let _ = self.msg_queue_sender.send(msg);
                    }
                    Some(ref key) => {
                        let text = msg.base.text.clone();
                        if let Some(nonce_json) =
                            msg.base.data.as_ref().map_or(None, |h| h.get("nonce"))
                        {
                            let mut nonce: encryption::Nonce = encryption::Nonce::default();
                            let nonce_ref_v =
                                encryption::from_base64(nonce_json.as_str().unwrap())?;
                            for i in 0..nonce.len() {
                                nonce[i] = nonce_ref_v[i];
                            }
                            match encryption::decrypt((text, nonce), key) {
                                Err(e) => {
                                    self.event_sender.send(NetworkEvent::Error(e.into()));
                                    self.event_sender.send(NetworkEvent::RequestKeyExchange);
                                    if msg.base.data.is_none() {
                                        msg.base.data = Some(std::collections::HashMap::new());
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
                                        //let _ = self.msg_queue_receiver.send(msg);
                                    }
                                }
                                Ok(txt) => {
                                    msg.base.text = txt;
                                    self.event_sender.send(NetworkEvent::Message(msg));
                                }
                            }
                        } else {
                            if let Ok(mut sym_key_guard) = self.symetric_key.lock() {
                                *sym_key_guard = None;
                            }
                            self.event_sender.send(NetworkEvent::RequestKeyExchange);
                            self.event_sender
                                .send(NetworkEvent::Error("Can't decode Message".into()));
                        }
                    }
                }
            }
            _ => {
                self.event_sender.send(NetworkEvent::Message(msg));
            }
        }
        Ok(())
    }

    pub fn run(mut self) -> HandleMessagesTask {
        tokio::spawn(async move {
            while let Some(msg) = self.msg_queue_receiver.recv().await {
                self.handle_message(msg)?;
            }
            Ok(self)
        })
    }
}
