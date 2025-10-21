use std::error::Error;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(Option<String>),
    NotFound(Option<String>),
    GenericError(String),
    UrlParseError(url::ParseError),
    ReqwestError(reqwest::Error),
    ClientError(reqwest::StatusCode),
    ServerError(reqwest::StatusCode),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ApiError::Unauthorized(Some(msg)) => write!(f, "Unauthorized: {}", msg),
            ApiError::Unauthorized(None) => write!(f, "Unauthorized access"),
            ApiError::NotFound(Some(msg)) => write!(f, "Not Found: {}", msg),
            ApiError::NotFound(None) => write!(f, "Resource not found"),
            ApiError::GenericError(msg) => write!(f, "Error: {}", msg),
            ApiError::UrlParseError(e) => write!(f, "URL Parse Error: {}", e),
            ApiError::ReqwestError(e) => write!(f, "Request Error: {}", e),
            ApiError::ClientError(code) => write!(f, "Client Error: HTTP {}", code),
            ApiError::ServerError(code) => write!(f, "Server Error: HTTP {}", code),
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

impl From<String> for ApiError {
    fn from(value: String) -> Self {
        Self::GenericError(value)
    }
}

impl Error for ApiError {}

pub mod client;
pub mod user;
