use color_eyre::Result;
use futures_util::StreamExt;
use openapi_custom::apis::{configuration, users_api};
use openapi_custom::models::{MessageSend, MessageType, Plaintext};
use reqwest_eventsource::{Event, EventSource};
use std::io::{self, Write}; // Import for reading user input
use std::sync::{Arc, Mutex}; // For shared mutable state
use std::time::SystemTime;
use tokio::sync::Notify; // Import Notify for async notifications

#[tokio::main]
async fn main() -> Result<()> {
    let mut conf = configuration::Configuration::new();
    conf.client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let token = users_api::online_users_online_get(&conf, None).await?;
    println!("Token: {:#?}", token);
    conf.bearer_access_token = Some(token.token.token.clone());

    // Shared state to indicate if the connection is open and a notification system
    let is_open = Arc::new((Mutex::new(false), Notify::new()));

    let mut req = conf.client.get(format!("{}/r/abc", conf.base_path));
    if let Some(token) = conf.bearer_access_token.as_ref() {
        req = req.bearer_auth(token.to_owned());
    }
    let mut es = EventSource::new(req)?;
    // Using tokio::select! to wait for both listening and input
    loop {
        tokio::select! {
            res = listen(&mut es, Arc::clone(&is_open)) => {
                if let Err(e) = res {
                    println!("Error listening: {:#?}", e);
                    break;
                }
            }
            _ = read_input_and_send(&conf, Arc::clone(&is_open)) => {
                // Input handling is done in this branch
            }
        }
    }
    Ok(())
}

// Function to listen for events
async fn listen(es: &mut EventSource, is_open: Arc<(Mutex<bool>, Notify)>) -> Result<()> {
    if let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => {
                println!("Connection Open!");
                // Set the shared state to true when the connection opens
                let (lock, notify) = &*is_open;
                let mut open = lock.lock().unwrap();
                *open = true;
                notify.notify_waiters(); // Notify waiting tasks that the connection is open
            }
            Ok(Event::Message(message)) => {
                println!(
                    "Message: \x1b[38;5;241m{:#?}\x1b[0m",
                    //serde_json::from_str::<serde_json::Value>(&message.data)
                    message
                );
            }
            Err(err) => {
                println!("Error: {}", err);
                es.close();
                return Err(err.into());
            }
        }
    }
    Ok(())
}

// Function to read input from the user and send it as a POST request
async fn read_input_and_send(
    conf: &configuration::Configuration,
    is_open: Arc<(Mutex<bool>, Notify)>,
) -> Result<()> {
    let (lock, notify) = &*is_open;

    // Wait until the connection is open
    let mut open;
    {
        open = *lock.lock().unwrap();
    }
    while !open {
        notify.notified().await; // Wait for the notify signal
        open = *lock.lock().unwrap(); // Re-lock to check the condition
    }

    let mut input = String::new();
    print!("Enter message to send (type 'exit' to quit): ");
    io::stdout().flush().unwrap(); // Ensure the prompt is printed before reading input
    io::stdin().read_line(&mut input).unwrap();

    if input.trim().eq_ignore_ascii_case("exit") {
        println!("Exiting...");
        std::process::exit(0);
    }
    let msg = MessageSend {
        content: MessageType::Plaintext(Plaintext::new(input.trim().to_owned())),
        send_at: SystemTime::now().into(),
        data: None,
    };

    let body = serde_json::json!(msg);

    let res = conf
        .client
        .post(format!("{}/r/abc", conf.base_path))
        .bearer_auth(conf.bearer_access_token.as_ref().unwrap())
        .json(&body)
        .send()
        .await?;

    //println!("Response from server: {:#?}", res);
    Ok(())
}
