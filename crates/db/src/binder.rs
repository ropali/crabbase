use crabbase_core::models::{DataTypes, TypedValue};

pub fn bind_typed_value(
    separated: &mut sqlx::query_builder::Separated<'_, '_, sqlx::Postgres, &'static str>,
    val: &TypedValue,
) {
    match val {
        TypedValue::Text(s) => {
            separated.push_bind(s.clone());
        }
        TypedValue::NumberInt(i) => {
            separated.push_bind(*i);
        }
        TypedValue::NumberFloat(f) => {
            separated.push_bind(*f);
        }
        TypedValue::Bool(b) => {
            separated.push_bind(*b);
        }
        TypedValue::Datetime(dt) => {
            separated.push_bind(*dt); // Sends OID 1184 (TIMESTAMPTZ)
        }
        TypedValue::Uuid(u) => {
            separated.push_bind(*u); // Sends OID 2950 (UUID)
        }
        TypedValue::Json(j) => {
            let s = match j {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            separated.push_bind(s);
        }
        TypedValue::Null(dtype) => match dtype {
            DataTypes::Relation => {
                separated.push_bind(Option::<uuid::Uuid>::None);
            }
            DataTypes::Datetime | DataTypes::AutoDatetime(_) => {
                separated.push_bind(Option::<chrono::DateTime<chrono::Utc>>::None);
            }
            DataTypes::Bool => {
                separated.push_bind(Option::<bool>::None);
            }
            DataTypes::Number => {
                separated.push_bind(Option::<i64>::None);
            }
            _ => {
                separated.push_bind(Option::<String>::None);
            }
        },
    }
}

pub fn bind_typed_value_to_builder(
    builder: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>,
    val: &TypedValue,
) {
    match val {
        TypedValue::Text(s) => {
            builder.push_bind(s.clone());
        }
        TypedValue::NumberInt(i) => {
            builder.push_bind(*i);
        }
        TypedValue::NumberFloat(f) => {
            builder.push_bind(*f);
        }
        TypedValue::Bool(b) => {
            builder.push_bind(*b);
        }
        TypedValue::Datetime(dt) => {
            builder.push_bind(*dt);
        }
        TypedValue::Uuid(u) => {
            builder.push_bind(*u);
        }
        TypedValue::Json(j) => {
            let s = match j {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            builder.push_bind(s);
        }
        TypedValue::Null(dtype) => match dtype {
            DataTypes::Relation => {
                builder.push_bind(Option::<uuid::Uuid>::None);
            }
            DataTypes::Datetime | DataTypes::AutoDatetime(_) => {
                builder.push_bind(Option::<chrono::DateTime<chrono::Utc>>::None);
            }
            DataTypes::Bool => {
                builder.push_bind(Option::<bool>::None);
            }
            DataTypes::Number => {
                builder.push_bind(Option::<i64>::None);
            }
            _ => {
                builder.push_bind(Option::<String>::None);
            }
        },
    }
}
