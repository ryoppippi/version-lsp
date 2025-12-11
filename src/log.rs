use std::fs::OpenOptions;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::JsonFields;
use tracing_subscriber::prelude::*;

use crate::config;

pub fn init() -> anyhow::Result<()> {
    let data_dir = config::data_dir();

    std::fs::create_dir_all(&data_dir).inspect_err(|e| {
        eprintln!("Failed to create data directory: {}", e);
    })?;

    let log_path = config::log_path();
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .inspect_err(|e| {
            eprintln!("Failed to open log file {:?}: {}", log_path, e);
        })?;

    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(log_file)
        .fmt_fields(JsonFields::default());

    // Use RUST_LOG if set, otherwise default to INFO
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(json_layer)
        .init();

    Ok(())
}
