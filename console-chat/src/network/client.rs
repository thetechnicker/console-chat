use super::error::*;
use crate::action::Action;
use color_eyre::Result;
use reqwest;
use std::convert::TryInto;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::{Mutex, OnceLock};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument, trace};
use url::Url;

pub struct Client {
    url: Url,
    client: reqwest::Client,
    token: Option<String>,
    action_tx: UnboundedSender<Action>,
}

impl Deref for Client {
    type Target = reqwest::Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

pub struct ListenHandler {
    pub task: JoinHandle<()>,
    pub cancellation_token: CancellationToken,
}

static CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();

#[instrument]
pub async fn init<T>(url: T, action_tx: UnboundedSender<Action>) -> Result<()>
where
    T: TryInto<Url> + std::fmt::Debug,
    <T as TryInto<url::Url>>::Error: Sync + Send + std::error::Error + 'static,
{
    debug!("Initializing network client");
    let url = url.try_into()?;
    CLIENT.get_or_init(|| {
        Mutex::new(Client {
            url,
            client: reqwest::Client::new(),
            token: None,
            action_tx,
        })
    });
    auth().await?;
    debug!("Initializing done");
    Ok(())
}

async fn auth() -> Result<()> {
    trace!("Getting client lock");
    if let Some(client_lock) = CLIENT.get() {
        let client = client_lock.lock().unwrap();
        trace!("sending auth request");
        let result = handle_errors_json::<serde_json::Value>(
            client.post(client.url.join("/auth")?).send().await?,
        )
        .await?;
        debug!(
            "got auth result: {}",
            serde_json::to_string_pretty(&result)?
        );
    }
    Ok(())
}

pub fn handle_network(action: Action) -> Result<Option<Action>> {
    Ok(match action {
        Action::PerformJoin(_) => Some(Action::OpenChat),
        Action::PerformLogin(_, _) => Some(Action::OpenHome),
        _ => None,
    })
}
