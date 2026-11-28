use std::collections::HashMap;

use chrono::{DateTime, Duration, Timelike, Utc};
use crabbase_core::errors::RepositoryError;
use serde::{Deserialize, Serialize};
use sqlx::query_builder::QueryBuilder;
use sqlx::{FromRow, Pool, Postgres};

/// One `_logs` row, shaped for the admin activity page.
///
/// The table itself is created by `migrations-logs/00001` in the *log*
/// target, which may be a different database than the application's. The pool
/// this repository holds is the read pool for that target.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub created: DateTime<Utc>,
    /// Legacy numeric level, kept in sync with `severity`.
    pub level: i32,
    pub severity: String,
    pub action: Option<String>,
    pub message: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub duration_ms: Option<f64>,
    pub actor_id: Option<String>,
    pub actor_collection: Option<String>,
    pub target_collection: Option<String>,
    pub target_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub outcome: String,
    /// Everything that was not a known column when the event was emitted.
    pub data: serde_json::Value,
}

/// Server-side filters for the activity page. Every field is optional so the
/// same struct serves "last 50 of everything" and a narrow search.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub page: i64,
    pub per_page: i64,
    /// `TRACE` / `DEBUG` / `INFO` / `WARN` / `ERROR`.
    pub severity: Option<String>,
    /// Prefix match, so `record.` selects every record action.
    pub action: Option<String>,
    pub actor_id: Option<String>,
    pub outcome: Option<String>,
    /// Substring match over message, path, action and actor.
    pub search: Option<String>,
    pub since: Option<DateTime<Utc>>,
    /// Restrict to `_superusers` actors.
    pub superusers_only: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LogBucket {
    pub bucket: DateTime<Utc>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogStats {
    pub total: i64,
    pub errors: i64,
    pub warnings: i64,
    pub avg_duration_ms: Option<f64>,
    pub buckets: Vec<LogBucket>,
}

pub struct LogsRepository {
    db: Pool<Postgres>,
}

impl LogsRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    /// One page of entries plus the total matching the same filter.
    ///
    /// Two statements rather than `count(*) OVER ()` so an empty last page
    /// still reports the real total instead of zero.
    pub async fn list(&self, filter: &LogFilter) -> Result<(Vec<LogEntry>, i64), RepositoryError> {
        let mut rows: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, created, level, severity, action, message, method, path, \
             status_code, duration_ms, actor_id, actor_collection, target_collection, \
             target_id, ip_address, user_agent, outcome, data \
             FROM _logs WHERE TRUE",
        );
        push_filters(&mut rows, filter);
        // `id` breaks ties: `created` has microsecond resolution and a batch
        // of request rows can share a timestamp, which would otherwise make
        // pagination repeat or skip rows.
        rows.push(" ORDER BY created DESC, id DESC LIMIT ")
            .push_bind(filter.per_page)
            .push(" OFFSET ")
            .push_bind((filter.page - 1) * filter.per_page);

        let entries = rows
            .build_query_as::<LogEntry>()
            .fetch_all(&self.db)
            .await?;

        let mut count: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT count(*) FROM _logs WHERE TRUE");
        push_filters(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.db).await?;

        Ok((entries, total))
    }

    /// Aggregate counters for the telemetry strip and its bar chart.
    ///
    /// `granularity` is a Postgres unit (`minute`, `hour`, `day`) chosen by the
    /// caller from the selected time range, so a 1 h view buckets by minute and
    /// a 30 d view by day. Quiet periods are emitted as zero-count buckets so
    /// the chart keeps a truthful time axis.
    pub async fn stats(
        &self,
        since: DateTime<Utc>,
        granularity: &str,
    ) -> Result<LogStats, RepositoryError> {
        let until = Utc::now();

        let (total, errors, warnings, avg_duration_ms): (i64, i64, i64, Option<f64>) =
            sqlx::query_as(
                "SELECT count(*), \
                        count(*) FILTER (WHERE severity = 'ERROR'), \
                        count(*) FILTER (WHERE severity = 'WARN'), \
                        avg(duration_ms) \
                   FROM _logs WHERE created >= $1",
            )
            .bind(since)
            .fetch_one(&self.db)
            .await?;

        // Gaps are filled with zero rows below so the chart's bar slots line
        // up with real time instead of collapsing quiet minutes together.
        let rows: Vec<LogBucket> = sqlx::query_as(
            "SELECT date_trunc($1, created AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AS bucket, \
                    count(*) AS count \
               FROM _logs WHERE created >= $2 \
              GROUP BY 1 ORDER BY 1",
        )
        .bind(granularity)
        .bind(since)
        .fetch_all(&self.db)
        .await?;

        let counts: HashMap<DateTime<Utc>, i64> = rows
            .into_iter()
            .map(|row| (row.bucket, row.count))
            .collect();
        let buckets = fill_buckets(since, until, granularity, &counts);

        Ok(LogStats {
            total,
            errors,
            warnings,
            avg_duration_ms,
            buckets,
        })
    }
}

