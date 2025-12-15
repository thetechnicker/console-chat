use openapi::apis::configuration;
use std::sync::Arc;
use std::sync::OnceLock;

pub static CLIENT: OnceLock<Arc<configuration::Configuration>> = OnceLock::new();
