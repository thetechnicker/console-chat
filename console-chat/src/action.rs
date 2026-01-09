pub(crate) use crate::error::{AppError, Result};
use crate::network::Message;
use openapi::models::UserPrivate;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum Action {
    // Unit variants (compact)
    Tick,
    Render,
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Help,
    Insert,
    Normal,
    OpenLogin,
    OpenSettings,
    OpenRawSettings,
    OpenChat,
    OpenHome,
    Hide,
    TriggerLogin,
    TriggerJoin,
    JoinRandom,
    Leave,
    SyncProfile,
    ReloadConfig,
    ResetConfig,
    StoreConfig,

    // Small fixed-size payloads
    Resize(u16, u16),
    OpenJoin(#[serde(skip)] bool),

    PerformLogin(String, String),
    PerformJoin(String, bool),
    SendMessage(String),
    Me(UserPrivate),
    ReceivedMessage(Message),

    // Error variant skipped in serde; boxed to avoid inflating enum
    #[serde(skip)]
    Error(AppError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_various_actions() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Stupid {
            action1: Action,
            action2: Action,
            action3: Action,
            action4: Action,
            action5: Action,
        }
        let x = Stupid {
            action1: Action::OpenJoin(true),
            action2: Action::Tick,
            action3: Action::OpenHome,
            action4: Action::Quit,
            action5: Action::OpenChat,
        };
        let a = serde_json::to_string(&x).unwrap();
        assert_eq!(
            a,
            "{\"action1\":\"OpenJoin\",\"action2\":\"Tick\",\"action3\":\"OpenHome\",\"action4\":\"Quit\",\"action5\":\"OpenChat\"}"
        );
        let b: Stupid = serde_json::from_str(&a).expect(&a);
        assert_eq!(
            b,
            Stupid {
                action1: Action::OpenJoin(false),
                action2: Action::Tick,
                action3: Action::OpenHome,
                action4: Action::Quit,
                action5: Action::OpenChat,
            }
        );
    }
}
