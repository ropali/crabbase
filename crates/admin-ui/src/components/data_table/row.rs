use super::cell_value::CellValue;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

// A single table row with no fixed schema.
/// Fields are stored as an ordered map: field_name → CellValue.
/// BTreeMap keeps column order consistent (important for display).
#[derive(Debug, PartialEq, Clone, Default)]
pub struct DynamicRow {
    pub values: BTreeMap<String, CellValue>,
}

impl DynamicRow {
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: CellValue) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> &CellValue {
        self.values.get(key).unwrap_or(&CellValue::Null)
    }

    // Build a DynamicRow from serde_json object map
    pub fn from_json_map(map: &Map<String, Value>) -> Self {
        let mut row = DynamicRow::new();

        for (key, value) in map {
            row.insert(key.clone(), CellValue::from_json(value));
        }

        row
    }

    // Helper: parse JSON array of record objects into rowas
    pub fn vec_from_json_arr(array: &[Value]) -> Vec<Self> {
        array
            .iter()
            .filter_map(|v| v.as_object().map(DynamicRow::from_json_map))
            .collect()
    }
}
