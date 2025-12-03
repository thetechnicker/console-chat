use alkali::AlkaliError;
use color_eyre::Report;
use std::error::Error;
use std::sync::Arc;
use tokio::task::JoinError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResponseErrorData {
    pub msg: String,
    pub status: reqwest::StatusCode,
    pub url: url::Url,
}

impl std::fmt::Display for ResponseErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} ({})", self.status, self.msg, self.url)
    }
}

#[derive(Debug, Clone)]
pub enum NetworkError {
    NoRoom,
    MissingAuthToken,
    BadKeyVerification,
    MissingEncryptionData,

    GenericError(String),
    UrlParseError(url::ParseError),

    Unauthorized(ResponseErrorData),
    NotFound(ResponseErrorData),
    ClientError(ResponseErrorData),
    ServerError(ResponseErrorData),

    ReqwestError(Arc<reqwest::Error>),

    Utf8Error(std::str::Utf8Error),
    Base64DecodeError(base64::DecodeError),
    SerdeError(Arc<serde_json::Error>),
    AlkaliError(AlkaliError),
    CompositError(Arc<NetworkError>, String),

    JoinError(Arc<JoinError>),
    Eyre(Arc<Report>),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NetworkError::NoRoom => write!(f, "Error: You haven't joined a room."),
            NetworkError::MissingAuthToken => {
                write!(f, "Error: The authentication token isn't set.")
            }
            NetworkError::BadKeyVerification => {
                write!(
                    f,
                    "Error: The received key verification message didn't match."
                )
            }
            NetworkError::MissingEncryptionData => {
                write!(
                    f,
                    "Error: Message is marked as containing encrypted data, but essential components are missing. \
                     Expected nonce, encrypted symmetric key, or other required data were not found."
                )
            }
            NetworkError::GenericError(msg) => write!(f, "Error: {}", msg),
            NetworkError::UrlParseError(e) => write!(f, "Error: URL parse error - {}", e),
            NetworkError::ReqwestError(e) => write!(f, "Error: Request error - {}", e),
            NetworkError::ClientError(data) => write!(f, "Error: Client error - HTTP {}", data),
            NetworkError::ServerError(data) => write!(f, "Error: Server error - HTTP {}", data),
            NetworkError::Unauthorized(data) => write!(f, "Error: Unauthorized - {}", data),
            NetworkError::NotFound(data) => write!(f, "Error: Not found - {}", data),
            NetworkError::Utf8Error(error) => write!(f, "Error: UTF-8 error - {}", error),
            NetworkError::SerdeError(error) => write!(f, "Error: Serde error - {}", error),
            NetworkError::AlkaliError(error) => write!(f, "Error: Alkali error - {}", error),
            NetworkError::Base64DecodeError(error) => {
                write!(f, "Error: Base64 decode error - {}", error)
            }
            NetworkError::JoinError(error) => write!(f, "Error: Tokio join error - {}", error),
            NetworkError::CompositError(error, str) => {
                write!(f, "Error: Composite error - {}, \"{}\"", error, str)
            }
            NetworkError::Eyre(e) => {
                write!(f, "Error: {}", e)
            }
        }
    }
}

impl From<url::ParseError> for NetworkError {
    fn from(value: url::ParseError) -> Self {
        Self::UrlParseError(value)
    }
}
impl From<reqwest::Error> for NetworkError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(Arc::new(value))
    }
}

impl From<std::str::Utf8Error> for NetworkError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeError(Arc::new(value))
    }
}

impl From<base64::DecodeError> for NetworkError {
    fn from(value: base64::DecodeError) -> Self {
        Self::Base64DecodeError(value)
    }
}

impl From<AlkaliError> for NetworkError {
    fn from(value: AlkaliError) -> Self {
        Self::AlkaliError(value)
    }
}
impl From<JoinError> for NetworkError {
    fn from(value: JoinError) -> Self {
        Self::JoinError(Arc::new(value))
    }
}
impl From<Report> for NetworkError {
    fn from(value: Report) -> Self {
        Self::Eyre(Arc::new(value))
    }
}

/// Emty trait to specify which objects get parsed to ApiError::GenericError
trait StringError: Into<String> {}

impl StringError for String {}
impl StringError for &str {}

impl<T> From<T> for NetworkError
where
    T: StringError,
{
    fn from(value: T) -> Self {
        Self::GenericError(value.into())
    }
}

impl<E, S> From<(E, S)> for NetworkError
where
    E: Into<NetworkError>,
    S: Into<String>,
{
    fn from(value: (E, S)) -> Self {
        Self::CompositError(Arc::new(value.0.into()), value.1.into())
    }
}

impl Error for NetworkError {}
