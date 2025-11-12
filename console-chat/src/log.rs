use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt;

pub fn init_tracing_file(file_prefix: &str) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, "./logs", file_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    fmt()
        .with_writer(non_blocking)
        .with_max_level(tracing_subscriber::filter::LevelFilter::from_level(
            tracing::Level::DEBUG,
        ))
        .with_ansi(false)
        //.with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        //.with_timer(fmt::time::LocalTime::rfc_3339())
        .event_format(
            fmt::format()
                //.compact()
                //.json()
                .with_level(true)
                //.with_target(true)
                //.with_line_number(true)
                //.with_file(true)
                //.with_thread_names(true)
                //.with_source_location(true)
                .with_timer(fmt::time::LocalTime::rfc_3339()),
        )
        .init();

    tracing::info!("logging setup");
    guard
}

pub fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    init_tracing_file("log.log")
}
