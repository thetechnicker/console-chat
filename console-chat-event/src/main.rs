use crate::app::App;

pub mod app;
pub mod event;
pub mod log;
pub mod network;
pub mod screens;
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
    log::init_logging_file("terminal-chat.log");

    color_eyre::install()?;
    let terminal = ratatui::init();
    //execute!(stdout(), EnableMouseCapture)?;
    let result = App::new().run(terminal).await;
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
