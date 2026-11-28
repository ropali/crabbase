//! Read API for the audit trail written by `crates/core/src/logging`.
//!
//! The rows may live in a different database or schema than the application
//! (see `docs/logging.md`), so every handler reads through `AppState::logs`
//! rather than the application pool. When database logging is off that pool is
//! `None` and the endpoints answer `{ "enabled": false }` instead of failing —
//! the UI shows a configuration hint rather than an error toast.

use axum::{
    Json, Router,
    extract::{Query, State},
    middleware::from_fn_with_state,
    routing::get,
};
use chrono::{DateTime, Duration, Utc};
use crabbase_core::errors::APIError;
use crabbase_db::repositories::logs::LogFilter;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{middleware::auth::require_admin, state::AppState};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/stats", get(stats))
        .layer(from_fn_with_state(state.clone(), require_admin))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    /// `ALL` or a severity name (`Error`, `WARN`, …).
    pub level: Option<String>,
    /// Action prefix, e.g. `record.`.
    pub action: Option<String>,
    pub actor: Option<String>,
    pub outcome: Option<String>,
    pub search: Option<String>,
    /// `1h` / `6h` / `24h` / `7d` / `30d` / `all`.
    pub range: Option<String>,
    pub superusers: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub range: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(params): Query<LogQuery>,
) -> Result<Json<Value>, APIError> {
    let Some(repo) = state.logs_repo() else {
        return Ok(Json(json!({
            "enabled": false,
            "items": [],
            "total": 0,
            "page": 1,
            "perPage": 50,
        })));
    };

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);

    let filter = LogFilter {
        page,
        per_page,
        severity: severity_param(params.level.as_deref()),
        action: non_empty(params.action),
        actor_id: non_empty(params.actor),
        outcome: non_empty(params.outcome),
        search: non_empty(params.search),
        since: range_since(params.range.as_deref()),
        superusers_only: params.superusers.unwrap_or(false),
    };

    let (items, total) = repo.list(&filter).await?;

    Ok(Json(json!({
        "enabled": true,
        "items": items,
        "total": total,
        "page": page,
        "perPage": per_page,
    })))
}

async fn stats(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<Value>, APIError> {
    let Some(repo) = state.logs_repo() else {
        return Ok(Json(json!({
            "enabled": false,
            "total": 0,
            "errors": 0,
            "warnings": 0,
            "avgDurationMs": null,
            "buckets": [],
        })));
    };

    let range = params.range.as_deref().unwrap_or("24h");
    // An unbounded "all" would scan the whole table to draw a chart; cap it.
    let since = range_since(Some(range)).unwrap_or_else(|| Utc::now() - Duration::days(30));

    let stats = repo.stats(since, granularity(range)).await?;

    Ok(Json(json!({
        "enabled": true,
        "total": stats.total,
        "errors": stats.errors,
        "warnings": stats.warnings,
        "avgDurationMs": stats.avg_duration_ms,
        "buckets": stats.buckets,
    })))
}

/// `ALL` (and anything unrecognised) means "no severity filter" rather than a
/// 400: the UI sends the label straight from a `<select>`.
fn severity_param(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("all") {
        return None;
    }

    let upper = value.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
    )
    .then_some(upper)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn range_since(range: Option<&str>) -> Option<DateTime<Utc>> {
    let window = match range?.trim().to_ascii_lowercase().as_str() {
        "5m" => Duration::minutes(5),
        "15m" => Duration::minutes(15),
        "30m" => Duration::minutes(30),
        "1h" => Duration::hours(1),
        "2h" => Duration::hours(2),
        "6h" => Duration::hours(6),
        "24h" => Duration::hours(24),
        "7d" => Duration::days(7),
        "30d" => Duration::days(30),
        // "all" (and anything unrecognised): no lower bound.
        _ => return None,
    };

    Some(Utc::now() - window)
}

/// Bucket size that keeps the chart at a readable number of bars.
fn granularity(range: &str) -> &'static str {
    match range.trim().to_ascii_lowercase().as_str() {
        "5m" | "15m" | "30m" | "1h" | "2h" | "6h" => "minute",
        "24h" | "7d" => "hour",
        _ => "day",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_accepts_ui_labels_and_rejects_noise() {
        assert_eq!(severity_param(Some("Error")).as_deref(), Some("ERROR"));
        assert_eq!(severity_param(Some("warn")).as_deref(), Some("WARN"));
        assert_eq!(severity_param(Some("ALL")), None);
        assert_eq!(severity_param(Some("  ")), None);
        assert_eq!(severity_param(Some("bogus")), None);
    }

    #[test]
    fn ranges_map_to_windows_and_buckets() {
        assert_eq!(granularity("5M"), "minute");
        assert_eq!(granularity("15m"), "minute");
        assert_eq!(granularity("30m"), "minute");
        assert_eq!(granularity("1H"), "minute");
        assert_eq!(granularity("2h"), "minute");
        assert_eq!(granularity("24h"), "hour");
        assert_eq!(granularity("30d"), "day");
        assert!(range_since(Some("all")).is_none());
        assert!(range_since(Some("5m")).is_some());
        assert!(range_since(Some("2h")).is_some());
        assert!(range_since(Some("7d")).is_some());
    }
}
