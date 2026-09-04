use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use serde_json::{Number, Value};
use sqlx::postgres::PgRow;
use sqlx::{Column as _, Row};
use uuid::Uuid;

pub type RecordData = serde_json::Map<String, Value>;
pub type OptionalData = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: RecordData,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Record {
    pub fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let mut data = RecordData::new();

        let id: String = if let Ok(id_str) = row.try_get::<String, _>("id") {
            id_str
        } else if let Ok(id_uuid) = row.try_get::<Uuid, _>("id") {
            id_uuid.to_string()
        } else {
            row.try_get::<Uuid, _>("id")?.to_string()
        };

        for c in row.columns() {
            let col = c.name();

            if matches!(col, "id" | "created" | "updated") {
                continue;
            }
            let value = Self::column_to_value(row, col);

            data.insert(col.to_string(), value);
        }

        Ok(Record {
            id,
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

        if let Ok(v) = row.try_get::<Uuid, _>(col_name) {
            return Value::String(v.to_string());
        }

        if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(col_name) {
            return Value::String(v.to_rfc3339());
        }

        if let Ok(v) = row.try_get::<serde_json::Value, _>(col_name) {
            return v;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub system: bool,

    #[serde[rename = "collection_type"]]
    pub collection_type: String,
    pub fields: Vec<Column>,
    pub indexes: Vec<Column>,
    // Rules
    pub list_rule: Option<String>,
    pub view_rule: Option<String>,
    pub create_rule: Option<String>,
    pub update_rule: Option<String>,
    pub delete_rule: Option<String>,
    pub options: CollectionOptions,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Collection {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let system_int: i32 = row.try_get("system")?;
        let fields: sqlx::types::Json<Vec<Column>> = row.try_get("fields")?;
        let indexes: sqlx::types::Json<Vec<Column>> = row.try_get("indexes")?;
        let options: sqlx::types::Json<CollectionOptions> = row.try_get("options")?;

        Ok(Collection {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            system: system_int != 0,
            fields: fields.0,
            indexes: indexes.0,
            list_rule: row.try_get("list_rule")?,
            view_rule: row.try_get("view_rule")?,
            create_rule: row.try_get("create_rule")?,
            update_rule: row.try_get("update_rule")?,
            delete_rule: row.try_get("delete_rule")?,
            options: options.0,
            created: row.try_get("created")?,
            updated: row.try_get("updated")?,
            collection_type: row.try_get("type")?,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DataTypes {
    #[default]
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
            | Self::Select
            | Self::Json
            | Self::GeoPoint => "TEXT".to_owned(), // TODO: Handle the Geo point data type properly
            Self::Datetime | Self::AutoDatetime(_) => "TIMESTAMPTZ".to_owned(),
            DataTypes::Number => "BIGINT".to_owned(),
            DataTypes::Bool => "BOOLEAN".to_owned(),
            DataTypes::Relation => "UUID".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default, PartialEq)]
pub struct Column {
    pub name: String,
    #[serde(deserialize_with = "deserialize_data_type", alias = "type")]
    pub data_type: DataTypes,
    #[serde(default)]
    pub index: bool,

    #[serde(default)]
    pub hidden: bool,

    #[serde(default)]
    pub required: bool,

    #[serde(default)]
    pub min: Option<usize>,

    #[serde(default)]
    pub max: Option<usize>,

    #[serde(default)]
    pub pattern: Option<String>,

    #[serde(default)]
    pub related_to: Option<String>,
}

impl Column {
    pub fn to_sql_definition(&self) -> String {
        match &self.data_type {
            DataTypes::Relation => {
                let target = self.related_to.as_deref().unwrap_or("unknown");
                format!("\"{}\" UUID REFERENCES \"{}\"(\"id\")", self.name, target)
            }
            DataTypes::AutoDatetime(action) if action == "now" => {
                format!("\"{}\" TIMESTAMPTZ DEFAULT now()", self.name)
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
            "RICHTEXT" | "EDITOR" => Ok(DataTypes::RichText),
            "INTEGER" | "INT" | "NUMBER" => Ok(DataTypes::Number),
            "BOOLEAN" | "BOOL" => Ok(DataTypes::Bool),
            "DATE" | "DATETIME" => Ok(DataTypes::Datetime),
            "AUTODATE" | "AUTODATETIME" => Ok(DataTypes::AutoDatetime("now".to_string())),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── DataTypes::to_db_type ─────────────────────────────────────────────────

    #[test]
    fn test_data_type_to_db_type_text_variants() {
        assert_eq!(DataTypes::PlainText.to_db_type(), "TEXT");
        assert_eq!(DataTypes::RichText.to_db_type(), "TEXT");
        assert_eq!(DataTypes::Email.to_db_type(), "TEXT");
        assert_eq!(DataTypes::Url.to_db_type(), "TEXT");
        assert_eq!(DataTypes::File.to_db_type(), "TEXT");
        assert_eq!(DataTypes::Select.to_db_type(), "TEXT");
        assert_eq!(DataTypes::Json.to_db_type(), "TEXT");
        assert_eq!(DataTypes::GeoPoint.to_db_type(), "TEXT");
    }

    #[test]
    fn test_data_type_to_db_type_number() {
        assert_eq!(DataTypes::Number.to_db_type(), "BIGINT");
    }

    #[test]
    fn test_data_type_to_db_type_bool() {
        assert_eq!(DataTypes::Bool.to_db_type(), "BOOLEAN");
    }

    #[test]
    fn test_data_type_to_db_type_datetime() {
        assert_eq!(DataTypes::Datetime.to_db_type(), "TIMESTAMPTZ");
        assert_eq!(
            DataTypes::AutoDatetime("now".to_string()).to_db_type(),
            "TIMESTAMPTZ"
        );
    }

    #[test]
    fn test_data_type_to_db_type_relation() {
        assert_eq!(DataTypes::Relation.to_db_type(), "UUID");
    }

    // ── Column::to_sql_definition ─────────────────────────────────────────────

    #[test]
    fn test_column_sql_definition_plain_text() {
        let col = Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            ..Default::default()
        };
        assert_eq!(col.to_sql_definition(), "\"title\" TEXT");
    }

    #[test]
    fn test_column_sql_definition_number() {
        let col = Column {
            name: "count".to_string(),
            data_type: DataTypes::Number,
            ..Default::default()
        };
        assert_eq!(col.to_sql_definition(), "\"count\" BIGINT");
    }

    #[test]
    fn test_column_sql_definition_bool() {
        let col = Column {
            name: "active".to_string(),
            data_type: DataTypes::Bool,
            ..Default::default()
        };
        assert_eq!(col.to_sql_definition(), "\"active\" BOOLEAN");
    }

    #[test]
    fn test_column_sql_definition_relation_with_target() {
        let col = Column {
            name: "user_id".to_string(),
            data_type: DataTypes::Relation,
            related_to: Some("users".to_string()),
            ..Default::default()
        };
        let sql = col.to_sql_definition();
        assert!(sql.contains("\"user_id\" UUID REFERENCES"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_column_sql_definition_relation_without_target_uses_unknown() {
        let col = Column {
            name: "ref_id".to_string(),
            data_type: DataTypes::Relation,
            related_to: None,
            ..Default::default()
        };
        let sql = col.to_sql_definition();
        assert!(sql.contains("unknown"));
    }

    #[test]
    fn test_column_sql_definition_auto_datetime_now() {
        let col = Column {
            name: "created_at".to_string(),
            data_type: DataTypes::AutoDatetime("now".to_string()),
            ..Default::default()
        };
        let sql = col.to_sql_definition();
        assert!(sql.contains("TIMESTAMPTZ DEFAULT now()"));
    }

    #[test]
    fn test_column_sql_definition_auto_datetime_non_now_falls_through() {
        // Non-"now" action should fall through to the plain type mapping (TIMESTAMPTZ)
        let col = Column {
            name: "ts".to_string(),
            data_type: DataTypes::AutoDatetime("custom".to_string()),
            ..Default::default()
        };
        let sql = col.to_sql_definition();
        assert_eq!(sql, "\"ts\" TIMESTAMPTZ");
    }

    // ── Deserialization backward-compat ────────────────────────────────────────

    #[test]
    fn test_deserialize_autodate_string() {
        let json_data = r#"{"name": "created", "type": "autodate"}"#;
        let col: Column = serde_json::from_str(json_data).unwrap();
        assert_eq!(col.data_type, DataTypes::AutoDatetime("now".to_string()));

        let json_data_caps = r#"{"name": "created", "type": "AUTODATE"}"#;
        let col_caps: Column = serde_json::from_str(json_data_caps).unwrap();
        assert_eq!(
            col_caps.data_type,
            DataTypes::AutoDatetime("now".to_string())
        );

        let json_data_autodatetime = r#"{"name": "created", "type": "autodatetime"}"#;
        let col_adt: Column = serde_json::from_str(json_data_autodatetime).unwrap();
        assert_eq!(
            col_adt.data_type,
            DataTypes::AutoDatetime("now".to_string())
        );
    }

    #[test]
    fn test_deserialize_all_legacy_type_strings() {
        let cases = vec![
            (r#"{"name":"c","type":"TEXT"}"#, DataTypes::PlainText),
            (r#"{"name":"c","type":"PLAINTEXT"}"#, DataTypes::PlainText),
            (r#"{"name":"c","type":"RICHTEXT"}"#, DataTypes::RichText),
            (r#"{"name":"c","type":"EDITOR"}"#, DataTypes::RichText),
            (r#"{"name":"c","type":"INTEGER"}"#, DataTypes::Number),
            (r#"{"name":"c","type":"NUMBER"}"#, DataTypes::Number),
            (r#"{"name":"c","type":"BOOLEAN"}"#, DataTypes::Bool),
            (r#"{"name":"c","type":"BOOL"}"#, DataTypes::Bool),
            (r#"{"name":"c","type":"DATE"}"#, DataTypes::Datetime),
            (r#"{"name":"c","type":"DATETIME"}"#, DataTypes::Datetime),
            (r#"{"name":"c","type":"EMAIL"}"#, DataTypes::Email),
            (r#"{"name":"c","type":"URL"}"#, DataTypes::Url),
            (r#"{"name":"c","type":"FILE"}"#, DataTypes::File),
            (r#"{"name":"c","type":"RELATION"}"#, DataTypes::Relation),
            (r#"{"name":"c","type":"SELECT"}"#, DataTypes::Select),
            (r#"{"name":"c","type":"JSON"}"#, DataTypes::Json),
            (r#"{"name":"c","type":"GEOPOINT"}"#, DataTypes::GeoPoint),
        ];

        for (json, expected) in cases {
            let col: Column = serde_json::from_str(json).unwrap();
            assert_eq!(col.data_type, expected, "failed for json: {json}");
        }
    }

    #[test]
    fn test_deserialize_invalid_type_string_errors() {
        let json = r#"{"name": "c", "type": "INVALID_TYPE"}"#;
        let result: Result<Column, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    #[serde(default)]
    pub collection_type: Option<String>,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub columns: Option<Vec<Column>>,
    #[serde(default)]
    pub list_rule: Option<String>,
    #[serde(default)]
    pub view_rule: Option<String>,
    #[serde(default)]
    pub create_rule: Option<String>,
    #[serde(default)]
    pub update_rule: Option<String>,
    #[serde(default)]
    pub delete_rule: Option<String>,
}
