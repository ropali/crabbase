use serde::Deserialize;

/// One `_logs` row as returned by `GET /api/logs`.
#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub created: String,
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
    pub data: serde_json::Value,
}

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogListResponse {
    /// `false` when `[logging.database]` is disabled on the server.
    pub enabled: bool,
    pub items: Vec<LogEntry>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct LogBucket {
    pub bucket: String,
    pub count: i64,
}

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogStatsResponse {
    pub enabled: bool,
    pub total: i64,
    pub errors: i64,
    pub warnings: i64,
    pub avg_duration_ms: Option<f64>,
    pub buckets: Vec<LogBucket>,
}
