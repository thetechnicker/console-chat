use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    ErrorDone,
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
    PerformLogin(String, Option<String>),
    TriggerJoin,
    PerformJoin(String),
    SyncProfile,
    Leave,
    ReloadConfig,
}
