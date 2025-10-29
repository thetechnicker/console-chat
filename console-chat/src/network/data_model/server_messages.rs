use super::messages::*;
use crate::network::user::PublicUser;
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub user: Option<PublicUser>,
}
