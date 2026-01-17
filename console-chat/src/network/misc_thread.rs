use crate::action::Action;
use crate::action::NetworkEvent;
use crate::network::Keys;
use crate::network::Result;
use crate::network::send_message;
use openapi::apis::configuration::Configuration;
use openapi::apis::rooms_api;
use openapi::apis::users_api;
use openapi::models::Token;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch::Receiver;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct MiscThreadData {
    conf: Arc<RwLock<Configuration>>, // Use Arc<Mutex> for shared access
    id: Option<uuid::Uuid>,

    keys: Arc<Keys>, // Shared ownership via Arc

    room_rx: Receiver<(String, bool)>,
    room: Option<(String, bool)>,

    sender_main: UnboundedSender<Action>, // Thread-local; no protection needed
    sender_inner: UnboundedSender<NetworkEvent>, // Thread-local; no protection needed
    receiver: UnboundedReceiver<NetworkEvent>, // Thread-local; no protection needed
}

impl MiscThreadData {
    pub fn new(
        conf: Arc<RwLock<Configuration>>,
        keys: Arc<Keys>, // Shared ownership via Arc
        room_rx: Receiver<(String, bool)>,
        sender_main: UnboundedSender<Action>,
        sender_inner: UnboundedSender<NetworkEvent>,
        receiver: UnboundedReceiver<NetworkEvent>,
    ) -> Self {
        Self {
            conf,
            room_rx,
            keys,
            room: None,
            id: None,
            sender_main,
            sender_inner,
            receiver,
        }
    }

    pub async fn token_refresh(&mut self) -> Result<Token> {
        let mut conf = self.conf.write().await;
        let response = users_api::users_online(&conf, None).await?;
        let token = response.token.clone();
        self.id = Some(response.user);
        conf.bearer_access_token = Some(token.token);
        let _ = self.sender_inner.send(NetworkEvent::RequestMe);
        Ok(response.token)
    }

    pub async fn request_me(&self) -> Result<()> {
        let conf = self.conf.read().await;
        let user = users_api::users_get_me(&conf).await?;
        let _ = self.sender_main.send(Action::Me(user));
        Ok(())
    }
    pub async fn send_msg(&self, msg: &str) -> Result<()> {
        if let Some((room, is_static)) = self.room.as_ref() {
            let conf = self.conf.read().await;
            let key_map = self.keys.symetric_keys.read().await;
            let symetric_key = key_map.get(room);
            send_message(&conf, room, *is_static, msg, symetric_key).await?;
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn handle_network_event(&self, event: NetworkEvent) -> Result<()> {
        match event {
            NetworkEvent::PerformLogin(username, password) => todo!(),
            NetworkEvent::JoinRandom => self.join_random_room().await?,
            NetworkEvent::RequestMe => self.request_me().await?,
            NetworkEvent::SendMessage(msg) => self.send_msg(&msg).await?,
            _ => {}
        }
        Ok(())
    }

    pub async fn join_random_room(&self) -> Result<()> {
        let conf = self.conf.read().await;
        let room = rooms_api::rooms_random_room(&conf).await?;
        let _ = self.sender_main.send(Action::PerformJoin(room, false));
        Ok(())
    }

    pub async fn misc_loop(mut self, cancel_token: CancellationToken) -> Result<()> {
        let token = self.token_refresh().await?;
        let mut token_refresh_interval =
            tokio::time::interval(Duration::from_secs(token.ttl as u64));
        loop {
            tokio::select! {
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
                Ok(_)=self.room_rx.changed()=>{
                    self.room=Some(self.room_rx.borrow_and_update().to_owned());
                    self.room_rx.mark_unchanged();
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(dead_code)]
    fn test_attributes<T>() {
        fn is_send<T: Send>() {}
        fn is_sync<T: Sync>() {}
        fn is_send_sync<T: Send + Sync>() {}
        is_send::<MiscThreadData>();
        is_sync::<MiscThreadData>();
        is_send_sync::<MiscThreadData>();
    }
}
