//use crate::network::{data_model::messages::Message, error::NetworkError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::Display;

#[derive(Debug, Clone)]
pub enum AppError {
    MissingActionTX,
    MissingPassword,
    MissingUsername,
    MissingPasswordAndUsername,
    //    NetworkError(NetworkError),
    Error(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingActionTX => write!(f, "MissingActionTX"),
            Self::MissingPassword => write!(f, "MissingPassword"),
            Self::MissingUsername => write!(f, "MissingUsername"),
            Self::MissingPasswordAndUsername => write!(f, "MissingPasswordAndUsername"),
            //            Self::NetworkError(e) => write!(f, "Network Error: {e}"),
            Self::Error(s) => write!(f, "Error: {s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl PartialEq for AppError {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Error(e) => match other {
                Self::Error(o) => e == o,
                //               Self::NetworkError(error) => &format!("{error}") == e,
                Self::MissingActionTX => false,
                Self::MissingPassword => false,
                Self::MissingUsername => false,
                Self::MissingPasswordAndUsername => false,
            },
            //         Self::NetworkError(error) => match other {
            //             Self::Error(e) => &format!("{error}") == e,
            //             Self::NetworkError(e) => format!("{error}") == format!("{e}"),
            //             Self::MissingActionTX => false,
            //             Self::MissingPassword => false,
            //             Self::MissingUsername => false,
            //             Self::MissingPasswordAndUsername => false,
            //         },
            Self::MissingActionTX => match other {
                Self::Error(_) => false,
                //            Self::NetworkError(_) => false,
                Self::MissingActionTX => true,
                Self::MissingPassword => false,
                Self::MissingUsername => false,
                Self::MissingPasswordAndUsername => false,
            },
            Self::MissingPassword => {
                matches!(other, Self::MissingPassword)
            }
            Self::MissingUsername => {
                matches!(other, Self::MissingUsername)
            }
            Self::MissingPasswordAndUsername => {
                matches!(other, Self::MissingPasswordAndUsername)
            }
        }
    }
}
impl Eq for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // You said “just convert to string”
        serializer.serialize_str(&self.to_string())
    }
}

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

//impl From<NetworkError> for AppError {
//    fn from(s: NetworkError) -> Self {
//        AppError::NetworkError(s.to_owned())
//    }
//}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
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
    //ReceivedMessage(Message),
    Leave,

    SyncProfile,
    ReloadConfig,
}
