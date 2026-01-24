use crate::network::models::Message;
use console_chat_proc_macro::Subsetable;
use openapi::models::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::Display;

pub(crate) use crate::error::{AppError, Result};

#[derive(Debug, Clone, PartialEq, Display, Subsetable, Serialize, Deserialize)]
#[subsetable(
    extra_fields={
        "VimEvent"=[
            "Enter(String)",
            "Up",
            "Down",
            "Nop"
        ],
        "NetworkEvent"=[
            "RequestMe"
        ],
        "DialogEvent"=[
            "Ok(Vec<String>)",
            "Cancel"
        ]
    },
    serialization={
        "NetworkEvent" = false,
        "DialogEvent" = false
    }
)]
pub enum Action {
    // Unit variants (compact)
    Tick,
    Render,
    Suspend,
    Resume,
    #[subset("ButtonEvent")]
    Quit,
    #[subset("ButtonEvent")]
    Ok,
    #[subset("ButtonEvent")]
    Cancel,
    ClearScreen,
    Help,
    #[subset("VimEvent", "DialogEvent")]
    Insert,
    #[subset("VimEvent", "DialogEvent")]
    Normal,
    #[subset("ButtonEvent")]
    OpenLogin,
    #[subset("ButtonEvent")]
    OpenSettings,
    #[subset("ButtonEvent")]
    OpenChat,
    #[subset("ButtonEvent")]
    OpenHome,
    Hide,
    #[subset("ButtonEvent")]
    TriggerLogin,
    #[subset("ButtonEvent")]
    TriggerJoin,
    #[subset("ButtonEvent", "NetworkEvent")]
    JoinRandom,
    Leave,
    SyncProfile,
    ReloadConfig,
    #[subset("ButtonEvent")]
    ResetConfig,
    #[subset("ButtonEvent")]
    OpenStaticRoomManagement,
    #[subset("NetworkEvent")]
    RequestMyRooms,

    // Small fixed-size payloads
    #[subset("ButtonEvent")]
    OpenJoin(#[serde(skip)] bool),
    Resize(u16, u16),

    #[subset("VimEvent")]
    StoreConfig(#[serde(skip)] String),

    #[subset("NetworkEvent")]
    SendMessage(String),
    #[subset("NetworkEvent")]
    PerformJoin(String, bool),
    #[subset("NetworkEvent")]
    PerformLogin(String, String),
    #[subset("NetworkEvent")]
    Me(UserPrivate),
    #[subset("NetworkEvent")]
    ReceivedMessage(Message),

    #[subset("NetworkEvent")]
    #[serde(skip)]
    MyRooms(Arc<[StaticRoomPublic]>),

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
        let Ok(a) = serde_json::to_string(&x) else {
            panic!("failed to serialize Actions");
        };
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
