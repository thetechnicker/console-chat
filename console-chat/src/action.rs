pub(crate) use crate::error::{AppError, Result};
use crate::network::Message;
use console_chat_proc_macro::Subsetable;
use openapi::models::UserPrivate;
use serde::{Deserialize, Serialize};
use strum::Display;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Display, Subsetable, Serialize, Deserialize)]
#[subsetable(
    extra_fields={
        "VimEvent"=[
            "Enter(String)",
            "Up",
            "Down"
        ],
        "NetworkEvent"=[
            "RequestMe(Uuid)"
        ]
    },
    serialization={"NetworkEvent"=false}
)]
pub enum Action {
    // Unit variants (compact)
    Tick,
    Render,
    Suspend,
    Resume,
    #[subset("ButtonEvent")]
    Quit,
    ClearScreen,
    Help,
    #[subset("VimEvent")]
    Insert,
    #[subset("VimEvent")]
    Normal,
    #[subset("ButtonEvent")]
    OpenLogin,
    #[subset("ButtonEvent")]
    OpenSettings,
    #[subset("ButtonEvent")]
    OpenRawSettings,
    #[subset("ButtonEvent")]
    OpenChat,
    #[subset("ButtonEvent")]
    OpenHome,
    Hide,
    #[subset("ButtonEvent")]
    TriggerLogin,
    #[subset("ButtonEvent")]
    TriggerJoin,
    #[subset("ButtonEvent")]
    JoinRandom,
    Leave,
    SyncProfile,
    ReloadConfig,
    #[subset("ButtonEvent")]
    ResetConfig,
    #[subset("VimEvent")]
    StoreConfig,

    // Small fixed-size payloads
    Resize(u16, u16),
    #[subset("ButtonEvent")]
    OpenJoin(#[serde(skip)] bool),

    #[subset("NetworkEvent")]
    PerformLogin(String, String),
    #[subset("NetworkEvent")]
    PerformJoin(String, bool),
    #[subset("NetworkEvent")]
    SendMessage(String),
    #[subset("NetworkEvent")]
    Me(UserPrivate),
    #[subset("NetworkEvent")]
    ReceivedMessage(Message),

    // Error variant skipped in serde
    #[subset("NetworkEvent")]
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
