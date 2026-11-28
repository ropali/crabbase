use sqlx::{
    Pool, Postgres,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::Instant,
};
use uuid::Uuid;

use super::event::AuditEvent;

#[derive(Debug, Clone)]
pub struct WriterConfig {
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub queue_capacity: usize,
    pub pool_max_connections: u32,
    // Hard cap on how long one batch insert may take before it is abandoned.
    pub insert_timeout: Duration,
    // Skip the fsync-before-commit barrier on the log pool.
    // Note the field is `sync_commit` but the value is "inverted"
    // relative to the SQL: `false` here means "do not force durability",
    // which is what the measurement favours.
    pub sync_commit: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100, // measured: past ~50 the win flattens
            flush_interval: Duration::from_millis(250),
            queue_capacity: 8_192,
            pool_max_connections: 2,
            insert_timeout: Duration::from_secs(2),
            sync_commit: false,
        }
    }
}

/// Handle held by `AppState` so the layer can be constructed from it.
#[derive(Clone)]
pub struct PgLogHandle {
    pub tx: mpsc::Sender<AuditEvent>,
    /// Shared with the layer so a full queue is counted in `stats()`.
    pub dropped: Arc<AtomicU64>,
}

/// Owned by `main` and held for the process lifetime — shutting this down
/// flushes and stops the task. See `shutdown`.
pub struct PgLogWriter {
    handle: JoinHandle<()>,
    shutdown: watch::Sender<bool>,
    dropped: Arc<AtomicU64>,
    written: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
}

impl PgLogWriter {
    // MUST own a separate pool.
    // `pool.clone()` shares the same pool — do not do that. Logging that
    // competes with user queries for connections can starve them, and the
    // reverse is worse.
    pub fn spawn(db_url: &str, schema: &str, cfg: WriterConfig) -> (PgLogHandle, Self) {
        let (tx, rx) = mpsc::channel(cfg.queue_capacity);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let dropped = Arc::new(AtomicU64::new(0));
        let written = Arc::new(AtomicU64::new(0));
        let errors = Arc::new(AtomicU64::new(0));

        // `search_path` is a connection option, so the pooled connection the
        // writer borrows resolves `_logs` in the configured schema without an
        // extra round trip. The logs own schema is also where their
        // `_sqlx_migrations` lives, keeping the history separate from the app.
        let opts = PgConnectOptions::from_str(db_url)
            .expect("log pool is lazy; only fails on malformed URL")
            .application_name("crabbase-logger")
            .options([("search_path", schema)]);

        let pool = PgPoolOptions::new()
            .max_connections(cfg.pool_max_connections)
            .acquire_timeout(Duration::from_millis(500))
            .after_connect({
                let sync_commit = cfg.sync_commit;
                move |conn, _meta| {
                    Box::pin(async move {
                        if !sync_commit {
                            // Applies to this connection only. The row is
                            // still WAL-written; we just stop waiting for
                            // the fsync before reporting success.
                            sqlx::query("SET synchronous_commit = off")
                                .execute(conn)
                                .await?;
                        }
                        Ok(())
                    })
                }
            })
            .connect_lazy_with(opts);

        let (d, w, e) = (dropped.clone(), written.clone(), errors.clone());

        let handle = tokio::spawn(writer_loop(pool, cfg, rx, shutdown_rx, d, w, e));

        (
            PgLogHandle {
                tx: tx.clone(),
                dropped: dropped.clone(),
            },
            Self {
                handle,
                shutdown: shutdown_tx,
                dropped,
                written,
                errors,
            },
        )
    }

    /// Flush and stop. Call before the process exits or you lose the tail.
    ///
    /// The watch channel is the shutdown signal rather than closing the
    /// mpsc channel, because `AppState` may still hold a `PgLogHandle`
    /// whose sender would keep the receiver alive.
    pub async fn shutdown(self) -> LogStats {
        let Self {
            handle,
            shutdown,
            dropped,
            written,
            errors,
        } = self;
        let _ = shutdown.send(true);
        handle.await.ok();
        LogStats {
            dropped: dropped.load(Ordering::Relaxed),
            written: written.load(Ordering::Relaxed),
            errors: errors.load(Ordering::Relaxed),
        }
    }

