//#![forbid(unsafe_code)]
use crate::app::App;
use tracing::info;

pub mod app;
pub mod custom_hashmap;
pub mod event;
pub mod log;
pub mod network;
pub mod screens;
pub mod utils;
pub mod widgets;

use ratatui::widgets::BorderType;
pub const DEFAULT_BORDER: BorderType = BorderType::Double;
//use ratatui::crossterm::{
//    event::{DisableMouseCapture, EnableMouseCapture},
//    execute,
//};
//use std::io::stdout;
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    #[allow(unused_variables)]
    let guard = log::init_tracing_file("terminal-chat.log");

    info!("App Starting");

    color_eyre::install()?;
    let terminal = ratatui::init();
    //execute!(stdout(), EnableMouseCapture)?;
    let result = App::default().run(terminal).await;
    ratatui::restore();
    //if let Err(err) = execute!(stdout(), DisableMouseCapture) {
    //    eprintln!("Error disabling mouse capture: {err}");
    //}
    match result {
        Ok(Some(d)) => {
            println!("{:?}", d);
            return Ok(());
        }
        Ok(None) => return Ok(()),
        Err(e) => return Err(e),
    }
}
