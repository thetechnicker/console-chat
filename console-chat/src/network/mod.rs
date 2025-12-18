use crate::cli::Cli;
use color_eyre::Result;
use lazy_static::lazy_static;
use openapi::apis::configuration::Configuration;
use openapi::apis::users_api;
use openapi::models::user_private::UserPrivate;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

lazy_static! {
    pub static ref CLIENT: Arc<Mutex<Configuration>> = Arc::new(Mutex::new(Configuration::new()));
    pub static ref USER: Arc<Mutex<Option<UserPrivate>>> = Arc::new(Mutex::new(None));
}

pub async fn init(config: Cli) -> Result<()> {
    let mut client = CLIENT.lock().await;
    if config.accept_invalid_certificate {
        client.client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;
    }
    let token = users_api::users_online(&client, None).await?;
    client.bearer_access_token = Some(token.token.token);
    let mut user = USER.lock().await;
    *user = Some(users_api::users_get_me(&client).await?);
    debug!("{:#?}", user);
    Ok(())
}
