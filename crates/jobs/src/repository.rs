use std::{str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use crabbase_core::errors::RepositoryError;
use croner::Cron as CronExpr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::warn;
use uuid::Uuid;

use crate::models::{Backup, Cron, Job};

#[derive(Debug, Clone)]
pub struct NewJob {
    pub kind: String,
    pub payload: Value,
    pub priority: i16,
    pub run_at: Option<DateTime<Utc>>,
    pub max_attempts: i32,
    pub dedupe_key: Option<String>,
    pub cron_id: Option<Uuid>,
}

impl NewJob {
    pub fn new(kind: &str) -> Self {
        Self::default_with_kind(kind)
    }

    fn default_with_kind(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            payload: Value::Object(Default::default()),
            priority: 100,
            run_at: None,
            max_attempts: 3,
            dedupe_key: None,
            cron_id: None,
        }
    }
}

impl Default for NewJob {
    fn default() -> Self {
        Self::default_with_kind("")
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobListResponse {
    pub items: Vec<Job>,
    pub total: i64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListResponse {
    pub items: Vec<Backup>,
    pub total: i64,
    pub page: u64,
    pub per_page: u64,
}

pub fn next_occurrence(
    expression: &str,
    after: DateTime<Utc>,
) -> Result<DateTime<Utc>, croner::errors::CronError> {
    CronExpr::from_str(expression)?.find_next_occurrence(&after, false)
}

pub fn backup_filename() -> String {
    format!(
        "crabbase-{}-{}.dump",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        Uuid::new_v4().simple()
    )
}

#[derive(Debug, Clone)]
pub struct JobRepository {
    pub(crate) db: PgPool,
}

impl JobRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Inserts a job. Returns `None` when a unique key already exists
    /// (`dedupe_key` or an active job for the same cron).
    pub async fn enqueue(&self, job: NewJob) -> Result<Option<Job>, RepositoryError> {
        let row = sqlx::query_as::<_, Job>(
            r#"
            INSERT INTO _jobs (kind, payload, priority, run_at, max_attempts, dedupe_key, cron_id)
            VALUES ($1, $2, $3, COALESCE($4, now()), $5, $6, $7)
            ON CONFLICT DO NOTHING
            RETURNING *
            "#,
        )
        .bind(&job.kind)
        .bind(&job.payload)
        .bind(job.priority)
        .bind(job.run_at)
        .bind(job.max_attempts)
        .bind(&job.dedupe_key)
        .bind(job.cron_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row)
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Job>, RepositoryError> {
        Ok(
            sqlx::query_as::<_, Job>("SELECT * FROM _jobs WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.db)
                .await?,
        )
    }

    pub async fn list(
        &self,
        page: u64,
        per_page: u64,
        status: Option<&str>,
        kind: Option<&str>,
    ) -> Result<JobListResponse, RepositoryError> {
        let offset = ((page.max(1) - 1) * per_page) as i64;
        let limit = per_page as i64;

        let items = sqlx::query_as::<_, Job>(
            r#"
            SELECT * FROM _jobs
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR kind = $2)
            ORDER BY created DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(status)
        .bind(kind)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM _jobs
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR kind = $2)
            "#,
        )
        .bind(status)
        .bind(kind)
        .fetch_one(&self.db)
        .await?;

        Ok(JobListResponse {
            items,
            total,
            page: page.max(1),
            per_page,
        })
    }

    /// Claims one due job with a lease. `FOR UPDATE SKIP LOCKED` makes this
    /// safe with any number of concurrent workers.
    pub async fn claim(
        &self,
        worker_id: &str,
        lease: Duration,
    ) -> Result<Option<Job>, RepositoryError> {
        let lease_secs = lease.as_secs() as i64;

        let job = sqlx::query_as::<_, Job>(
            r#"
            WITH candidate AS (
                SELECT id
                FROM _jobs
                WHERE status = 'queued' AND run_at <= now()
                ORDER BY priority ASC, run_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            UPDATE _jobs j
            SET status        = 'running',
                locked_by     = $1,
                locked_until  = now() + $2 * interval '1 second',
                attempts      = j.attempts + 1,
                started       = COALESCE(j.started, now())
            FROM candidate
            WHERE j.id = candidate.id
            RETURNING j.*
            "#,
        )
        .bind(worker_id)
        .bind(lease_secs)
        .fetch_optional(&self.db)
        .await?;

        Ok(job)
    }

    /// Records success. Returns `false` when the lease was lost (fencing).
    pub async fn complete(
        &self,
        job: &Job,
        worker_id: &str,
        result: &Value,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self.db.begin().await?;

        let updated = sqlx::query_scalar::<_, Option<Uuid>>(
            r#"
            UPDATE _jobs
            SET status = 'succeeded',
                result = $3,
                finished = now(),
                locked_by = NULL,
                locked_until = NULL,
                last_error = NULL
            WHERE id = $1 AND status = 'running' AND locked_by = $2
            RETURNING cron_id
            "#,
        )
        .bind(job.id)
        .bind(worker_id)
        .bind(result)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(cron_id) = updated else {
            tx.rollback().await?;
            return Ok(false);
        };

        if let Some(cron_id) = cron_id {
            sqlx::query(
                "UPDATE _crons SET last_run = now(), last_status = 'succeeded', last_error = NULL, updated = now() WHERE id = $1",
            )
            .bind(cron_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    /// Records failure. Retryable failures requeue with exponential backoff.
    pub async fn fail(
        &self,
        job: &Job,
        worker_id: &str,
        error: &str,
        retryable: bool,
    ) -> Result<(), RepositoryError> {
        let is_final = !retryable || job.attempts >= job.max_attempts;

        let mut tx = self.db.begin().await?;

        if is_final {
            sqlx::query(
                r#"
                UPDATE _jobs
                SET status = 'failed', last_error = $3, finished = now(),
                    locked_by = NULL, locked_until = NULL
                WHERE id = $1 AND status = 'running' AND locked_by = $2
                "#,
            )
            .bind(job.id)
            .bind(worker_id)
            .bind(error)
            .execute(&mut *tx)
            .await?;
        } else {
            // 30s, 60s, 120s, ... capped at ~8.5h.
            let delay = 30_i64 * 2_i64.pow((job.attempts - 1).clamp(0, 10) as u32);

            sqlx::query(
                r#"
                UPDATE _jobs
                SET status = 'queued',
                    last_error = $3,
                    run_at = now() + $4 * interval '1 second',
                    locked_by = NULL,
                    locked_until = NULL
                WHERE id = $1 AND status = 'running' AND locked_by = $2
                "#,
            )
            .bind(job.id)
            .bind(worker_id)
            .bind(error)
            .bind(delay)
            .execute(&mut *tx)
            .await?;
        }

        if let Some(cron_id) = job.cron_id {
            sqlx::query(
                "UPDATE _crons SET last_run = now(), last_status = $2, last_error = $3, updated = now() WHERE id = $1",
            )
            .bind(cron_id)
            .bind(if is_final { "failed" } else { "queued" })
            .bind(error)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Requeues or fails jobs whose worker lease expired, and reconciles the
    /// backup catalog for permanently failed jobs.
    pub async fn reap_expired_leases(&self) -> Result<Vec<Job>, RepositoryError> {
        let jobs = sqlx::query_as::<_, Job>(
            r#"
            UPDATE _jobs
            SET status = CASE
                    WHEN attempts >= max_attempts THEN 'failed'
                    ELSE 'queued'
                END,
                run_at = CASE
                    WHEN attempts >= max_attempts THEN run_at
                    ELSE now()
                END,
                finished = CASE
                    WHEN attempts >= max_attempts THEN now()
                    ELSE finished
                END,
                last_error = COALESCE(last_error, 'worker lease expired'),
                locked_by = NULL,
                locked_until = NULL
            WHERE status = 'running' AND locked_until < now()
            RETURNING *
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        sqlx::query(
            r#"
            UPDATE _backups b
            SET status = 'failed',
                error = COALESCE(b.error, 'job failed or lease expired'),
                finished = now()
            FROM _jobs j
            WHERE b.job_id = j.id
              AND b.status IN ('pending', 'running')
              AND j.status = 'failed'
            "#,
        )
        .execute(&self.db)
        .await?;

        Ok(jobs)
    }

    pub async fn heartbeat(
        &self,
        job_id: Uuid,
        worker_id: &str,
        lease: Duration,
    ) -> Result<bool, RepositoryError> {
        let res = sqlx::query(
            "UPDATE _jobs SET locked_until = now() + $3 * interval '1 second'
             WHERE id = $1 AND status = 'running' AND locked_by = $2",
        )
        .bind(job_id)
        .bind(worker_id)
        .bind(lease.as_secs() as i64)
        .execute(&self.db)
        .await?;

        Ok(res.rows_affected() == 1)
    }
}

#[derive(Debug, Clone)]
pub struct CronRepository {
    db: PgPool,
}

impl CronRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn list(&self) -> Result<Vec<Cron>, RepositoryError> {
        Ok(
            sqlx::query_as::<_, Cron>("SELECT * FROM _crons ORDER BY name")
                .fetch_all(&self.db)
                .await?,
        )
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Cron>, RepositoryError> {
        Ok(
            sqlx::query_as::<_, Cron>("SELECT * FROM _crons WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.db)
                .await?,
        )
    }

    pub async fn update(
        &self,
        id: Uuid,
        schedule: Option<String>,
        enabled: Option<bool>,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<Cron, RepositoryError> {
        let cron = sqlx::query_as::<_, Cron>(
            r#"
            UPDATE _crons
            SET schedule = COALESCE($2, schedule),
                enabled = COALESCE($3, enabled),
                next_run = CASE WHEN $2 IS NOT NULL THEN $4 ELSE next_run END,
                updated = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(schedule)
        .bind(enabled)
        .bind(next_run)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| RepositoryError::NotFound("cron".to_string()))?;

        Ok(cron)
    }

    pub async fn set_last_status(&self, id: Uuid, status: &str) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE _crons SET last_run = now(), last_status = $2, updated = now() WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

impl JobRepository {
    /// Fills `next_run` for enabled crons that do not have one yet, without
    /// running them immediately.
    pub async fn ensure_next_run(&self) -> Result<u64, RepositoryError> {
        let mut tx = self.db.begin().await?;

        let db_now: DateTime<Utc> = sqlx::query_scalar("SELECT now()")
            .fetch_one(&mut *tx)
            .await?;

        let rows = sqlx::query_as::<_, Cron>(
            r#"
            SELECT * FROM _crons
            WHERE enabled AND next_run IS NULL
            ORDER BY created
            LIMIT 100
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut updated = 0;

        for cron in rows {
            match next_occurrence(&cron.schedule, db_now) {
                Ok(next) => {
                    sqlx::query(
                        "UPDATE _crons SET next_run = $2, last_error = NULL, updated = now() WHERE id = $1",
                    )
                    .bind(cron.id)
                    .bind(next)
                    .execute(&mut *tx)
                    .await?;

                    updated += 1;
                }
                Err(e) => {
                    // A bad schedule must not poison the loop. Disable it and surface the error.
                    sqlx::query(
                        "UPDATE _crons SET enabled = FALSE, last_error = $2, updated = now() WHERE id = $1",
                    )
                    .bind(cron.id)
                    .bind(e.to_string())
                    .execute(&mut *tx)
                    .await?;
                    warn!(cron = %cron.name, error = %e, "invalid cron schedule; disabled");
                }
            }
        }

        tx.commit().await?;
        Ok(updated)
    }

    /// Dispatches due crons. Each occurrence is deduped by
    /// `cron:{id}:{scheduled_unix_ts}`, and the partial unique index prevents
    /// overlapping runs of the same cron.
    pub async fn dispatch_due(&self, limit: i64) -> Result<usize, RepositoryError> {
        let mut tx = self.db.begin().await?;

        // Database time is the single clock. App clocks on different replicas may skew.
        let db_now: DateTime<Utc> = sqlx::query_scalar("SELECT now()")
            .fetch_one(&mut *tx)
            .await?;

        let due = sqlx::query_as::<_, Cron>(
            r#"
            SELECT * FROM _crons
            WHERE enabled AND next_run IS NOT NULL AND next_run <= now()
            ORDER BY next_run
            LIMIT $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

        let mut enqueued = 0;
        for cron in &due {
            let scheduled_for = cron.next_run.expect("filtered above");

            let next = match next_occurrence(&cron.schedule, db_now) {
                Ok(next) => next,
                Err(e) => {
                    sqlx::query(
                        "UPDATE _crons SET enabled = FALSE, last_error = $2, updated = now() WHERE id = $1",
                    )
                    .bind(cron.id)
                    .bind(e.to_string())
                    .execute(&mut *tx)
                    .await?;
                    continue;
                }
            };

            // Dedupe on the occurrence, not on wall-clock now: retries and two
            // schedulers racing on the same occurrence produce the same key.
            let dedupe_key = format!("cron:{}:{}", cron.id, scheduled_for.timestamp());

            let job_id = sqlx::query_scalar::<_, Uuid>(
                r#"
                INSERT INTO _jobs (kind, payload, cron_id, dedupe_key, run_at)
                VALUES ($1, $2, $3, $4, now())
                ON CONFLICT DO NOTHING          -- dedupe_key OR uq_jobs_active_cron
                RETURNING id
                "#,
            )
            .bind(&cron.kind)
            .bind(&cron.payload)
            .bind(cron.id)
            .bind(&dedupe_key)
            .fetch_optional(&mut *tx)
            .await?;

            if job_id.is_some() {
                enqueued += 1;
            }

            sqlx::query("UPDATE _crons SET next_run = $2, updated = now() WHERE id = $1")
                .bind(cron.id)
                .bind(next)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(enqueued)
    }
}

// ============================================================
// Backups
// ============================================================

#[derive(Debug, Clone)]
pub struct BackupRepository {
    db: PgPool,
}

impl BackupRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_pending(&self, triggered_by: &str) -> Result<Backup, RepositoryError> {
        let filename = backup_filename();

        let backup = sqlx::query_as::<_, Backup>(
            r#"
            INSERT INTO _backups (filename, status, triggered_by)
            VALUES ($1, 'pending', $2)
            RETURNING *
            "#,
        )
        .bind(&filename)
        .bind(triggered_by)
        .fetch_one(&self.db)
        .await?;

        Ok(backup)
    }

    /// Scheduled path: there is no pre-created row, so create and start one atomically.
    pub async fn create_running(&self, triggered_by: &str) -> Result<Backup, RepositoryError> {
        let backup = self.create_pending(triggered_by).await?;
        self.mark_running(backup.id).await?.ok_or_else(|| {
            RepositoryError::OtherError("failed to mark new backup running".to_string())
        })
    }

    pub async fn mark_running(&self, id: Uuid) -> Result<Option<Backup>, RepositoryError> {
        Ok(sqlx::query_as::<_, Backup>(
            "UPDATE _backups
             SET status = 'running', started = now(), error = NULL
             WHERE id = $1 AND status IN ('pending','running','failed')
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?)
    }

    pub async fn mark_completed(
        &self,
        id: Uuid,
        size_bytes: i64,
        checksum: &str,
        pg_dump_version: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE _backups
            SET status = 'completed', size_bytes = $2, checksum = $3,
                pg_dump_version = $4, finished = now(), error = NULL
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(size_bytes)
        .bind(checksum)
        .bind(pg_dump_version)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE _backups SET status = 'failed', error = $2, finished = now() WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Backup>, RepositoryError> {
        Ok(
            sqlx::query_as::<_, Backup>("SELECT * FROM _backups WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.db)
                .await?,
        )
    }

    pub async fn list(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<BackupListResponse, RepositoryError> {
        let offset = ((page.max(1) - 1) * per_page) as i64;
        let limit = per_page as i64;

        let items = sqlx::query_as::<_, Backup>(
            "SELECT * FROM _backups ORDER BY created DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let total: i64 = sqlx::query_scalar("SELECT count(*) FROM _backups")
            .fetch_one(&self.db)
            .await?;

        Ok(BackupListResponse {
            items,
            total,
            page: page.max(1),
            per_page,
        })
    }

    pub async fn delete_row(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM _backups WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Deletes completed backups older than `keep_days`, but always keeps the
    /// newest successful backup. Returns the filenames that were removed.
    pub async fn prune(&self, keep_days: i64) -> Result<Vec<String>, RepositoryError> {
        let removed = sqlx::query_scalar::<_, String>(
            r#"
            DELETE FROM _backups
            WHERE status = 'completed'
              AND created < now() - $1 * interval '1 day'
              AND id <> (
                  SELECT id FROM _backups
                  WHERE status = 'completed'
                  ORDER BY created DESC
                  LIMIT 1
              )
            RETURNING filename
            "#,
        )
        .bind(keep_days)
        .fetch_all(&self.db)
        .await?;

        Ok(removed)
    }
}

// ============================================================
// Backup service (API-facing)
// ============================================================

#[derive(Debug, Clone)]
pub struct BackupService {
    db: PgPool,
}

impl BackupService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Creates the catalog row and the `backup.create` job in one transaction,
    /// so the UI never sees a backup without a job behind it.
    pub async fn enqueue_manual(&self) -> Result<(Backup, Job), RepositoryError> {
        let mut tx = self.db.begin().await?;
        let filename = backup_filename();

        let backup = sqlx::query_as::<_, Backup>(
            r#"
            INSERT INTO _backups (filename, status, triggered_by)
            VALUES ($1, 'pending', 'manual')
            RETURNING *
            "#,
        )
        .bind(&filename)
        .fetch_one(&mut *tx)
        .await?;

        let job = sqlx::query_as::<_, Job>(
            r#"
            INSERT INTO _jobs (kind, payload)
            VALUES ('backup.create', jsonb_build_object('backupId', $1))
            RETURNING *
            "#,
        )
        .bind(backup.id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE _backups SET job_id = $2 WHERE id = $1")
            .bind(backup.id)
            .bind(job.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((backup, job))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        let repo = BackupRepository::new(self.db.clone());
        let backup = repo
            .get(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound("backup".to_string()))?;

        repo.delete_row(id).await?;

        // Delete the artifact after the row. An orphan file is recoverable; a
        // catalog row pointing at a deleted file is not.
        let dir = crate::settings::load_backup_settings(&self.db)
            .await
            .map(|settings| settings.dir)
            .unwrap_or_default();
        let _ = tokio::fs::remove_file(dir.join(&backup.filename)).await;
        Ok(())
    }
}

/// Reconciles a stale `running` backup row when its job has already failed.
pub async fn reconcile_backup_statuses(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), RepositoryError> {
    sqlx::query(
        r#"
        UPDATE _backups b
        SET status = 'failed',
            error = COALESCE(b.error, 'job failed or lease expired'),
            finished = now()
        FROM _jobs j
        WHERE b.job_id = j.id
          AND b.status IN ('pending', 'running')
          AND j.status = 'failed'
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::next_occurrence;

    #[test]
    fn next_run_is_strictly_after_now() {
        let now = chrono::Utc::now();
        let next = next_occurrence("0 2 * * *", now).unwrap();
        assert!(next > now);
        assert_eq!(next.format("%H:%M").to_string(), "02:00");
    }

    #[test]
    fn invalid_expression_is_rejected() {
        assert!(next_occurrence("not a cron", chrono::Utc::now()).is_err());
    }
}
