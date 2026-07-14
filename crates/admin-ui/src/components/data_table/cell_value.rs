use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl CellValue {
    // Convert a raw serde_json::Value into a CellValue
    pub fn from_json(value: &Value) -> Self {
        match value {
            Value::String(s) => CellValue::Text(s.clone()),
            Value::Number(n) => CellValue::Number(n.as_f64().unwrap_or(0.0)),
            Value::Bool(b) => CellValue::Bool(*b),
            Value::Null => CellValue::Null,
            other => CellValue::Text(other.to_string()),
        }
    }

    // Display representation - used by default column renderer.
    pub fn display(&self) -> String {
        match self {
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{:.2}", n)
                }
            }
            CellValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            CellValue::Null => "null".to_string(),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => n.to_string(),
            CellValue::Bool(b) => b.to_string(),
            CellValue::Null => "N/A".to_string(),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, CellValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CellValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}
