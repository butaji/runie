use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging to files in the runie directory
pub fn init_logging(runie_dir: &PathBuf) {
    let logs_dir = runie_dir.join("logs");
    std::fs::create_dir_all(&logs_dir).ok();

    // File appender for runie.log (rotating daily)
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "runie");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // JSON layer for structured logging
    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_ansi(false);

    // Filter: INFO by default, DEBUG for runie crates
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,runie=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(json_layer)
        .init();

    // Keep guard alive for the duration of the program
    std::mem::forget(_guard);
}
