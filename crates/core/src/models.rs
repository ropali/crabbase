use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use serde_json::{Number, Value};
use sqlx::postgres::PgRow;
use sqlx::{Column as _, Row};

pub type RecordData = serde_json::Map<String, Value>;
pub type OptionalData = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Record {
    pub id: i64,
    pub data: RecordData,
    pub created: String,
    pub updated: String,
}

impl Record {
    pub fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let mut data = RecordData::new();

        for c in row.columns() {
            let col = c.name();

            if matches!(col, "id" | "created" | "updated") {
                continue;
            }
            let value = Self::column_to_value(row, col);

            data.insert(col.to_string(), value);
        }

        Ok(Record {
            id: row.try_get("id")?,
            data,
            created: row.try_get("created")?,
            updated: row.try_get("updated")?,
        })
    }

    pub fn column_to_value(row: &PgRow, col_name: &str) -> Value {
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

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionOptions {
    #[serde(alias = "auth_token")]
    pub auth_token: Option<OptionalData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Collection {
    pub id: String,
    pub name: String,

    #[sqlx(json)]
    pub fields: Vec<Column>,

    #[sqlx(json)]
    pub indexes: Vec<Column>,

    // Rules
    pub list_rule: Option<String>,
    pub view_rule: Option<String>,
    pub create_rule: Option<String>,
    pub update_rule: Option<String>,
    pub delete_rule: Option<String>,

    #[sqlx(json)]
    pub options: CollectionOptions,
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
    Url,
    Datetime,
    AutoDatetime(String),
    File,
    Relation,
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
            DataTypes::Number => "BIGINT".to_owned(),
            DataTypes::Bool => "BOOLEAN".to_owned(),
            DataTypes::AutoDatetime(_) => "TEXT".to_owned(),
            DataTypes::Relation => "BIGINT".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Column {
    pub name: String,
    #[serde(deserialize_with = "deserialize_data_type", alias = "type")]
    pub data_type: DataTypes,
    #[serde(default)]
    pub index: bool,
    #[serde(default)]
    pub related_to: Option<String>,
}

impl Column {
    pub fn to_sql_definition(&self) -> String {
        match &self.data_type {
            DataTypes::Relation => {
                let target = self.related_to.as_deref().unwrap_or("unknown");
                format!("\"{}\" BIGINT REFERENCES \"{}\"(\"id\")", self.name, target)
            }
            _ => format!("\"{}\" {}", self.name, self.data_type.to_db_type()),
        }
    }
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

    // Backward compatibility: accept legacy/cased strings in stored JSON.
    if let Some(raw) = value.as_str() {
        return match raw.to_ascii_uppercase().as_str() {
            "TEXT" | "PLAINTEXT" => Ok(DataTypes::PlainText),
            "RICHTEXT" => Ok(DataTypes::RichText),
            "INTEGER" | "INT" | "NUMBER" => Ok(DataTypes::Number),
            "BOOLEAN" | "BOOL" => Ok(DataTypes::Bool),
            "DATE" | "DATETIME" => Ok(DataTypes::Datetime),
            "EMAIL" => Ok(DataTypes::Email),
            "URL" => Ok(DataTypes::Url),
            "FILE" => Ok(DataTypes::File),
            "RELATION" => Ok(DataTypes::Relation),
            "SELECT" => Ok(DataTypes::Select),
            "JSON" => Ok(DataTypes::Json),
            "GEOPOINT" => Ok(DataTypes::GeoPoint),
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
