use crate::action::Action;
use crate::action::NetworkEvent;
use crate::network::Keys;
use crate::network::Result;
use crate::network::send_message;
use openapi::apis::configuration::Configuration;
use openapi::apis::rooms_api;
use openapi::apis::users_api;
use openapi::models::CreateRoom;
use openapi::models::LoginData;
use openapi::models::RoomLevel;
use openapi::models::Token;
use openapi::models::UpdateRoom;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

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
        info!("Refreshing access token...");
        let mut conf = self.conf.write().await;
        match users_api::users_online(&conf, None).await {
            Ok(response) => {
                debug!("Got token response: {:?}", response.token);
                let token = response.token.clone();
                self.id = Some(response.user);
                conf.bearer_access_token = Some(token.token.clone());
                let _ = self.sender_inner.send(NetworkEvent::RequestMe);
                info!("Token refreshed successfully (TTL: {}s)", token.ttl);
                Ok(response.token)
            }
            Err(err) => {
                error!("Failed to refresh token: {}", err);
                Err(err.into())
            }
        }
    }

    pub async fn request_me(&self) -> Result<()> {
        info!("Requesting current user info...");
        let conf = self.conf.read().await;
        match users_api::users_get_me(&conf).await {
            Ok(user) => {
                debug!("Fetched user info: {:?}", user);
                let _ = self.sender_main.send(Action::Me(user));
                Ok(())
            }
            Err(err) => {
                error!("Failed to get user info: {}", err);
                Err(err.into())
            }
        }
    }

    pub async fn send_msg(&self, msg: &str) -> Result<()> {
        if let Some((room, is_static)) = self.room.as_ref() {
            debug!("Sending message to room '{}' (static: {})", room, is_static);
            let conf = self.conf.read().await;
            let key_map = self.keys.symetric_keys.read().await;
            let symetric_key = key_map.get(room);
            send_message(&conf, room, *is_static, msg, symetric_key).await?;
            info!("Message sent to {}", room);
        } else {
            warn!("Tried to send message but no room is currently joined");
        }
        Ok(())
    }

    async fn login(&self, username: String, password: String) -> Result<()> {
        info!("Login in...");
        let mut conf = self.conf.write().await;
        let login_data = LoginData::new(username, password);
        match users_api::users_login(&conf, login_data).await {
            Ok(token) => {
                debug!("New id: {:?}", token.user);
                conf.bearer_access_token = Some(token.token.token.clone());
                info!("Token refreshed successfully (TTL: {}s)", token.token.ttl);
                let _ = self.sender_inner.send(NetworkEvent::RequestMe);
                let _ = self.sender_main.send(Action::OpenHome);
                Ok(())
            }
            Err(err) => {
                error!("Failed to get user info: {}", err);
                Err(err.into())
            }
        }
    }

    #[allow(unused_variables)]
    pub async fn handle_network_event(&self, event: NetworkEvent) -> Result<()> {
        debug!("Handling network event: {:?}", event);
        let result = match event.clone() {
            NetworkEvent::PerformLogin(username, password) => {
                //warn!("Login event handling not yet implemented for {}", username);
                self.login(username, password).await?;
                Ok(())
            }
            NetworkEvent::JoinRandom => self.join_random_room().await,
            NetworkEvent::RequestMe => self.request_me().await,
            NetworkEvent::SendMessage(msg) => self.send_msg(&msg).await,
            NetworkEvent::RequestMyRooms => self.request_rooms().await,
            NetworkEvent::CreateRoom(name, key, level) => self.create_room(name, key, level).await,
            NetworkEvent::DeleteRoom(name) => self.delete_room(&name).await,
            NetworkEvent::UpdateRoom(room, key, level) => self.update_room(&room, key, level).await,
            _ => {
                debug!("Unhandled network event: {:?}", event);
                Ok(())
            }
        };

        if let Err(err) = &result {
            error!("Error during network event {:?}: {}", event, err);
        }
        result
    }

    async fn update_room(
        &self,
        room: &str,
        key: Option<String>,
        level: Option<RoomLevel>,
    ) -> Result<()> {
        let conf = self.conf.read().await;
        let room_update = UpdateRoom {
            invite: None,
            key: key,
            private_level: level,
        };
        rooms_api::rooms_update_room(&conf, room, room_update).await?;
        Ok(())
    }

    async fn delete_room(&self, room: &str) -> Result<()> {
        let conf = self.conf.read().await;
        rooms_api::rooms_delete_room(&conf, room).await?;
        Ok(())
    }

    async fn request_rooms(&self) -> Result<()> {
        info!("Fetching list of rooms for current user...");
        let conf = self.conf.read().await;
        match rooms_api::rooms_get_my_rooms(&conf).await {
            Ok(rooms) => {
                debug!("Fetched {} rooms", rooms.len());
                let _ = self
                    .sender_main
                    .send(Action::MyRooms(Arc::from(rooms.into_boxed_slice())));
                Ok(())
            }
            Err(err) => {
                error!("Failed to fetch rooms: {}", err);
                Err(err.into())
            }
        }
    }

    pub async fn join_random_room(&self) -> Result<()> {
        info!("Attempting to join a random room...");
        let conf = self.conf.read().await;
        match rooms_api::rooms_random_room(&conf).await {
            Ok(room) => {
                info!("Joined random room: {:?}", room);
                let _ = self.sender_main.send(Action::PerformJoin(room, false));
                Ok(())
            }
            Err(err) => {
                error!("Failed to join random room: {}", err);
                Err(err.into())
            }
        }
    }

    pub async fn misc_loop(mut self, cancel_token: CancellationToken) -> Result<()> {
        info!("Starting misc loop...");
        let mut token_refresh_interval = match self.token_refresh().await {
            Ok(token) => tokio::time::interval(Duration::from_secs(token.ttl as u64)),
            Err(_) => tokio::time::interval(Duration::from_secs(10)),
        };

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Cancellation requested; stopping misc loop.");
                    break
                }
                _ = token_refresh_interval.tick() => {
                    debug!("Token refresh interval tick");
                    if let Err(e) = self.token_refresh().await {
                        error!("Failed to refresh token during misc loop: {}", e);
                    } else {
                        debug!("Token refreshed successfully in loop");
                    }
                }
                Some(event) = self.receiver.recv() => {
                    if let Err(e) = self.handle_network_event(event).await {
                        error!("Error while handling network event in misc loop: {}", e);
                    }
                }
                Ok(_) = self.room_rx.changed() => {
                    let (room, is_static) = self.room_rx.borrow_and_update().to_owned();
                    info!("Room changed to '{}' (static: {})", room, is_static);
                    self.room = Some((room, is_static));
                    self.room_rx.mark_unchanged();
                }
            }
        }

        info!("Misc loop exited cleanly");
        Ok(())
    }

    async fn create_room(&self, name: String, key: Option<String>, level: RoomLevel) -> Result<()> {
        info!("Creating new room: '{}' with level {:?}", name, level);

        let conf = self.conf.read().await;

        // Build CreateRoom model
        let mut create_room_data = CreateRoom::new(level);

        // Set key based on room level
        match level {
            RoomLevel::Key | RoomLevel::InviteAndKey => {
                if let Some(k) = key {
                    create_room_data.key = Some(k);
                }
            }
            _ => {
                // FREE or INVITE-ONLY don't need a key
                create_room_data.key = None;
            }
        }

        // Invites not implemented yet
        create_room_data.invite = None;

        match rooms_api::rooms_create_room(&conf, &name, create_room_data).await {
            Ok(_) => {
                info!("Successfully created room: {}", name);
                // Refresh the room list
                let _ = self.sender_inner.send(NetworkEvent::RequestMyRooms);
                Ok(())
            }
            Err(err) => {
                error!("Failed to create room '{}': {}", name, err);
                Err(err.into())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(dead_code)]
    fn test_attributes() {
        fn is_send<T: Send>() {}
        fn is_sync<T: Sync>() {}
        fn is_send_sync<T: Send + Sync>() {}
        is_send::<MiscThreadData>();
        is_sync::<MiscThreadData>();
        is_send_sync::<MiscThreadData>();
    }
}
