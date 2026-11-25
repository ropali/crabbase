use serde_json::{Value, json};

use crate::{
    handler::{JobContext, JobError, JobHandler},
    models::Job,
};

pub struct LogsCleanupHandler;

#[async_trait::async_trait]
impl JobHandler for LogsCleanupHandler {
    fn kind(&self) -> &'static str {
        "logs.cleanup"
    }

    async fn run(&self, ctx: &JobContext, job: &Job) -> Result<Value, JobError> {
        let older_than_days = job
            .payload
            .get("olderThanDays")
            .and_then(Value::as_i64)
            .unwrap_or(30)
            .max(1);

        // Logging can live in another schema or another database entirely, so
        // this deliberately opens the log connection instead of borrowing the
        // application pool.
        let Some(logs) = ctx.database.logs.as_ref() else {
            return Ok(json!({
                "olderThanDays": older_than_days,
                "deleted": 0,
                "skipped": "database logging is disabled",
            }));
        };

        let pool = crabbase_db::connection::pool(&logs.url, &logs.schema)
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        let table = format!(
            "{}.{}",
            crabbase_core::utils::string_utils::quote_ident(&logs.schema),
            crabbase_core::utils::string_utils::quote_ident("_logs"),
        );

        let result = sqlx::query(&format!(
            "DELETE FROM {table} WHERE created < now() - $1 * interval '1 day'"
        ))
        .bind(older_than_days)
        .execute(&pool)
        .await
        .map_err(|e| JobError::Retryable(e.to_string()))?;

        pool.close().await;

        Ok(json!({
            "olderThanDays": older_than_days,
            "deleted": result.rows_affected(),
        }))
    }
}

pub struct OtpCleanupHandler;

#[async_trait::async_trait]
impl JobHandler for OtpCleanupHandler {
    fn kind(&self) -> &'static str {
        "otp.cleanup"
    }

    async fn run(&self, ctx: &JobContext, _job: &Job) -> Result<Value, JobError> {
        let result = sqlx::query(
            "DELETE FROM _otps WHERE created + COALESCE(valid_for, 300) * interval '1 second' < now()",
        )
        .execute(&ctx.pool)
        .await
        .map_err(|e| JobError::Retryable(e.to_string()))?;

        Ok(json!({ "deleted": result.rows_affected() }))
    }
}

pub struct DbVacuumHandler;

#[async_trait::async_trait]
impl JobHandler for DbVacuumHandler {
    fn kind(&self) -> &'static str {
        "db.vacuum"
    }

    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(60 * 60)
    }

    async fn run(&self, ctx: &JobContext, _job: &Job) -> Result<Value, JobError> {
        // VACUUM cannot run inside a transaction and cannot be a prepared
        // statement; `raw_sql` uses the simple protocol.
        sqlx::raw_sql("VACUUM (ANALYZE)")
            .execute(&ctx.pool)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        Ok(json!({ "status": "vacuumed" }))
    }
}
