use crate::action::Action;
use crate::action::NetworkEvent;
use crate::network::Result;
use crate::network::send_message;
use alkali::mem::ReadOnly;
use alkali::symmetric::cipher::Key;
use openapi::apis::configuration::Configuration;
use openapi::apis::users_api;
use openapi::models::Token;
use openapi::models::UserPrivate;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch::Receiver;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct MiscThreadData {
    me: Arc<Mutex<Option<UserPrivate>>>,
    conf: Arc<RwLock<Configuration>>, // Use Arc<Mutex> for shared access

    used_key: Option<Key<ReadOnly>>,

    signal: Arc<Notify>,
    room_rx: Receiver<String>,
    room: Option<String>,
    sender_main: UnboundedSender<Action>, // Thread-local; no protection needed
    sender_inner: UnboundedSender<NetworkEvent>, // Thread-local; no protection needed
    receiver: UnboundedReceiver<NetworkEvent>, // Thread-local; no protection needed
}

impl MiscThreadData {
    pub fn new(
        me: Arc<Mutex<Option<UserPrivate>>>,
        conf: Arc<RwLock<Configuration>>,
        signal: Arc<Notify>,
        room_rx: Receiver<String>,
        sender_main: UnboundedSender<Action>,
        sender_inner: UnboundedSender<NetworkEvent>,
        receiver: UnboundedReceiver<NetworkEvent>,
    ) -> Self {
        Self {
            me,
            conf,
            room_rx: room_rx,
            room: None,
            used_key: None, // Default value for non-public field
            signal,
            sender_main,
            sender_inner,
            receiver,
        }
    }

    pub async fn token_refresh(&self) -> Result<Token> {
        let mut conf = self.conf.write().await;
        let response = users_api::users_online(&conf, None).await?;
        let token = response.token.clone();
        conf.bearer_access_token = Some(token.token);
        let _ = self
            .sender_inner
            .send(NetworkEvent::RequestMe(response.user));
        Ok(response.token)
    }

    #[allow(unused_variables)]
    pub async fn handle_network_event(&self, event: NetworkEvent) -> Result<()> {
        match event {
            NetworkEvent::PerformLogin(username, password) => todo!(),
            NetworkEvent::RequestMe(id) => todo!(),
            NetworkEvent::SendMessage(msg) => {
                todo!();
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn misc_loop(mut self, cancel_token: CancellationToken) -> Result<()> {
        let token = self.token_refresh().await?;
        let mut token_refresh_interval =
            tokio::time::interval(Duration::from_secs(token.ttl as u64));
        loop {
            tokio::select! {
                _ = self.signal.notified() => {
                }
                _ = cancel_token.cancelled() => {
                    break
                }
                _ = token_refresh_interval.tick() => {
                    let token = self.token_refresh().await?;
                    if token_refresh_interval.period().as_secs() != token.ttl as u64 {
                        token_refresh_interval = tokio::time::interval(Duration::from_secs(token.ttl as u64));
                    }
                }
                Some(event) = self.receiver.recv()=>{
                    self.handle_network_event(event ).await?;
                }
                Ok(room)=self.room_rx.changed()=>{
                    self.room=Some(self.room_rx.borrow_and_update().to_owned());
                    self.room_rx.mark_unchanged();
                }
            }
        }
        Ok(())
    }
}

fn test_attributes<T>() {
    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}
    fn is_send_sync<T: Send + Sync>() {}
    is_send::<MiscThreadData>();
    is_sync::<MiscThreadData>();
    is_send_sync::<MiscThreadData>();
}
