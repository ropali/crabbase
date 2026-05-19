use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use sqlx::types::Json;
use sqlx::{FromRow, Row};

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

    #[sqlx(json)]
    pub fields: Vec<Column>,

    #[sqlx(json)]
    pub indexes: Vec<Column>,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Serialize)]
pub struct CollectionListResponse {
    pub items: Vec<Collection>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataTypes {
    PlainText,
    RichText,
    Number,
    Bool,
    Email,
    URL,
    Datetime,
    AutoDatetime(String),
    File,
    Relation(String),
    Select,
    JSON,
    GeoPoint,
}

impl DataTypes {
    pub fn to_db_type(self) -> String {
        match self {
            Self::PlainText
            | Self::RichText
            | Self::Email
            | Self::URL
            | Self::File
            | Self::Datetime
            | Self::Select
            | Self::JSON
            | Self::GeoPoint => "TEXT".to_owned(),
            DataTypes::Number => "INTEGER".to_owned(),
            DataTypes::Bool => "INTEGER CHECK (is_active IN (0, 1))".to_owned(),
            DataTypes::AutoDatetime(option) => "TEXT".to_owned(),
            DataTypes::Relation(field) => {
                format!("FOREIGN KEY (id) REFERENCES {} (id)", field)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Column {
    pub name: String,
    #[serde(deserialize_with = "deserialize_data_type")]
    pub data_type: DataTypes,
    pub index: bool,
}

fn deserialize_data_type<'de, D>(deserializer: D) -> Result<DataTypes, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    // First, accept the current enum representation.
    if let Ok(data_type) = serde_json::from_value::<DataTypes>(value.clone()) {
        return Ok(data_type);
    }

    // Backward compatibility: accept legacy SQL-ish type strings in stored JSON.
    if let Some(raw) = value.as_str() {
        return match raw.to_ascii_uppercase().as_str() {
            "TEXT" => Ok(DataTypes::PlainText),
            "INTEGER" | "INT" => Ok(DataTypes::Number),
            "BOOLEAN" | "BOOL" => Ok(DataTypes::Bool),
            "DATE" => Ok(DataTypes::Datetime),
            other => Err(D::Error::custom(format!(
                "unsupported data_type string: {other}"
            ))),
        };
    }

    Err(D::Error::custom("invalid data_type format"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub columns: Option<Vec<Column>>,
}
