use super::messages::*;
use crate::network::{data_model::user::PublicUser, encryption, error::NetworkError};
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub user: Option<PublicUser>,
}

impl ServerMessage {
    pub fn get_key_exchange_data(
        &self,
    ) -> Result<
        (
            encryption::PublicKey,
            encryption::Nonce,
            String,
            encryption::Nonce,
        ),
        NetworkError,
    > {
        if self.base.message_type != MessageType::Key {
            return Err(format!("ServerMessage ({self:?}) doesnt have key exchange data").into());
        }
        let public_key_vec = if let Some(key) = self.base.get_data_str("public_key") {
            Ok(encryption::from_base64(&key)?)
        } else {
            Err(NetworkError::from("public_key not given"))
        }?;
        let nonce_vec = if let Some(str) = self.base.get_data_str("nonce") {
            Ok(encryption::from_base64(&str)?)
        } else {
            Err(NetworkError::from("nonce not given"))
        }?;
        let sym_key_nonce_vec = if let Some(str) = self.base.get_data_str("key_nonce") {
            Ok(encryption::from_base64(&str)?)
        } else {
            Err(NetworkError::from("key_nonce not given"))
        }?;

        let mut public_key: encryption::PublicKey = encryption::PublicKey::default();
        public_key.copy_from_slice(public_key_vec.as_slice());

        let mut nonce: encryption::Nonce = encryption::Nonce::default();
        nonce.copy_from_slice(nonce_vec.as_slice());

        let mut key_nonce: encryption::Nonce = encryption::Nonce::default();
        key_nonce.copy_from_slice(sym_key_nonce_vec.as_slice());

        let sym_key = if let Some(str) = self.base.get_data_str("key") {
            Ok(str)
        } else {
            Err(NetworkError::from("key not given"))
        }?;

        Ok((public_key, nonce, sym_key, key_nonce))
    }
}
