use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{
    EnvFilter, Layer, fmt, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use super::pg_layer::PgLayer;
use super::writer::{PgLogWriter, WriterConfig};

/// Guards that must live as long as the process. Dropping the file guard
/// flushes the file appender; shutting the writer down flushes the audit
/// tail to Postgres.
pub struct LoggingGuards {
    pub file: Option<WorkerGuard>,
    pub writer: Option<PgLogWriter>,
}

/// Everything the database logger needs from the application config.
/// `core` cannot depend on the binary's `Config` type, so this is the
/// boundary between the two.
pub struct PgLogSetup {
    pub database_url: String,
    /// Schema the `_logs` table (and `_sqlx_migrations`) lives in. Set as the
    /// pool's `search_path` so the writer's unqualified INSERT lands there.
    pub schema: String,
    pub writer: WriterConfig,
    /// Minimum severity persisted to `_logs`, e.g. "warn". The console and
    /// file layers keep their own filters (`RUST_LOG`).
    pub min_level: String,
}

pub fn init_logging(pg: Option<PgLogSetup>) -> LoggingGuards {
    let (pg_handle, pg_writer, pg_min_level) = match pg {
        Some(setup) => {
            let min_level = setup.min_level.clone();
            let (handle, writer) =
                PgLogWriter::spawn(&setup.database_url, &setup.schema, setup.writer);
            (Some(handle), Some(writer), min_level)
        }
        None => (None, None, String::new()),
    };

    let (file_writer, file_guard) =
        tracing_appender::non_blocking(rolling::daily("logs", "app.log"));

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
    let file_filter = std::env::var("RUST_LOG_FILE")
        .ok()
        .map(EnvFilter::new)
        .unwrap_or_else(|| EnvFilter::new("info"));

    let file_layer = fmt::layer()
        .json()
        .with_writer(file_writer)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_filter(file_filter);

    // Layer 3: audit events into Postgres via the background writer.
    let pg_layer = pg_handle.map(|handle| {
        let filter: Level = pg_min_level.parse().unwrap_or(Level::WARN);
        PgLayer::new(handle.tx.clone(), filter, handle.dropped.clone())
    });

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(pg_layer);

    // try_init, not init: integration tests call this repeatedly.
    if let Err(e) = subscriber.try_init() {
        eprintln!("logging already initialised: {e}");
    }

    LoggingGuards {
        file: Some(file_guard),
        writer: pg_writer,
    }
}
