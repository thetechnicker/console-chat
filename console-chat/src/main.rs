extern crate openapi;

use crate::app::App;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod network;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    let res = actual_main().await;
    crate::logging::clear_logs();
    res
}

async fn actual_main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;
    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;

    Ok(())
}
