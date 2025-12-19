use crate::util::TypeErasedWrapper;
use openapi::apis::{Error as OpenapiError, ResponseContent};
use reqwest_eventsource::{CannotCloneRequestError, Error as EventError};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum NetworkError {
    Reqwest(Arc<reqwest::Error>),
    ReqwestEventSource(Arc<EventError>),
    Serde(Arc<serde_json::Error>),
    Io(Arc<std::io::Error>),
    ResponseError(Arc<ResponseContent<TypeErasedWrapper>>),
    EventSourceError(CannotCloneRequestError),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (module, e) = match self {
            Self::Reqwest(e) => ("reqwest", e.to_string()),
            Self::ReqwestEventSource(e) => ("reqwest-eventsource", e.to_string()),
            Self::Serde(e) => ("serde", e.to_string()),
            Self::Io(e) => ("IO", e.to_string()),
            Self::EventSourceError(e) => ("event source", e.to_string()),
            Self::ResponseError(e) => ("response", format!("status code {}", e.status)),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl PartialEq for NetworkError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}
impl Eq for NetworkError {}
impl std::error::Error for NetworkError {}

impl<T> From<OpenapiError<T>> for NetworkError
where
    T: 'static,
{
    fn from(value: OpenapiError<T>) -> NetworkError {
        match value {
            OpenapiError::Reqwest(e) => NetworkError::Reqwest(Arc::new(e)),
            OpenapiError::ReqwestEventSource(e) => NetworkError::ReqwestEventSource(Arc::new(e)),
            OpenapiError::Serde(e) => NetworkError::Serde(Arc::new(e)),
            OpenapiError::Io(e) => NetworkError::Io(Arc::new(e)),
            OpenapiError::EventSourceError(e) => NetworkError::EventSourceError(e),
            OpenapiError::ResponseError(e) => {
                NetworkError::ResponseError(Arc::new(ResponseContent {
                    status: e.status,
                    content: e.content,
                    entity: Some(TypeErasedWrapper::new(e.entity)),
                }))
            }
        }
    }
}

pub trait ToNetworkError: Into<OpenapiError<()>> {}

impl ToNetworkError for reqwest::Error {}
impl ToNetworkError for reqwest_eventsource::Error {}
impl ToNetworkError for serde_json::Error {}
impl ToNetworkError for std::io::Error {}

impl<T> From<T> for NetworkError
where
    T: ToNetworkError,
{
    fn from(value: T) -> NetworkError {
        let x: OpenapiError<()> = value.into();
        x.into()
    }
}
