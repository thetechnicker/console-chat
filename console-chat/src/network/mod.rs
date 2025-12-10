use lazy_static::lazy_static;
use openapi::apis::{configuration, rooms_api, users_api};
use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;

pub static CLIENT: OnceLock<Arc<configuration::Configuration>> = OnceLock::new();

lazy_static! {
    static ref Client: Mutex<Option<configuration::ApiKey>> = Mutex::new(None);
}

#[derive(Serialize)]
pub struct A(String);
#[derive(Serialize)]
pub struct B(String, u32);
#[derive(Serialize)]
pub struct C(String, f32);

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum MessageType {
    A(A),
    B(B),
    C(C),
}

#[derive(Serialize)]
pub struct Message {
    content: MessageType,
    other_data: String,
    other_data_idk: Vec<String>,
}
