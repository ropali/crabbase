use std::{sync::Arc, time::Duration};

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    handler::{JobContext, JobError},
    handlers::backup::{BackupCreateHandler, BackupPruneHandler},
    handlers::cleanup::{DbVacuumHandler, LogsCleanupHandler, OtpCleanupHandler},
    registry::Registry,
    repository::JobRepository,
    scheduler::run_scheduler,
    settings::{DatabaseParts, load_jobs_settings},
    worker::Worker,
};

pub use crate::handlers::backup::preflight;

pub struct JobRuntime {
    cancel: CancellationToken,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

pub async fn start(pool: sqlx::PgPool, database: DatabaseParts) -> Result<JobRuntime, JobError> {
    let cancel = CancellationToken::new();
    let mut tasks = Vec::new();

    let jobs = load_jobs_settings(&pool)
        .await
        .map_err(|e| JobError::Retryable(e.to_string()))?;

    if !jobs.enabled {
        info!("job runtime disabled by the `jobs` setting");
        return Ok(JobRuntime { cancel, tasks });
    }

    let repo = JobRepository::new(pool.clone());
    let ctx = JobContext {
        pool: pool.clone(),
        database,
    };

    let registry = Arc::new(build_registry());
    let base_worker_id = worker_id();

    if jobs.scheduler {
        tasks.push(tokio::spawn(run_scheduler(
            repo.clone(),
            Duration::from_secs(jobs.tick_seconds.max(1)),
            cancel.clone(),
        )));
    }

    if jobs.worker {
        for i in 0..jobs.concurrency.max(1) {
            let worker = Worker::new(
                repo.clone(),
                registry.clone(),
                ctx.clone(),
                format!("{base_worker_id}-{i}"),
                Duration::from_secs(jobs.lease_seconds.max(1)),
            );
            tasks.push(tokio::spawn(worker.run(cancel.clone())));
        }
    } else {
        warn!("job worker disabled by the `jobs` setting");
    }

    info!(
        scheduler = jobs.scheduler,
        workers = if jobs.worker {
            jobs.concurrency.max(1)
        } else {
            0
        },
        "job runtime started"
    );

    Ok(JobRuntime { cancel, tasks })
}

fn build_registry() -> Registry {
    Registry::new(vec![
        Arc::new(BackupCreateHandler),
        Arc::new(BackupPruneHandler),
        Arc::new(LogsCleanupHandler),
        Arc::new(OtpCleanupHandler),
        Arc::new(DbVacuumHandler),
    ])
}

fn worker_id() -> String {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "worker".to_string());
    format!("{host}-{}", uuid::Uuid::new_v4().simple())
}

impl JobRuntime {
    pub async fn shutdown(self) {
        self.cancel.cancel();
        for task in self.tasks {
            let _ = task.await;
        }
    }
}
