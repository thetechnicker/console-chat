use openapi::apis::{configuration, users_api};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;
    let token = users_api::users_online(&conf, Some("abc")).await?;
    println!("{:#?}", token);
    conf.bearer_access_token = Some(token.token.token);
    let me = users_api::users_get_me(&conf).await?;
    println!("{:#?}", me);
    Ok(())
}
