use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use uuid::Uuid;

use crate::api::models::{Record, RecordListResponse};

#[derive(Debug, Clone)]
pub struct CollectionStore {
    collections: Arc<RwLock<HashMap<String, Vec<Record>>>>,
}

impl CollectionStore {
    pub fn new() -> Self {
        Self {
            collections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn collection_exists(&self, name: &str) -> bool {
        self.collections.read().unwrap().contains_key(name)
    }

    pub fn create_collection(&self, name: String) {
        self.collections
            .write()
            .unwrap()
            .insert(name, Vec::new());
    }

    pub fn list_records(
        &self,
        collection: &str,
        page: u64,
        per_page: u64,
    ) -> Option<RecordListResponse> {
        let collections = self.collections.read().unwrap();
        let records = collections.get(collection)?;
        let total = records.len() as u64;

        let start = ((page - 1) * per_page) as usize;
        let end = (start + per_page as usize).min(records.len());
        let items = if start < records.len() {
            records[start..end].to_vec()
        } else {
            Vec::new()
        };

        Some(RecordListResponse {
            items,
            total,
            page,
            per_page,
        })
    }

    pub fn get_record(&self, collection: &str, id: &str) -> Option<Record> {
        let collections = self.collections.read().unwrap();
        let records = collections.get(collection)?;
        records.iter().find(|r| r.id == id).cloned()
    }

    pub fn create_record(&self, collection: &str, data: serde_json::Value) -> Option<Record> {
        if !self.collection_exists(collection) {
            return None;
        }

        let now = Utc::now().to_rfc3339();
        let record = Record {
            id: Uuid::new_v4().to_string(),
            data,
            created: now.clone(),
            updated: now,
        };

        self.collections
            .write()
            .unwrap()
            .get_mut(collection)
            .unwrap()
            .push(record.clone());

        Some(record)
    }

    pub fn update_record(
        &self,
        collection: &str,
        id: &str,
        data: serde_json::Value,
    ) -> Option<Record> {
        let mut collections = self.collections.write().unwrap();
        let records = collections.get_mut(collection)?;
        let record = records.iter_mut().find(|r| r.id == id)?;
        record.data = data;
        record.updated = Utc::now().to_rfc3339();
        Some(record.clone())
    }

    pub fn delete_record(&self, collection: &str, id: &str) -> bool {
        let mut collections = self.collections.write().unwrap();
        let Some(records) = collections.get_mut(collection) else {
            return false;
        };
        let len_before = records.len();
        records.retain(|r| r.id != id);
        records.len() < len_before
    }
}
