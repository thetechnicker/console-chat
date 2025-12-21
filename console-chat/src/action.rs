use crate::network::error::NetworkError;
use openapi::models::{MessagePublic, UserPrivate};
use serde::{Deserialize, Deserializer, Serialize};
use strum::Display;

pub(crate) type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Debug, Clone)]
pub enum AppError {
    MissingActionTX,
    MissingPassword,
    MissingUsername,
    MissingPasswordAndUsername,
    NetworkError(NetworkError),
    Error(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingActionTX => write!(f, "MissingActionTX"),
            Self::MissingPassword => write!(f, "MissingPassword"),
            Self::MissingUsername => write!(f, "MissingUsername"),
            Self::MissingPasswordAndUsername => write!(f, "MissingPasswordAndUsername"),
            Self::NetworkError(e) => write!(f, "Network Error: {e}"),
            Self::Error(s) => write!(f, "Error: {s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl PartialEq for AppError {
    fn eq(&self, _: &Self) -> bool {
        false // No error is equal
    }
}
impl Eq for AppError {}

impl<'de> Deserialize<'de> for AppError {
    fn deserialize<D>(deserializer: D) -> Result<AppError, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Simplest interpretation of your requirement:
        // always end up with a UiError(String).
        // If you *do* want to recover NetworkError, parse here instead.
        Ok(AppError::Error(s))
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Error(s.to_owned())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Error(s)
    }
}

impl From<NetworkError> for AppError {
    fn from(s: NetworkError) -> Self {
        AppError::NetworkError(s.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    #[serde(skip)]
    Error(AppError),
    Help,

    Insert,
    Normal,

    //  Open
    OpenLogin,
    OpenSettings,
    OpenRawSettings,
    OpenChat,
    OpenJoin,
    OpenHome,
    Hide,

    TriggerLogin,
    PerformLogin(String, String),
    TriggerJoin,
    PerformJoin(String),
    SendMessage(String),
    Me(UserPrivate),
    ReceivedMessage(MessagePublic),
    Leave,

    SyncProfile,
    ReloadConfig,
}
