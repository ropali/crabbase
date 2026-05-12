use sqlx::{Pool, Sqlite};

use crate::api::models::{Record, RecordListResponse};

#[derive(Debug, Clone)]
pub struct RecordsRepository {
    db: Pool<Sqlite>,
}

impl RecordsRepository {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn create(&self, name: String) {
        let query = r#"
             r#"INSERT INTO collections (name, description,
                owner_id) VALUES ($1, $2, $3)
            "#;

        let r = sqlx::query(query)
            .bind(name)
            .bind("Test")
            .bind(1)
            .execute(&self.db)
            .await;

        eprint!("Collection created {:?}", r)
    }

    pub async fn list(
        &self,
        collection: &str,
        page: u64,
        per_page: u64,
    ) -> Option<RecordListResponse> {
        let q = r#"SELECT id, collection_id, data, created_at, updated_at FROM records WHERE collection_id = ? LIMIT ? OFFSET ?"#;

        let offset = (page - 1) * per_page;
        let result = sqlx::query_as::<_, Record>(q)
            .bind(collection)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await;

        let items = match result {
            Ok(items) => items,
            Err(e) => {
                eprintln!("Error: can't fetch records {:?}", e);
                return Some(RecordListResponse {
                    items: vec![],
                    total: 0,
                    page,
                    per_page,
                });
            }
        };

        let total = items.len();

        return Some(RecordListResponse {
            items,
            total: total as u64,
            page,
            per_page,
        });
    }

    pub fn get_record(&self, collection: &str, id: &str) -> Option<Record> {
        todo!()
    }

    pub fn create_record(&self, collection: &str, data: serde_json::Value) -> Option<Record> {
        todo!()
    }

    pub fn update_record(
        &self,
        collection: &str,
        id: &str,
        data: serde_json::Value,
    ) -> Option<Record> {
        todo!()
    }

    pub fn delete_record(&self, collection: &str, id: &str) -> bool {
        todo!()
    }
}
