use std::error::Error;

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

#[derive(Debug)]
pub enum ApiError {
    GenericError(String),
    UrlParseError(url::ParseError),

    Unauthorized(ResponseErrorData),
    NotFound(ResponseErrorData),
    ClientError(ResponseErrorData),
    ServerError(ResponseErrorData),

    ReqwestError(reqwest::Error),
    /// Used when ApiError::ReqwestError needs to be cloned.
    /// Since reqwest::Error does not implement Clone.
    ReqwestErrorClone(String),
}

impl Clone for ApiError {
    fn clone(&self) -> Self {
        match self {
            Self::GenericError(e) => Self::GenericError(e.clone()),
            Self::UrlParseError(e) => Self::UrlParseError(e.clone()),

            Self::ReqwestError(e) => Self::ReqwestErrorClone(e.to_string()),
            Self::ReqwestErrorClone(e) => Self::ReqwestErrorClone(e.clone()),

            Self::Unauthorized(e) => Self::Unauthorized(e.clone()),
            Self::NotFound(e) => Self::NotFound(e.clone()),
            Self::ServerError(e) => Self::ServerError(e.clone()),
            Self::ClientError(e) => Self::ClientError(e.clone()),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ApiError::GenericError(msg) => write!(f, "Error: {}", msg),
            ApiError::UrlParseError(e) => write!(f, "URL Parse Error: {}", e),

            ApiError::ReqwestError(e) => write!(f, "Request Error: {}", e),
            ApiError::ReqwestErrorClone(e) => write!(f, "Request Error: {}", e),

            ApiError::ClientError(data) => write!(f, "Client Error: HTTP {}", data),
            ApiError::ServerError(data) => write!(f, "Server Error: HTTP {}", data),
            ApiError::Unauthorized(data) => write!(f, "Unauthorized: {}", data),
            ApiError::NotFound(data) => write!(f, "Not Found: {}", data),
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
        Self::ReqwestError(value)
    }
}

/// Emtry trait to specify which objects get the default
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

impl Error for ApiError {}

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    None,
    Message,
    Error(ApiError),
}
