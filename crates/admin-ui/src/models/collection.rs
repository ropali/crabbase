use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(deserialize_with = "deserialize_data_type")]
    pub data_type: String,
    pub index: bool,
    pub related_to: Option<String>,
}

fn deserialize_data_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Object(map) => {
            if let Some(key) = map.keys().next() {
                Ok(key.clone())
            } else {
                Ok("Unknown".to_string())
            }
        }
        _ => Ok("Unknown".to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub system: bool,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub collection_type: String,

    pub updated: String,
    pub created: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub columns: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CollectionListResponse {
    pub items: Vec<Collection>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: serde_json::Value,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RecordsResponse {
    pub items: Vec<Record>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}
