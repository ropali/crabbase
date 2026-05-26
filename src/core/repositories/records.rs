use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};
use tracing::{error, info};

use crate::{
    api::models::{CreateRecordRequest, Record, RecordListResponse, UpdateRecordRequest},
    core::{
        errors::RepositoryError, repositories::collections::CollectionRepository,
        utils::string_utils::quote_ident,
    },
};

#[derive(Debug, Clone)]
pub struct RecordsRepository {
    db: Pool<Sqlite>,
}

impl RecordsRepository {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn list(
        &self,
        collection: &str,
        page: u64,
        per_page: u64,
    ) -> Result<RecordListResponse, RepositoryError> {
        let q = format!("SELECT * FROM {} LIMIT ? OFFSET ?", collection);

        let offset = (page - 1) * per_page;
        let result = sqlx::query(&q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await?;

        let items = result
            .iter()
            .filter_map(|r| Record::from_row(r).ok())
            .collect::<Vec<Record>>();

        let total = items.len();

        Ok(RecordListResponse {
            items,
            total: total as u64,
            page,
            per_page,
        })
    }

    pub async fn get_record(&self, collection: &str, id: &str) -> Result<Record, RepositoryError> {
        let col_repo = CollectionRepository::new(self.db.clone());

        let exist = col_repo.exists(collection).await;

        if !exist {
            return Err(RepositoryError::NotFound(collection.to_string()));
        }

        let row = sqlx::query(&format!("SELECT * FROM {collection} WHERE id = $1"))
            .bind(id)
            .fetch_one(&self.db)
            .await?;

        Ok(Record::from_row(&row)?)
    }

    pub async fn create_record(
        &self,
        collection: String,
        body: CreateRecordRequest,
    ) -> Result<Record, RepositoryError> {
        let obj = &body.data;

        if obj.is_empty() {
            return Err(RepositoryError::NotFound("Empty Input".to_string()));
        }

        let exist = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name=$1")
            .bind(&collection)
            .execute(&self.db)
            .await
            .is_ok();

        info!("TABLE EXIST {}: {}", collection, exist);

        if !exist {
            return Err(RepositoryError::NotFound(
                "Collection does not exist".to_string(),
            ));
        }

        let columns: Vec<&String> = obj.keys().collect();

        let values: Vec<&Value> = obj.values().collect();

        let quoted_table = quote_ident(&collection);
        let mut query_builder =
            sqlx::QueryBuilder::<Sqlite>::new(format!("INSERT INTO {} (", quoted_table));

        // Add columns with proper sepration
        let mut separated = query_builder.separated(",");

        for col in &columns {
            separated.push(col);
        }

        separated.push_unseparated(") VALUES (");

        // Add values as bound parameters; QueryBuilder writes `?` placeholders for SQLite.
        let mut separated_values = query_builder.separated(",");
        for v in values {
            match v {
                Value::String(s) => {
                    separated_values.push_bind(s.clone());
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        separated_values.push_bind(i);
                    } else if let Some(f) = n.as_f64() {
                        separated_values.push_bind(f);
                    } else {
                        separated_values.push_bind(n.to_string());
                    }
                }
                Value::Bool(b) => {
                    separated_values.push_bind(*b);
                }
                Value::Null => {
                    separated_values.push_bind(Option::<String>::None);
                }
                other => {
                    separated_values.push_bind(other.to_string());
                }
            }
        }
        separated_values.push_unseparated(")");

        let query = query_builder.build();

        match query.execute(&self.db).await {
            Ok(res) => {
                let row = sqlx::query(&format!(
                    "SELECT id, created, updated FROM {} WHERE id = ?;",
                    quote_ident(&collection)
                ))
                .bind(res.last_insert_rowid())
                .fetch_one(&self.db)
                .await?;

                Ok(Record {
                    id: row.try_get::<i64, _>("id")?,
                    data: body.data,
                    created: row.try_get::<String, _>("created")?,
                    updated: row.try_get::<String, _>("updated")?,
                })
            }
            Err(err) => {
                error!("Error: {}", err);
                Err(RepositoryError::QueryFailed(err.to_string()))
            }
        }
    }

    pub async fn update_record(
        &self,
        collection: &str,
        id: &str,
        payload: UpdateRecordRequest,
    ) -> Result<Record, RepositoryError> {
        let col_repo = CollectionRepository::new(self.db.clone());

        let exist = col_repo.exists(collection).await;

        if !exist {
            return Err(RepositoryError::NotFound(collection.to_string()));
        }

        if payload.data.is_empty() {
            return Err(RepositoryError::OtherError(
                "empty update payload".to_string(),
            ));
        }

        // Validate record existence and return NotFound before attempting update.
        let _ = self.get_record(collection, id).await?;

        let quoted_table = quote_ident(collection);
        let mut query_builder =
            sqlx::QueryBuilder::<Sqlite>::new(format!("UPDATE {} SET ", quoted_table));

        let mut first = true;
        for (k, v) in &payload.data {
            if !first {
                query_builder.push(", ");
            }
            first = false;

            query_builder.push(quote_ident(k)).push(" = ");
            match v {
                Value::String(s) => {
                    query_builder.push_bind(s.clone());
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.push_bind(i);
                    } else if let Some(f) = n.as_f64() {
                        query_builder.push_bind(f);
                    } else {
                        query_builder.push_bind(n.to_string());
                    }
                }
                Value::Bool(b) => {
                    query_builder.push_bind(*b);
                }
                Value::Null => {
                    query_builder.push_bind(Option::<String>::None);
                }
                other => {
                    query_builder.push_bind(other.to_string());
                }
            }
        }

        query_builder.push(", updated = strftime('%Y-%m-%d %H:%M:%fZ')");

        query_builder.push(" WHERE id = ").push_bind(id);

        info!("SQL: {}", query_builder.sql());

        let res = query_builder.build().execute(&self.db).await?;

        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(format!("record {id}")));
        }

        self.get_record(collection, id).await
    }

    pub async fn delete_record(&self, collection: &str, id: &str) -> Result<bool, RepositoryError> {
        let col_repo = CollectionRepository::new(self.db.clone());

        let exist = col_repo.exists(collection).await;

        if !exist {
            return Err(RepositoryError::NotFound(collection.to_string()));
        }

        sqlx::query(&format!("DELETE FROM {collection} WHERE id = $1"))
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(true)
    }
}
