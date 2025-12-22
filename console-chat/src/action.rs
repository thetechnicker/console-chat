pub(crate) use crate::error::{AppError, Result};
use openapi::models::{MessagePublic, UserPrivate};
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    #[serde(skip)]
    Error(AppError),
    Help,

    Insert,
    Normal,

    //  Open
    OpenLogin,
    OpenSettings,
    OpenRawSettings,
    OpenChat,
    OpenJoin,
    OpenHome,
    Hide,

    TriggerLogin,
    PerformLogin(String, String),
    TriggerJoin,
    PerformJoin(String),
    SendMessage(String),
    Me(UserPrivate),
    ReceivedMessage(MessagePublic),
    Leave,

    SyncProfile,
    ReloadConfig,
}
