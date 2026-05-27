use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use serde_json::{Number, Value};
use sqlx::any::AnyRow;
use sqlx::{Column as _, Error, FromRow, Row};
use tracing::info;

pub type RecordData = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Record {
    pub id: i64,
    pub data: RecordData,
    pub created: String,
    pub updated: String,
}

impl Record {
    pub fn from_row(row: &AnyRow) -> Result<Self, sqlx::Error> {
        let mut data = RecordData::new();

        for c in row.columns() {
            let col = c.name();

            if matches!(col, "id" | "created" | "updated") {
                continue;
            }

            let value = Self::column_to_value(row, col);

            info!("Value: {}", value);

            data.insert(col.to_string(), value);
        }

        Ok(Record {
            id: row.try_get("id")?,
            data,
            created: row.try_get("created")?,
            updated: row.try_get("updated")?,
        })
    }

    pub fn column_to_value(row: &AnyRow, col_name: &str) -> Value {
        if let Ok(v) = row.try_get::<i64, _>(col_name) {
            return Value::Number(Number::from(v));
        }

        if let Ok(v) = row.try_get::<f64, _>(col_name) {
            return serde_json::json!(v);
        }

        if let Ok(v) = row.try_get::<bool, _>(col_name) {
            return Value::Bool(v);
        }

        if let Ok(v) = row.try_get::<String, _>(col_name) {
            return Value::String(v);
        }

        Value::Null
    }
}

#[derive(Debug, Serialize)]
pub struct RecordListResponse {
    pub items: Vec<Record>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRecordRequest {
    pub data: RecordData,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecordRequest {
    pub data: RecordData,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,

    pub fields: Vec<Column>,

    pub indexes: Vec<Column>,
    pub created: String,
    pub updated: String,
}

impl<'r> FromRow<'r, AnyRow> for Collection {
    fn from_row(row: &'r AnyRow) -> Result<Self, Error> {
        // 1. Extract standard text fields
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let created: String = row.try_get("created")?;
        let updated: String = row.try_get("updated")?;

        // 2. Extract and manually decode the JSON fields from their text columns
        let fields_raw: String = row.try_get("fields")?;
        let fields: Vec<Column> =
            serde_json::from_str(&fields_raw).map_err(|e| Error::Decode(Box::new(e)))?;

        let indexes_raw: String = row.try_get("indexes")?;
        let indexes: Vec<Column> =
            serde_json::from_str(&indexes_raw).map_err(|e| Error::Decode(Box::new(e)))?;

        // 3. Construct and return the fully populated struct
        Ok(Collection {
            id,
            name,
            fields,
            indexes,
            created,
            updated,
        })
    }
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
    Url,
    Datetime,
    AutoDatetime(String),
    File,
    Relation(String),
    Select,
    Json,
    GeoPoint,
}

impl DataTypes {
    pub fn to_db_type(&self) -> String {
        match self {
            Self::PlainText
            | Self::RichText
            | Self::Email
            | Self::Url
            | Self::File
            | Self::Datetime
            | Self::Select
            | Self::Json
            | Self::GeoPoint => "TEXT".to_owned(),
            DataTypes::Number => "INTEGER".to_owned(),
            DataTypes::Bool => "INTEGER CHECK (is_active IN (0, 1))".to_owned(),
            DataTypes::AutoDatetime(_) => "TEXT".to_owned(),
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
