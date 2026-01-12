#![deny(clippy::unwrap_used)]
//#![deny(clippy::expect_used)]
use crate::app::App;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use tracing::error;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod error;
mod errors;
mod logging;
mod network;
mod tui;
mod util;
//pub(crate) use error::LockErrorExt;

#[tokio::main]
async fn main() -> Result<()> {
    let res = actual_main().await;
    if res.is_err() {
        error!("App exited with: {:#?}", res.as_ref().err())
    }
    crate::logging::clear_logs();
    res
}

async fn actual_main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;
    let args = Cli::parse();
    let mut app = App::new(args)?;
    app.run().await?;
    Ok(())
}
