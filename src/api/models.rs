use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Record {
    pub id: String,
    pub data: serde_json::Value,
    #[sqlx(rename = "created_at")]
    pub created: String,
    #[sqlx(rename = "updated_at")]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: String,
    #[sqlx(rename = "created_at")]
    pub created: String,
    #[sqlx(rename = "updated_at")]
    pub updated: String,
}

#[derive(Debug, Serialize)]
pub struct CollectionListResponse {
    pub items: Vec<Collection>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
