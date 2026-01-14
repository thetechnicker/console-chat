use super::*;
use crate::action::Action;
use crate::action::NetworkEvent;
use crate::cli::Cli;
use crate::config::Config;
use crate::network::Result;
use alkali::asymmetric::cipher::{self};
use alkali::mem::ReadOnly;
use alkali::symmetric::cipher::Key;
use derive_deref::{Deref, DerefMut};
use openapi::apis::configuration::Configuration;
use openapi::apis::users_api;
use openapi::models::Token;
use reqwest::Certificate;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;

#[derive(Deref, DerefMut)]
pub(crate) struct Keypair(pub cipher::Keypair);
impl From<cipher::Keypair> for Keypair {
    fn from(c: cipher::Keypair) -> Keypair {
        Keypair(c)
    }
}

impl std::fmt::Debug for Keypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keypair")
            .field("private_key", &"*".repeat(self.private_key.len()))
            .field("public_key", &self.public_key)
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct Keys {
    pub symetric_keys: Mutex<HashMap<String, Key<ReadOnly>>>, // Protect with Mutex
    pub asymetric_keys: Option<Keypair>,
}

#[derive(Debug)]
pub struct MiscThreadData {
    pub conf: Arc<Mutex<Configuration>>, // Use Arc<Mutex> for shared access
    pub sender_main: UnboundedSender<Action>, // Thread-local; no protection needed
    pub sender_inner: UnboundedSender<NetworkEvent>, // Thread-local; no protection needed
    pub receiver: UnboundedReceiver<NetworkEvent>, // Thread-local; no protection needed
}

#[derive(Debug)]
pub struct ListenThreadData {
    pub room: String,
    pub keys: Arc<Keys>,                 // Shared ownership via Arc
    pub conf: Arc<Mutex<Configuration>>, // Use Arc<Mutex> for shared access
    pub sender: UnboundedSender<Action>, // Thread-local; no protection needed
}

#[derive(Debug)]
pub struct ThreadManagement<T> {
    pub join_handle: JoinHandle<T>,
    pub cancellation_token: CancellationToken,
}

#[derive(Debug)]
pub struct NetworkStack {
    room: Option<String>,
    keys: Arc<Keys>,                             // Shared ownership via Arc
    conf: Arc<Mutex<Configuration>>,             // Use Arc<Mutex> to allow controlled access
    sender_inner: UnboundedSender<NetworkEvent>, // Thread-local; no protection needed
    sender_main: UnboundedSender<Action>,        // Thread-local; no protection needed

    listen_thread: Option<ThreadManagement<Result<()>>>,
    misc_thread: ThreadManagement<Result<()>>,
}

impl NetworkStack {
    pub fn new(cli: Cli, config: Config, sender: UnboundedSender<Action>) -> Result<Self> {
        debug!("Network config: {:#?}", config.network);
        let mut conf = Configuration::new();

        conf.base_path = config.network.host.clone();

        let mut builder = reqwest::ClientBuilder::new();

        builder = builder
            .danger_accept_invalid_hostnames(config.network.disable_hostname_verification)
            .danger_accept_invalid_certs(
                config.network.accept_danger || cli.accept_invalid_certificate,
            );

        if let Some(ca_path) = config.network.ca_cert_path {
            if ca_path.is_file() {
                let ca_vec = std::fs::read(&ca_path)?;
                let ca = Certificate::from_pem(ca_vec.as_slice())?;
                builder = builder.add_root_certificate(ca)
            }
        }
        debug!("Client Builder: {builder:#?}");
        conf.client = builder.build()?;
        debug!("Config: {conf:#?}");

        let conf = Arc::new(Mutex::new(conf));

        let (sender_a, receiver_a) = unbounded_channel();

        let misc_data = MiscThreadData {
            conf: Arc::clone(&conf),
            sender_main: sender.clone(),
            sender_inner: sender_a.clone(),
            receiver: receiver_a,
        };
        let cancel_token = CancellationToken::new();
        let misc_thread = tokio::spawn(misc_loop(misc_data, cancel_token.clone()));
        let misc_thread_manager = ThreadManagement {
            join_handle: misc_thread,
            cancellation_token: cancel_token,
        };

        Ok(Self {
            room: None,
            conf,
            keys: Arc::new(Keys::default()),
            listen_thread: None,
            misc_thread: misc_thread_manager,
            sender_main: sender.clone(),
            sender_inner: sender_a,
        })
    }
}

async fn token_refresh(data: &MiscThreadData) -> Result<Token> {
    let mut conf = data.conf.lock().await;
    let response = users_api::users_online(&conf, None).await?;
    let token = response.token.clone();
    conf.bearer_access_token = Some(token.token);
    send_no_err(&data.sender_inner, NetworkEvent::RequestMe(response.user));
    Ok(response.token)
}

async fn handle_network_event(_event: NetworkEvent, _data: &MiscThreadData) -> Result<()> {
    Ok(())
}

async fn misc_loop(mut data: MiscThreadData, cancel_token: CancellationToken) -> Result<()> {
    let token = token_refresh(&data).await?;
    let mut token_refresh_interval = tokio::time::interval(Duration::from_secs(token.ttl as u64));
    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                break
            }
            _ = token_refresh_interval.tick() => {
                let token = token_refresh(&data).await?;
                if token_refresh_interval.period().as_secs() != token.ttl as u64 {
                    token_refresh_interval = tokio::time::interval(Duration::from_secs(token.ttl as u64));
                }
            }
            Some(event) = data.receiver.recv()=>{
                handle_network_event(event, &data).await?;
            }
        }
    }
    Ok(())
}
