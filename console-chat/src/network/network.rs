use super::Keys;
use super::listen_thread::ListenThreadData;
use super::misc_thread::MiscThreadData;
use crate::action::Action;
use crate::action::NetworkEvent;
use crate::cli::Cli;
use crate::config::Config;
use crate::network::Result;
use openapi::apis::configuration::Configuration;
use openapi::models::UserPrivate;
use reqwest::Certificate;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::watch::Sender;
use tokio::sync::watch::channel;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;

#[derive(Debug)]
pub struct ThreadManagement<T> {
    pub join_handle: JoinHandle<T>,
    pub cancellation_token: CancellationToken,
}

#[derive(Debug)]
pub struct NetworkStack {
    me: Option<UserPrivate>,
    keys: Arc<Keys>,                             // Shared ownership via Arc
    conf: Arc<RwLock<Configuration>>,            // Use Arc<Mutex> to allow controlled access
    sender_inner: UnboundedSender<NetworkEvent>, // Thread-local; no protection needed
    sender_main: UnboundedSender<Action>,        // Thread-local; no protection needed

    signal: Arc<Notify>,
    room_tx: Sender<(String, bool)>,

    listen_thread: Option<ThreadManagement<Result<()>>>,
    misc_thread: ThreadManagement<Result<()>>,
}

impl NetworkStack {
    pub fn new(cli: Cli, config: Config, sender: UnboundedSender<Action>) -> Result<Self> {
        debug!("Network config: {:#?}", config.network);
        let mut conf = Configuration::new();
        let signal = Arc::new(Notify::new());

        conf.base_path = config.network.host.clone();

        let mut builder = reqwest::ClientBuilder::new();

        builder = builder
            .danger_accept_invalid_hostnames(config.network.disable_hostname_verification)
            .danger_accept_invalid_certs(
                config.network.accept_danger || cli.accept_invalid_certificate,
            );

        if let Some(ca_path) = config.network.ca_cert_path
            && ca_path.is_file()
        {
            let ca_vec = std::fs::read(&ca_path)?;
            let ca = Certificate::from_pem(ca_vec.as_slice())?;
            builder = builder.add_root_certificate(ca)
        }
        debug!("Client Builder: {builder:#?}");
        conf.client = builder.build()?;
        debug!("Config: {conf:#?}");

        let conf = Arc::new(RwLock::new(conf));

        let (sender_a, receiver_a) = unbounded_channel();
        let (room_tx, room_rx) = channel((String::new(), false));

        let misc_data = MiscThreadData::new(
            Arc::clone(&conf),
            signal.clone(),
            room_rx,
            sender.clone(),
            sender_a.clone(),
            receiver_a,
        );

        let cancel_token = CancellationToken::new();
        let misc_token = cancel_token.clone();
        let misc_thread = tokio::spawn(async move { misc_data.misc_loop(misc_token).await });
        let misc_thread_manager = ThreadManagement {
            join_handle: misc_thread,
            cancellation_token: cancel_token,
        };

        Ok(Self {
            room_tx,
            signal,
            conf,
            me: None,
            keys: Arc::new(Keys::default()),
            listen_thread: None,
            misc_thread: misc_thread_manager,
            sender_main: sender.clone(),
            sender_inner: sender_a,
        })
    }

    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        if let Ok(network_event) = action.try_into() {
            match network_event {
                NetworkEvent::PerformJoin(room, is_static) => self.join(room, is_static)?,
                NetworkEvent::Me(me) => self.me = Some(me),
                _ => {
                    let _ = self.sender_inner.send(network_event);
                }
            }
        }
        Ok(())
    }

    pub fn join(&mut self, room: String, is_static: bool) -> Result<()> {
        let Some(me) = self.me.clone() else {
            let _ = self.sender_inner.send(NetworkEvent::RequestMe);
            return Ok(());
        };
        let listen_thread_data = ListenThreadData::new(
            is_static,
            room,
            self.keys.clone(),
            self.signal.clone(),
            self.room_tx.clone(),
            me,
            self.conf.clone(),
            self.sender_main.clone(),
        );
        let cancellation_token = CancellationToken::new();
        let listen_thread = ThreadManagement {
            join_handle: tokio::spawn(listen_thread_data.run(cancellation_token.clone())),
            cancellation_token,
        };
        self.listen_thread = Some(listen_thread);
        Ok(())
    }
}

impl Drop for NetworkStack {
    fn drop(&mut self) {
        self.misc_thread.cancellation_token.cancel();
        self.misc_thread.join_handle.abort();
        if let Some(listen_thread) = self.listen_thread.take() {
            listen_thread.cancellation_token.cancel();
            listen_thread.join_handle.abort();
        }
    }
}
