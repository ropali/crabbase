use serde_json::Value;
use sqlx::{Pool, Sqlite};
use tracing::error;

use crate::{
    api::models::{CreateRecordRequest, Record, RecordListResponse},
    core::repositories::collections::RepositoryError,
};

#[derive(Debug, Clone)]
pub struct RecordsRepository {
    db: Pool<Sqlite>,
}

impl RecordsRepository {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn create(&self, name: String, body: CreateRecordRequest) {}

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

    pub async fn create_record(
        &self,
        collection: String,
        body: CreateRecordRequest,
    ) -> Result<Record, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct InsertedMeta {
            id: i64,
            created: String,
            updated: String,
        }

        let obj = body.data.as_object().ok_or(RepositoryError::InvalidInput(
            "Invalid input data".to_string(),
        ));

        let obj = obj.unwrap();

        if obj.is_empty() {
            return Err(RepositoryError::InvalidInput("Empty Input".to_string()));
        }

        let columns: Vec<&String> = obj.keys().collect();

        let values: Vec<&Value> = obj.values().collect();

        let quoted_table = quote_ident(&collection);
        let mut query_builder =
            sqlx::QueryBuilder::<Sqlite>::new(format!("INSERT INTO {} (", quoted_table));

        // Add columns with proper sepration
        let mut separated = query_builder.separated(",");

        for col in &columns {
            separated.push(quote_ident(col));
        }

        separated.push_unseparated(") VALUES (");

        // Add values as bound parameters; QueryBuilder writes `?` placeholders for SQLite.
        let mut separated_values = query_builder.separated(",");
        for v in values {
            separated_values.push_bind(v);
        }
        separated_values.push_unseparated(")");

        let query = query_builder.build();

        match query.execute(&self.db).await {
            Ok(res) => {
                let record = sqlx::query_as::<_, InsertedMeta>(&format!(
                    "SELECT id, created, updated FROM {} WHERE id = ?;",
                    quote_ident(&collection)
                ))
                .bind(res.last_insert_rowid())
                .fetch_one(&self.db)
                .await?;

                Ok(Record {
                    id: record.id,
                    data: body.data,
                    created: record.created,
                    updated: record.updated,
                })
            }
            Err(err) => {
                error!("Error: {}", err);
                Err(RepositoryError::Sqlx(err))
            }
        }
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

fn quote_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}
