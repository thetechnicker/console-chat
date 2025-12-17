use openapi_custom::apis::{configuration, users_api};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;
    let token = users_api::online_users_online_get(&conf, Some("abc")).await?;
    println!("{:#?}", token);
    conf.bearer_access_token = Some(token.token.token);
    let me = users_api::get_me_users_me_get(&conf).await?;
    println!("{:#?}", me);
    Ok(())
}
