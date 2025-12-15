use openapi::apis::configuration;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;

pub static CLIENT: OnceLock<Arc<Mutex<configuration::Configuration>>> = OnceLock::new();
