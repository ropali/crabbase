use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() -> WorkerGuard {
    // Rolling file appender (daily rotation)
    let file_appender = rolling::daily("logs", "app.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Layer 1: Pretty console output (human-readable)
    let console_filter = std::env::var("RUST_LOG")
        .ok()
        .map(EnvFilter::new)
        .unwrap_or_else(|| EnvFilter::new("info"));

    let console_layer = fmt::layer()
        .pretty()
        .with_target(false)
        .with_thread_ids(false)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_filter(console_filter);

    // Layer 2: JSON file output (machine-readable, for aggregators)
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_writer)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_filter(EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}