    pub fn stats(&self) -> LogStats {
        LogStats {
            dropped: self.dropped.load(Ordering::Relaxed),
            written: self.written.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogStats {
    pub dropped: u64,
    pub written: u64,
    pub errors: u64,
}

async fn writer_loop(
    pool: Pool<Postgres>,
    cfg: WriterConfig,
    mut rx: mpsc::Receiver<AuditEvent>,
    mut shutdown: watch::Receiver<bool>,
    dropped: Arc<AtomicU64>,
    written: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
) {
    let mut batch: Vec<AuditEvent> = Vec::with_capacity(cfg.batch_size);
    let mut deadline = Instant::now() + cfg.flush_interval;

    loop {
        tokio::select! {
            // Shutdown signalled. Drain the channel tail, flush, then exit.
            _ = shutdown.changed() => {
                while let Ok(ev) = rx.try_recv() {
                    batch.push(ev);
                }
                if !batch.is_empty() {
                    flush(&pool, &cfg, &mut batch, &dropped, &written, &errors).await;
                }
                pool.close().await;
                return;
            }
            // All senders dropped == shutdown. Drain, flush, then exit.
            maybe = rx.recv() => {
                match maybe {
                    Some(ev) => {
                        batch.push(ev);
                        // Opportunistically drain up to one full batch so a
                        // burst is written in a single statement.
                        while batch.len() < cfg.batch_size {
                            match rx.try_recv() {
                                Ok(ev) => batch.push(ev),
                                Err(_) => break,
                            }
                        }
                        if batch.len() >= cfg.batch_size {
                            flush(&pool, &cfg, &mut batch, &dropped, &written, &errors).await;
                            deadline = Instant::now() + cfg.flush_interval;
                        }
                    }
                    None => {
                        if !batch.is_empty() {
                            flush(&pool, &cfg, &mut batch, &dropped, &written, &errors).await;
                        }
                        pool.close().await;
                        return;
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                if !batch.is_empty() {
                    flush(&pool, &cfg, &mut batch, &dropped, &written, &errors).await;
                }
                deadline = Instant::now() + cfg.flush_interval;
            }
        }
    }
}

// The `_logs` table created by `migrations-logs/00001` does not yet have the
// `request_id`/`trace_id`/`span_id` columns (they are commented out pending
// step 2). Those fields stay on `AuditEvent` as the step-2 seam but are not
// persisted yet; add them back here when the columns exist.
const INSERT_BATCH: &str = r#"
INSERT INTO _logs (
    id, level, severity, message, data,
    action, actor_id, actor_collection,
    target_collection, target_id,
    method, path, status_code, duration_ms,
    ip_address, user_agent, outcome
)
SELECT * FROM UNNEST(
    $1::text[], $2::int[], $3::text[], $4::text[], $5::jsonb[],
    $6::text[], $7::text[], $8::text[],
    $9::text[], $10::text[],
    $11::text[], $12::text[], $13::int[], $14::float8[],
    $15::text[], $16::text[], $17::text[]
)"#;

/// One statement for the whole batch, using UNNEST so we send 17 arrays
/// rather than 100 × 17 bind parameters (Postgres caps at 65535 binds, and
/// 100 rows × 17 columns would already be 1,700 — fine, but UNNEST keeps
/// the statement text constant, which lets it be prepared and cached).
async fn flush(
    pool: &Pool<Postgres>,
    cfg: &WriterConfig,
    batch: &mut Vec<AuditEvent>,
    dropped: &Arc<AtomicU64>,
    written: &Arc<AtomicU64>,
    errors: &Arc<AtomicU64>,
) {
    if batch.is_empty() {
        return;
    }
    let count = batch.len();

    let mut ids = Vec::with_capacity(count);
    let mut levels = Vec::with_capacity(count);
    let mut severities = Vec::with_capacity(count);
    let mut messages = Vec::with_capacity(count);
    let mut data = Vec::with_capacity(count);
    let mut actions = Vec::with_capacity(count);
    let mut actor_ids = Vec::with_capacity(count);
    let mut actor_collections = Vec::with_capacity(count);
    let mut target_collections = Vec::with_capacity(count);
    let mut target_ids = Vec::with_capacity(count);
    let mut methods = Vec::with_capacity(count);
    let mut paths = Vec::with_capacity(count);
    let mut status_codes = Vec::with_capacity(count);
    let mut durations_ms = Vec::with_capacity(count);
    let mut ip_addresses = Vec::with_capacity(count);
    let mut user_agents = Vec::with_capacity(count);
    let mut outcomes = Vec::with_capacity(count);

    for ev in batch.drain(..) {
        ids.push(Uuid::new_v4().to_string());
        levels.push(ev.severity.as_level());
        severities.push(ev.severity.as_str().to_string());
        messages.push(ev.message);
        data.push(ev.data);
        actions.push(ev.action);
        actor_ids.push(ev.actor_id);
        actor_collections.push(ev.actor_collection);
        target_collections.push(ev.target_collection);
        target_ids.push(ev.target_id);
        methods.push(ev.method);
        paths.push(ev.path);
        status_codes.push(ev.status_code.map(i32::from));
        durations_ms.push(ev.duration_ms);
        ip_addresses.push(ev.ip_address);
        user_agents.push(ev.user_agent);
        outcomes.push(ev.outcome.as_str().to_string());
    }

    let insert = sqlx::query(INSERT_BATCH)
        .bind(ids)
        .bind(levels)
        .bind(severities)
        .bind(messages)
        .bind(data)
        .bind(actions)
        .bind(actor_ids)
        .bind(actor_collections)
        .bind(target_collections)
        .bind(target_ids)
        .bind(methods)
        .bind(paths)
        .bind(status_codes)
        .bind(durations_ms)
        .bind(ip_addresses)
        .bind(user_agents)
        .bind(outcomes)
        .execute(pool);

    match tokio::time::timeout(cfg.insert_timeout, insert).await {
        Ok(Ok(_)) => {
            written.fetch_add(count as u64, Ordering::Relaxed);
        }
        Ok(Err(e)) => {
            errors.fetch_add(count as u64, Ordering::Relaxed);
            tracing::error!(error = %e, count, "audit log batch insert failed; batch lost");
        }
        Err(_) => {
            // A lost batch counted as written is an audit gap you cannot
            // detect — count a timeout as dropped.
            dropped.fetch_add(count as u64, Ordering::Relaxed);
            tracing::error!(count, "audit log batch insert timed out; batch dropped");
        }
    }
}
