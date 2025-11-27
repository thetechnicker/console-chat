use alkali::AlkaliError;
use reqwest::StatusCode;
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
pub enum NetworkError {
    CriticalFailure,
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
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NetworkError::GenericError(msg) => write!(f, "Error: {}", msg),
            NetworkError::UrlParseError(e) => write!(f, "URL Parse Error: {}", e),
            NetworkError::ReqwestError(e) => write!(f, "Request Error: {}", e),
            NetworkError::ClientError(data) => write!(f, "Client Error: HTTP {}", data),
            NetworkError::ServerError(data) => write!(f, "Server Error: HTTP {}", data),
            NetworkError::Unauthorized(data) => write!(f, "Unauthorized: {}", data),
            NetworkError::NotFound(data) => write!(f, "Not Found: {}", data),
            NetworkError::Utf8Error(error) => write!(f, "Utf8Error: {}", error),
            NetworkError::SerdeError(error) => write!(f, "SerdeError: {}", error),
            NetworkError::AlkaliError(error) => write!(f, "AlkaliError: {}", error),
            NetworkError::Base64DecodeError(error) => write!(f, "Base64Error: {}", error),
            NetworkError::CompositError(error, str) => {
                write!(f, "CompositError: {}, \"{}\"", error, str)
            }
            NetworkError::CriticalFailure => {
                write!(f, "A Unexpected Error Happend")
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

#[inline]
pub async fn handle_errors_raw(resp: reqwest::Response) -> Result<reqwest::Response, NetworkError> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let url = resp.url().to_owned();
    let msg = resp
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read error message.".to_string());

    let error_data = ResponseErrorData { msg, status, url };

    match status {
        StatusCode::NOT_FOUND => Err(NetworkError::NotFound(error_data)),
        StatusCode::UNAUTHORIZED => Err(NetworkError::Unauthorized(error_data)),
        status if status.is_client_error() => Err(NetworkError::ClientError(error_data)),
        status if status.is_server_error() => Err(NetworkError::ServerError(error_data)),
        _ => Err(format!("Unexpected status: {}", status).into()),
    }
}

#[allow(unused_lifetimes)]
#[inline]
pub async fn handle_errors_json<'a, T>(resp: reqwest::Response) -> Result<T, NetworkError>
where
    T: serde::de::DeserializeOwned,
{
    if resp.status().is_success() {
        let data = resp.json::<T>().await?;
        return Ok(data);
    }
    let status = resp.status();
    let url = resp.url().to_owned();
    let msg = resp
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read error message.".to_string());

    let error_data = ResponseErrorData { msg, status, url };

    match status {
        StatusCode::NOT_FOUND => Err(NetworkError::NotFound(error_data)),
        StatusCode::UNAUTHORIZED => Err(NetworkError::Unauthorized(error_data)),
        status if status.is_client_error() => Err(NetworkError::ClientError(error_data)),
        status if status.is_server_error() => Err(NetworkError::ServerError(error_data)),
        _ => Err(format!("Unexpected status: {}", status).into()),
    }
}
