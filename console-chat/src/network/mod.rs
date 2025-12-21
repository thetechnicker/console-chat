use crate::action::Action;
use crate::cli::Cli;
use lazy_static::lazy_static;
use openapi::apis::Error as ApiError;
use openapi::apis::configuration::Configuration;
use openapi::apis::users_api;
use openapi::models::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub(crate) mod error;
pub(self) type Result<T, E = error::NetworkError> = std::result::Result<T, E>;

lazy_static! {
    pub static ref CONFIGURATION: Arc<RwLock<Configuration>> =
        Arc::new(RwLock::new(Configuration::new()));
    pub static ref USER: Arc<RwLock<Option<UserPrivate>>> = Arc::new(RwLock::new(None));
    pub static ref ROOM: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
}

pub async fn init(config: Cli) -> Result<()> {
    let mut client = CONFIGURATION.write().await;
    if config.accept_invalid_certificate {
        client.client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;
    }
    let response = users_api::users_online(&client, None).await?;
    client.bearer_access_token = Some(response.token.token);
    let mut user = USER.write().await;
    *user = Some(users_api::users_get_me(&client).await?);
    debug!("{:#?}", user);
    Ok(())
}

pub async fn handle_actions(event: Action) -> Result<Option<Action>> {
    match event {
        Action::OpenLogin => {
            let me = USER.read().await;
            if let Some(me) = me.as_ref() {
                return Ok(Some(Action::Me(me.clone())));
            }
        }
        Action::PerformLogin(username, password) => {
            login(&username, &password).await?;
            return Ok(Some(Action::OpenHome));
        }
        Action::PerformJoin(room) => {
            join(&room).await?;
        }
        Action::SendMessage(msg) => {
            send_message(&msg).await?;
        }
        _ => {}
    }
    Ok(None)
}

async fn join(_room: &str) -> Result<()> {
    Ok(())
}
async fn send_message(_message: &str) -> Result<()> {
    Ok(())
}

async fn login(username: &str, password: &str) -> Result<()> {
    let mut conf = CONFIGURATION.write().await;
    let login = LoginData {
        username: username.to_owned(),
        password: password.to_owned(),
    };
    match users_api::users_login(&conf, login.clone()).await {
        Ok(response) => {
            conf.bearer_access_token = Some(response.token.token);

            let mut user = USER.write().await;
            *user = Some(users_api::users_get_me(&conf).await?);
            debug!("{:#?}", user);
            Ok(())
        }
        Err(e) => {
            // TODO: is it a good idea to register if login fails?
            if let ApiError::ResponseError(ref e) = e {
                if let Some(users_api::UsersLoginError::Status401(_)) = e.entity {
                    if let Ok(string) = serde_json::to_string(&login) {
                        debug!("{}", string);
                    }
                    let response = users_api::users_register(&conf, login).await?;
                    conf.bearer_access_token = Some(response.token.token);

                    let mut user = USER.write().await;
                    *user = Some(users_api::users_get_me(&conf).await?);
                    debug!("{:#?}", user);
                }
            }
            Err(e.into())
        }
    }
}
