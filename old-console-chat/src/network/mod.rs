//pub mod api;
pub mod client;
pub mod data_model;
pub use data_model::*;
pub mod error;
pub use error::*;
pub mod listen;

#[allow(dead_code)]
pub mod encryption;

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    None,
    RequestReconnect,

    RequestKeyExchange,
    CreateKey,
    SendKey(encryption::PublicKey),

    StrMessage(String),
    Message(messages::ServerMessage),
    Error(ApiError),
    Leaf,
}
