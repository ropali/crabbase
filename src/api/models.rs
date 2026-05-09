use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: serde_json::Value,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Serialize)]
pub struct RecordListResponse {
    pub items: Vec<Record>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecordRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecordRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}
