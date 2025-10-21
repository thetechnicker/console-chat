use fern::Dispatch;
use std::fs::OpenOptions;

pub fn init_logging() {
    init_logging_file("log.log")
}

pub fn init_logging_file(file_name: &str) {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_name)
        .unwrap();

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(log_file) // Write logs to file only
        .apply()
        .expect("Failed to initialize logging");
}
