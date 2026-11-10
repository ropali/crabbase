use std::time::Duration;

use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::{models::Job, settings::DatabaseParts};

#[derive(Debug, Clone)]
pub struct JobContext {
    pub pool: PgPool,
    pub database: DatabaseParts,
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("permanent: {0}")]
    Permanent(String),
    #[error("retryable: {0}")]
    Retryable(String),
    #[error("cancelled")]
    Cancelled,
}

impl JobError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, JobError::Retryable(_))
    }
}

#[async_trait::async_trait]
pub trait JobHandler: Send + Sync {
    fn kind(&self) -> &'static str;

    /// Hard upper bound. The worker wraps the handler in `tokio::time::timeout`.
    fn timeout(&self) -> Duration {
        Duration::from_secs(5 * 60)
    }

    async fn run(&self, ctx: &JobContext, job: &Job) -> Result<JsonValue, JobError>;
}
