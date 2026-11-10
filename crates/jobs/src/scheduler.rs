use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::repository::JobRepository;

/// Dispatches due crons into the job queue. It never executes jobs itself.
pub async fn run_scheduler(repo: JobRepository, tick: Duration, cancel: CancellationToken) {
    let mut interval = tokio::time::interval(tick);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {}
        }

        if let Err(e) = repo.ensure_next_run().await {
            error!(error = %e, "failed to initialize cron next_run");
        }

        match repo.dispatch_due(50).await {
            Ok(0) => {}
            Ok(n) => info!(count = n, "cron jobs enqueued"),
            Err(e) => error!(error = %e, "cron dispatch failed"),
        }
    }
}
