pub mod data_model;
pub mod error;
pub type Result<T, E = error::NetworkError> = std::result::Result<T, E>;
