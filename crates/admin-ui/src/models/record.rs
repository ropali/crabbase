use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateRecordRequest {
    pub data: serde_json::Map<String, serde_json::Value>,
}
