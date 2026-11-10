use std::{sync::Arc, time::Duration};

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    handler::{JobContext, JobError},
    models::Job,
    registry::Registry,
    repository::JobRepository,
};

pub struct Worker {
    repo: JobRepository,
    registry: Arc<Registry>,
    ctx: JobContext,
    worker_id: String,
    lease: Duration,
}

impl Worker {
    pub fn new(
        repo: JobRepository,
        registry: Arc<Registry>,
        ctx: JobContext,
        worker_id: String,
        lease: Duration,
    ) -> Self {
        Self {
            repo,
            registry,
            ctx,
            worker_id,
            lease,
        }
    }

    pub async fn run(self, cancel: CancellationToken) {
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tick.tick() => {}
            }

            if let Err(e) = self.repo.reap_expired_leases().await {
                error!(error = %e, "lease reaper failed");
            }

            loop {
                match self.repo.claim(&self.worker_id, self.lease).await {
                    Ok(Some(job)) => self.execute(job, &cancel).await,
                    Ok(None) => break,
                    Err(e) => {
                        error!(error = %e, "claim failed");
                        break;
                    }
                }
            }
        }

        info!(worker = %self.worker_id, "worker stopped");
    }

    async fn execute(&self, job: Job, cancel: &CancellationToken) {
        let Some(handler) = self.registry.get(&job.kind) else {
            let err = JobError::Permanent(format!("unknown job kind: {}", job.kind));
            let _ = self
                .repo
                .fail(&job, &self.worker_id, &err.to_string(), false)
                .await;
            return;
        };

        let heartbeat = LeaseGuard(tokio::spawn({
            let repo = self.repo.clone();
            let worker_id = self.worker_id.clone();
            let job_id = job.id;
            let lease = self.lease;
            let period = std::cmp::max(lease / 2, Duration::from_secs(1));
            async move {
                let mut hb = tokio::time::interval(period);
                hb.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    hb.tick().await;
                    match repo.heartbeat(job_id, &worker_id, lease).await {
                        Ok(true) => {}
                        Ok(false) => break, // lease was lost; stop renewing
                        Err(e) => error!(error = %e, "heartbeat failed"),
                    }
                }
            }
        }));

        let outcome = tokio::select! {
            _ = cancel.cancelled() => Err(JobError::Cancelled),
            result = tokio::time::timeout(handler.timeout(), handler.run(&self.ctx, &job)) => {
                match result {
                    Ok(result) => result,
                    Err(_) => Err(JobError::Retryable("handler timed out".to_string())),
                }
            }
        };

        let result = match outcome {
            Ok(value) => self
                .repo
                .complete(&job, &self.worker_id, &value)
                .await
                .map(|_| ()),
            Err(JobError::Cancelled) => {
                self.repo
                    .fail(&job, &self.worker_id, "cancelled by shutdown", true)
                    .await
            }
            Err(err) => {
                self.repo
                    .fail(&job, &self.worker_id, &err.to_string(), err.is_retryable())
                    .await
            }
        };

        if let Err(e) = result {
            error!(job_id = %job.id, error = %e, "failed to record job outcome");
        }

        drop(heartbeat);
    }
}

/// Aborts the heartbeat task when dropped, so every code path stops it.
struct LeaseGuard(tokio::task::JoinHandle<()>);

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

// `Arc<Registry>` is cheap to clone; kept explicit so the worker can be moved
// into a spawned task without holding a reference to the runtime.
impl Clone for Worker {
    fn clone(&self) -> Self {
        Self {
            repo: self.repo.clone(),
            registry: self.registry.clone(),
            ctx: self.ctx.clone(),
            worker_id: self.worker_id.clone(),
            lease: self.lease,
        }
    }
}
