use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub system: bool,
    #[serde(default)]
    pub collection_type: String,
    #[serde(default)]
    pub records: u32,
    #[serde(default)]
    pub modified: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CollectionListResponse {
    pub items: Vec<Collection>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
