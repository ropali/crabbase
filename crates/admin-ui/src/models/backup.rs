use serde::Deserialize;

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub filename: String,
    pub status: String,
    pub triggered_by: String,
    pub storage_backend: String,
    pub size_bytes: Option<i64>,
    pub checksum: Option<String>,
    pub error: Option<String>,
    pub created: String,
    pub finished: Option<String>,
}

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListResponse {
    pub items: Vec<Backup>,
    pub total: i64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cron {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub schedule: String,
    pub enabled: bool,
    pub next_run: Option<String>,
    pub last_run: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}
