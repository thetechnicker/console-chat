use crate::util::TypeErasedWrapper;
use alkali::AlkaliError;
use base64::DecodeError;
use openapi::apis::{Error as OpenapiError, ResponseContent};
use reqwest_eventsource::{CannotCloneRequestError, Error as EventError};
use std::error::Error;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum NetworkError {
    Eyre(Arc<color_eyre::Report>),
    Reqwest(Arc<reqwest::Error>),
    ReqwestEventSource(Arc<EventError>),
    Serde(Arc<serde_json::Error>),
    Io(Arc<std::io::Error>),
    ResponseError(Arc<ResponseContent<TypeErasedWrapper>>),
    CannotCloneRequestError(CannotCloneRequestError),
    AlkaliError(AlkaliError),
    Base64Error(DecodeError),
    Utf8Error(std::str::Utf8Error),
}

pub fn print_recursive_error(e: impl Error) -> String {
    fn print_recursive_error_inner(e: impl Error, depth: usize) -> String {
        if let Some(source) = e.source() {
            format!(
                "{}{}\nsource: {}",
                "\t".repeat(depth),
                e,
                print_recursive_error_inner(source, depth + 1)
                    .replace("\n", &format!("\n{}", "\t".repeat(depth + 1)))
            )
        } else {
            e.to_string()
        }
    }
    print_recursive_error_inner(e, 0)
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (module, e) = match self {
            Self::Eyre(e) => ("network", print_recursive_error(e.root_cause())),
            Self::Reqwest(e) => ("reqwest", print_recursive_error(e)),
            Self::Utf8Error(e) => ("utf8", print_recursive_error(e)),
            Self::AlkaliError(e) => ("alkali", print_recursive_error(e)),
            Self::Base64Error(e) => ("base64", print_recursive_error(e)),
            Self::ReqwestEventSource(e) => ("reqwest-eventsource", print_recursive_error(e)),
            Self::Serde(e) => ("serde", print_recursive_error(e)),
            Self::Io(e) => ("IO", print_recursive_error(e)),
            Self::CannotCloneRequestError(e) => ("event source", print_recursive_error(e)),
            Self::ResponseError(e) => (
                "response",
                if let Some(entity) = e.entity.as_ref() {
                    format!(
                        "status code {}, contet: {}, data: {:?}",
                        e.status, e.content, entity
                    )
                } else {
                    format!("status code {}, contet: {}", e.status, e.content)
                },
            ),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl std::error::Error for NetworkError {}

impl<T> From<OpenapiError<T>> for NetworkError
where
    T: 'static + Clone + Sync + Send + std::fmt::Debug,
{
    fn from(value: OpenapiError<T>) -> NetworkError {
        match value {
            OpenapiError::Reqwest(e) => NetworkError::Reqwest(Arc::new(e)),
            OpenapiError::ReqwestEventSource(e) => NetworkError::ReqwestEventSource(Arc::new(e)),
            OpenapiError::Serde(e) => NetworkError::Serde(Arc::new(e)),
            OpenapiError::Io(e) => NetworkError::Io(Arc::new(e)),
            OpenapiError::EventSourceError(e) => NetworkError::CannotCloneRequestError(e),
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

impl From<DecodeError> for NetworkError {
    fn from(value: DecodeError) -> NetworkError {
        Self::Base64Error(value)
    }
}

impl From<AlkaliError> for NetworkError {
    fn from(value: AlkaliError) -> NetworkError {
        Self::AlkaliError(value)
    }
}

impl From<std::str::Utf8Error> for NetworkError {
    fn from(value: std::str::Utf8Error) -> NetworkError {
        Self::Utf8Error(value)
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

impl From<color_eyre::Report> for NetworkError {
    fn from(value: color_eyre::Report) -> NetworkError {
        Self::Eyre(Arc::new(value))
    }
}
