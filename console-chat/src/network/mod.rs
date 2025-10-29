//pub mod api;
pub mod client;
pub mod data_model;
pub use data_model::*;
pub mod error;
pub use error::*;

#[allow(dead_code)]
pub mod encryption;

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    None,
    RequestReconnect,
    StrMessage(String),
    Message(messages::ServerMessage),
    Error(ApiError),
}
