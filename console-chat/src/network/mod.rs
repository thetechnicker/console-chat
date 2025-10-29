//pub mod api;
pub mod client;
pub mod error;
pub mod user;
pub use error::*;

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    None,
    RequestReconnect,
    StrMessage(String),
    Message(user::ServerMessage),
    Error(ApiError),
}
