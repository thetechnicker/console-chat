use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::network::error::NetworkError;
use alkali::asymmetric::cipher::{self};
use alkali::mem::ReadOnly;
use alkali::symmetric::cipher::Key;
use derive_deref::{Deref, DerefMut};
use openapi::apis::configuration::Configuration;
use reqwest::Certificate;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

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
    pub room: RwLock<Option<String>>,
    pub conf: RwLock<Configuration>,
    pub asym_key: Option<Keypair>,
    pub keys: RwLock<HashMap<String, Key<ReadOnly>>>,
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
    pub fn new(args: Cli, config: Config) -> Result<Self, NetworkError> {
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

        Ok(Self {
            config: conf_copy,
            data: Arc::new(NetworkData {
                room: RwLock::new(None),
                conf: RwLock::new(conf),
                asym_key: cipher::Keypair::generate().ok().map(|k| k.into()),
                keys: RwLock::new(HashMap::new()),
            }),
            listen_thread: None,
        })
    }

    async fn join(&mut self, room: impl Into<String>) -> Result<()> {
        Ok(())
    }
    async fn leave(&mut self) -> Result<()> {
        Ok(())
    }
}

fn listen(data: Arc<NetworkData>, cancel_token: CancellationToken) -> Result<()> {
    Ok(())
}
