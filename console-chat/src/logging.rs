use chrono::*;
use color_eyre::Result;
use std::sync::OnceLock;
use std::time::SystemTime;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config;

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOG_LEVEL", config::PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
    pub static ref ANSI_LOG_FILE: String = format!("{}.ansi.log", env!("CARGO_PKG_NAME"));
    pub static ref START: DateTime<Utc> =  DateTime::<Utc>::from(SystemTime::now());
}

pub static LOG_FILE_ID: OnceLock<usize> = OnceLock::new();
pub static LOG_FILE_SET: OnceLock<()> = OnceLock::new();

//pub fn init() -> Result<Option<usize>> {
pub fn init() -> Result<()> {
    let directory = config::get_data_dir();

    std::fs::create_dir_all(directory.clone())?;
    //let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    let mut log_path = directory.join(LOG_FILE.clone());
    let mut ansi_log_path = directory.join(ANSI_LOG_FILE.clone());
    let mut i = 0;
    while log_path.exists() {
        i += 1;
        log_path = directory.join(format!("{i}_{}", LOG_FILE.clone()));
        ansi_log_path = directory.join(format!("{i}_{}", ANSI_LOG_FILE.clone()));
    }
    let log_file = std::fs::File::create(log_path)?;
    let ansi_log_file = std::fs::File::create(ansi_log_path)?;
    if i > 0 {
        let _ = LOG_FILE_ID.set(i);
    }
    let _ = LOG_FILE_SET.set(());

    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());
    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;

    let file_subscriber = fmt::layer()
        .json()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter.clone());

    let file_subscriber_ansi = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(ansi_log_file)
        .with_target(false)
        .with_ansi(true)
        .with_filter(env_filter);

    //let debug_str = format!("{:#?}", file_subscriber);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(file_subscriber_ansi)
        .with(ErrorLayer::default())
        .try_init()?;
    //tracing::info!("{}", debug_str);
    //Ok(if i > 0 { Some(i) } else { None })
    Ok(())
}

pub fn clear_logs() {
    if LOG_FILE_SET.get().is_none() {
        return;
    }
    let i = LOG_FILE_ID.get();
    let directory = config::get_data_dir();
    let (log_path, ansi_log_path) = match i {
        Some(i) => (
            directory.join(format!("{i}_{}", LOG_FILE.clone())),
            directory.join(format!("{i}_{}", ANSI_LOG_FILE.clone())),
        ),
        None => (
            directory.join(LOG_FILE.clone()),
            directory.join(ANSI_LOG_FILE.clone()),
        ),
    };

    let timestamp = START.format("%Y-%m-%dT%H-%M-%S").to_string();

    let new_log_path = directory.join(format!(
        "{}_{}",
        LOG_FILE.trim_end_matches(".log"),
        timestamp
    ));
    let new_ansi_path = directory.join(format!(
        "{}_{}",
        ANSI_LOG_FILE.trim_end_matches(".log"),
        timestamp
    ));

    if log_path.exists() {
        std::fs::rename(&log_path, &new_log_path).expect("Failed to rename log");
    }
    if ansi_log_path.exists() {
        std::fs::rename(&ansi_log_path, &new_ansi_path).expect("Failed to rename ANSI log");
    }
}