/// The API's ranges top out at 6 h of minutes, 7 d of hours or 30 d of days
/// (≤ 361 buckets); this is only a guard against an unbounded caller.
const MAX_BUCKETS: usize = 2_000;

fn truncate_bucket(ts: DateTime<Utc>, granularity: &str) -> DateTime<Utc> {
    match granularity {
        "minute" => ts
            .with_second(0)
            .and_then(|ts| ts.with_nanosecond(0))
            .unwrap_or(ts),
        "hour" => ts
            .with_minute(0)
            .and_then(|ts| ts.with_second(0))
            .and_then(|ts| ts.with_nanosecond(0))
            .unwrap_or(ts),
        _ => ts
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|naive| naive.and_utc())
            .unwrap_or(ts),
    }
}

fn bucket_step(granularity: &str) -> Duration {
    match granularity {
        "minute" => Duration::minutes(1),
        "hour" => Duration::hours(1),
        _ => Duration::days(1),
    }
}

/// Expand the sparse SQL result into one entry per bucket between `since`
/// and `until`, inserting zeroes for quiet periods. Truncation is done in UTC
/// both here and in SQL, so the series does not depend on the database
/// session's TimeZone.
fn fill_buckets(
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    granularity: &str,
    counts: &HashMap<DateTime<Utc>, i64>,
) -> Vec<LogBucket> {
    let step = bucket_step(granularity);
    let mut cursor = truncate_bucket(since, granularity);
    let end = truncate_bucket(until, granularity);
    let mut buckets = Vec::new();

    while cursor <= end && buckets.len() < MAX_BUCKETS {
        buckets.push(LogBucket {
            bucket: cursor,
            count: counts.get(&cursor).copied().unwrap_or(0),
        });
        cursor += step;
    }

    buckets
}

fn push_filters(qb: &mut QueryBuilder<Postgres>, filter: &LogFilter) {
    if let Some(since) = filter.since {
        qb.push(" AND created >= ").push_bind(since);
    }

    if let Some(severity) = &filter.severity {
        qb.push(" AND severity = ").push_bind(severity.clone());
    }

    if let Some(action) = &filter.action {
        qb.push(" AND action LIKE ")
            .push_bind(format!("{}%", escape_like(action)));
    }

    if let Some(actor_id) = &filter.actor_id {
        qb.push(" AND actor_id = ").push_bind(actor_id.clone());
    }

    if let Some(outcome) = &filter.outcome {
        qb.push(" AND outcome = ").push_bind(outcome.clone());
    }

    if let Some(search) = &filter.search {
        let term = format!("%{}%", escape_like(search));
        qb.push(" AND (message ILIKE ")
            .push_bind(term.clone())
            .push(" OR path ILIKE ")
            .push_bind(term.clone())
            .push(" OR action ILIKE ")
            .push_bind(term.clone())
            .push(" OR actor_id ILIKE ")
            .push_bind(term)
            .push(")");
    }

    if filter.superusers_only {
        qb.push(" AND actor_collection = ")
            .push_bind("_superusers".to_string());
    }
}

/// `_` and `%` are LIKE wildcards; a user typing them means the literal
/// character, not "match anything".
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn quiet_buckets_are_filled_with_zero() {
        let since = utc(2026, 11, 28, 10, 30);
        let until = utc(2026, 11, 28, 13, 10);
        let mut counts = HashMap::new();
        counts.insert(utc(2026, 11, 28, 11, 0), 4);

        let buckets = fill_buckets(since, until, "hour", &counts);

        assert_eq!(buckets.len(), 4); // 10:00, 11:00, 12:00, 13:00
        assert_eq!(buckets[0].count, 0);
        assert_eq!(buckets[1].count, 4);
        assert_eq!(buckets[2].count, 0);
        assert_eq!(buckets[3].count, 0);
    }

    #[test]
    fn minute_buckets_cross_midnight() {
        let since = utc(2026, 11, 28, 23, 58);
        let until = utc(2026, 11, 29, 0, 1);

        let buckets = fill_buckets(since, until, "minute", &HashMap::new());

        assert_eq!(buckets.len(), 4);
        assert_eq!(buckets[3].bucket, utc(2026, 11, 29, 0, 1));
    }

    #[test]
    fn day_buckets_truncate_to_utc_midnight() {
        let since = utc(2026, 11, 27, 23, 59);
        let until = utc(2026, 11, 29, 0, 1);

        let buckets = fill_buckets(since, until, "day", &HashMap::new());

        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].bucket, utc(2026, 11, 27, 0, 0));
        assert_eq!(buckets[2].bucket, utc(2026, 11, 29, 0, 0));
    }
}
