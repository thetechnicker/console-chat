use crate::action::Action;
use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::network::error::NetworkError;
use alkali::asymmetric::cipher::{self, PUBLIC_KEY_LENGTH, PublicKey};
use alkali::mem::{FullAccess, ReadOnly};
use alkali::symmetric::cipher::{self as symetric_cipher, Key, NONCE_LENGTH};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use color_eyre::eyre::OptionExt;
use derive_deref::{Deref, DerefMut};
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use openapi::apis::Error as ApiError;
use openapi::apis::configuration::Configuration;
use openapi::apis::{rooms_api, users_api};
use openapi::models::*;
use reqwest::Certificate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic;
use std::sync::atomic::Ordering;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

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

#[derive(Default, Debug)]
pub(crate) struct NetworkData {
    pub room: atomic::AtomicPtr<String>,
    pub conf: RwLock<Configuration>,
    pub asym_key: Option<Keypair>,
    pub keys: RwLock<HashMap<String, Key<ReadOnly>>>,
}

lazy_static! {
    pub(crate) static ref CLIENT: OnceCell<NetworkClient> = OnceCell::new();
}

#[derive(Debug)]
struct Thread<T> {
    handle: JoinHandle<T>,
    cancel_token: CancellationToken,
}

#[derive(Default, Debug)]
pub(crate) struct NetworkClient {
    config: Config,
    data: Arc<NetworkData>,
    listen_thread: Option<Thread<Result<(), NetworkError>>>,
}

impl NetworkClient {
    pub fn init(args: Cli, config: Config) -> Result<bool, NetworkError> {
        let conf_copy = config.clone();
        let mut conf = Configuration::new();
        conf.base_path = config.network.host.as_str().to_owned();
        let mut builder = reqwest::ClientBuilder::new();

        builder = builder
            .danger_accept_invalid_hostnames(config.network.disable_hostname_verification)
            .danger_accept_invalid_certs(
                config.network.accept_danger || args.accept_invalid_certificate,
            );

        if let Some(ca_path) = config.network.ca_cert_path {
            if ca_path.is_file() {
                let ca = std::fs::read(&ca_path)?;
                builder = builder.add_root_certificate(Certificate::from_pem(ca.as_slice())?)
            }
        }

        conf.client = builder.build()?;

        Ok(CLIENT
            .set(Self {
                config: conf_copy,
                data: Arc::new(NetworkData {
                    room: atomic::AtomicPtr::new(std::ptr::null_mut()),
                    conf: RwLock::new(conf),
                    asym_key: cipher::Keypair::generate().ok().map(|k| k.into()),
                    keys: RwLock::new(HashMap::new()),
                }),
                listen_thread: None,
            })
            .is_ok())
    }
}
