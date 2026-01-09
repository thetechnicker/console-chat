use crate::action::Action;
use crate::cli::Cli;
//use crate::error::print_recursive_error;
use crate::config::Config;
use alkali::asymmetric::cipher::{self, Keypair, PUBLIC_KEY_LENGTH, PublicKey};
use alkali::mem::FullAccess;
use alkali::symmetric::cipher::{self as symetric_cipher, Key, NONCE_LENGTH};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use color_eyre::eyre::OptionExt;
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use openapi::apis::Error as ApiError;
use openapi::apis::configuration::Configuration;
use openapi::apis::{rooms_api, users_api};
use openapi::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tracing::{debug, error};

pub(crate) struct NetworkData {
    config: Arc<std::sync::RwLock<Config>>,
    conf: Arc<RwLock<Configuration>>,
    keys: Arc<RwLock<HashMap<String, Key<FullAccess>>>>,
}
