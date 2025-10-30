use alkali::AlkaliError;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone, Debug)]
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
pub enum ApiError {
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
    CompositError(Arc<ApiError>, String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ApiError::GenericError(msg) => write!(f, "Error: {}", msg),
            ApiError::UrlParseError(e) => write!(f, "URL Parse Error: {}", e),
            ApiError::ReqwestError(e) => write!(f, "Request Error: {}", e),
            ApiError::ClientError(data) => write!(f, "Client Error: HTTP {}", data),
            ApiError::ServerError(data) => write!(f, "Server Error: HTTP {}", data),
            ApiError::Unauthorized(data) => write!(f, "Unauthorized: {}", data),
            ApiError::NotFound(data) => write!(f, "Not Found: {}", data),
            ApiError::Utf8Error(error) => write!(f, "Utf8Error: {}", error),
            ApiError::SerdeError(error) => write!(f, "SerdeError: {}", error),
            ApiError::AlkaliError(error) => write!(f, "AlkaliError: {}", error),
            ApiError::Base64DecodeError(error) => write!(f, "Base64Error: {}", error),
            ApiError::CompositError(error, str) => {
                write!(f, "CompositError: {}, \"{}\"", error, str)
            }
        }
    }
}

impl From<url::ParseError> for ApiError {
    fn from(value: url::ParseError) -> Self {
        Self::UrlParseError(value)
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(Arc::new(value))
    }
}

impl From<std::str::Utf8Error> for ApiError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeError(Arc::new(value))
    }
}

impl From<base64::DecodeError> for ApiError {
    fn from(value: base64::DecodeError) -> Self {
        Self::Base64DecodeError(value)
    }
}

impl From<AlkaliError> for ApiError {
    fn from(value: AlkaliError) -> Self {
        Self::AlkaliError(value)
    }
}

/// Emty trait to specify which objects get parsed to ApiError::GenericError
trait StringError: Into<String> {}

impl StringError for String {}
impl StringError for &str {}

impl<T> From<T> for ApiError
where
    T: StringError,
{
    fn from(value: T) -> Self {
        Self::GenericError(value.into())
    }
}

impl<E, S> From<(E, S)> for ApiError
where
    E: Into<ApiError>,
    S: Into<String>,
{
    fn from(value: (E, S)) -> Self {
        Self::CompositError(Arc::new(value.0.into()), value.1.into())
    }
}

impl Error for ApiError {}
