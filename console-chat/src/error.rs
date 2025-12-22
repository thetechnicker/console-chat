use crate::network::error::NetworkError;
use serde::{Deserialize, Deserializer};
use std::sync::Arc;
//use strum::Display;

pub(crate) type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Debug, Clone)]
pub enum AppError {
    MissingActionTX,
    MissingPassword,
    MissingUsername,
    MissingPasswordAndUsername,
    Eyre(Arc<color_eyre::Report>),
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
            Self::Eyre(e) => write!(f, "Eyre: {e}"),
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

impl From<serde_json::Error> for AppError {
    fn from(s: serde_json::Error) -> Self {
        AppError::Eyre(Arc::new(s.into()))
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for AppError
where
    T: std::marker::Send + std::marker::Sync + 'static,
{
    fn from(s: tokio::sync::mpsc::error::SendError<T>) -> Self {
        AppError::Eyre(Arc::new(s.into()))
    }
}

impl From<std::io::Error> for AppError {
    fn from(s: std::io::Error) -> Self {
        AppError::Eyre(Arc::new(s.into()))
    }
}

impl From<color_eyre::Report> for AppError {
    fn from(s: color_eyre::Report) -> Self {
        AppError::Eyre(Arc::new(s))
    }
}

//impl<T> From<T> for AppError
//where
//    T: std::error::Error + 'static + std::marker::Send + std::marker::Sync,
//{
//    fn from(err: T) -> Self {
//        AppError::Eyre(Arc::new(err.into()))
//    }
//}
