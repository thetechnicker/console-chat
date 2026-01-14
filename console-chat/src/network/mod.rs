pub(crate) mod error;
pub(crate) mod message;
pub(crate) mod network;

pub(crate) use message::*;

use base64::{Engine as _, engine::general_purpose};
use tokio::sync::mpsc::UnboundedSender;

type Result<T, E = error::NetworkError> = std::result::Result<T, E>;

pub(self) fn send_no_err<T>(sender: &UnboundedSender<T>, message: T) {
    let _ = sender.send(message);
}

pub(self) fn to_base64(arg: &[u8]) -> String {
    general_purpose::STANDARD.encode(arg)
}

pub(self) fn from_base64(arg: &str) -> Result<Vec<u8>> {
    Ok(general_purpose::STANDARD.decode(arg)?)
}
