use lazy_static::lazy_static;
use openapi::apis::{configuration, rooms_api, users_api};
use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;

pub static CLIENT: OnceLock<Arc<configuration::Configuration>> = OnceLock::new();
