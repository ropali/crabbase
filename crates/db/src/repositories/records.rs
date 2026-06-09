use serde_json::Value;
use sqlx::{Pool, Postgres, Row};
use tracing::info;

use crate::repositories::collections::CollectionRepository;
use crabbase_core::{
    errors::RepositoryError,
    models::{CreateRecordRequest, Record, RecordListResponse, UpdateRecordRequest},
    rules::{
        compiler::{RulesSqlCompiler, SqlContext},
        parser::{RuleParser, tokenize},
    },
    utils::string_utils::quote_ident,
};

#[derive(Debug, Clone)]
pub struct RecordsRepository {
    db: Pool<Postgres>,
}

impl RecordsRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    pub async fn list(
        &self,
        collection: &str,
        page: u64,
        per_page: u64,
        sql_context: SqlContext,
    ) -> Result<RecordListResponse, RepositoryError> {
        let col = CollectionRepository::new(self.db.clone())
            .get_by_name(collection)
            .await?;

        let mut base_query = format!("SELECT * FROM {}", collection);
        let mut bindings: Vec<String> = vec![];

        if let Some(rule) = &col.list_rule
            && !rule.trim().is_empty()
        {
            let tokens = tokenize(rule);
            let mut parser = RuleParser::new(tokens);

            if let Ok(ast) = parser.parse() {
                let mut compiler = RulesSqlCompiler::new(sql_context);

                if let Ok(sql_clause) = compiler.compile(&ast) {
                    base_query.push_str(" WHERE ");
                    base_query.push_str(&sql_clause);
                    bindings = compiler.bindings;
                }
            }
        }

        let limit_idx = bindings.len() + 1;
        let offset_idx = bindings.len() + 2;
        base_query.push_str(&format!(" LIMIT ${limit_idx} OFFSET ${offset_idx}"));

        let offset = (page - 1) * per_page;

        let mut query = sqlx::query(&base_query);

        for bind_val in bindings {
            query = query.bind(bind_val);
        }

        let query = query.bind(per_page as i64).bind(offset as i64);

        let result = query.fetch_all(&self.db).await?;

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

        let id_uuid = uuid::Uuid::parse_str(id)
            .map_err(|e| RepositoryError::OtherError(format!("invalid uuid id: {e}")))?;

        let row = sqlx::query(&format!("SELECT * FROM {collection} WHERE id = $1"))
            .bind(id_uuid)
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

        let col_repo = CollectionRepository::new(self.db.clone());
        let exist = col_repo.exists(&collection).await;

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
            sqlx::QueryBuilder::<Postgres>::new(format!("INSERT INTO {} (", quoted_table));

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

        query_builder.push(" RETURNING id, created, updated");
        let query = query_builder.build();

        match query.fetch_one(&self.db).await {
            Ok(row) => Ok(Record {
                id: row.try_get::<uuid::Uuid, _>("id")?,
                data: body.data,
                created: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created")?,
                updated: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated")?,
            }),
            Err(err) => Err(RepositoryError::QueryFailed {
                message: "failed to create the record".to_string(),
                source: Some(err.to_string()),
            }),
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
            sqlx::QueryBuilder::<Postgres>::new(format!("UPDATE {} SET ", quoted_table));

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

        query_builder.push(", updated = now()");

        let id_uuid = uuid::Uuid::parse_str(id)
            .map_err(|e| RepositoryError::OtherError(format!("invalid uuid id: {e}")))?;

        query_builder.push(" WHERE id = ").push_bind(id_uuid);

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

        let id_uuid = uuid::Uuid::parse_str(id)
            .map_err(|e| RepositoryError::OtherError(format!("invalid uuid id: {e}")))?;

        sqlx::query(&format!("DELETE FROM {collection} WHERE id = $1"))
            .bind(id_uuid)
            .execute(&self.db)
            .await?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::collections::CollectionRepository;
    use crabbase_core::models::{
        Column, CreateCollectionRequest, CreateRecordRequest, DataTypes, UpdateRecordRequest,
    };
    use serde_json::{Value, map::Map};
    use sqlx::postgres::PgPoolOptions;

    async fn setup_pool(schema: &str) -> sqlx::Pool<sqlx::Postgres> {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

        let init_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .unwrap();

        let schema_ident = format!("\"{}\"", schema);
        let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE;", schema_ident))
            .execute(&init_pool)
            .await;

        sqlx::query(&format!("CREATE SCHEMA {};", schema_ident))
            .execute(&init_pool)
            .await
            .unwrap();

        init_pool.close().await;

        let mut options: sqlx::postgres::PgConnectOptions = db_url.parse().unwrap();
        options = options.options([("search_path", schema)]);

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        // Remove seeded _collections row inserted by migrations to avoid deserialize errors in tests
        sqlx::query("DELETE FROM _collections;")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_record() {
        let pool = setup_pool("rec_create_record").await;
        let col_repo = CollectionRepository::new(pool.clone());
        let columns = vec![
            Column {
                name: "title".into(),
                data_type: DataTypes::PlainText,
                index: false,
                related_to: None,
            },
            Column {
                name: "views".into(),
                data_type: DataTypes::Number,
                index: false,
                related_to: None,
            },
        ];
        let create_col = CreateCollectionRequest {
            name: "articles".into(),
            columns: columns.clone(),
        };
        col_repo.create(create_col).await.unwrap();

        let repo = RecordsRepository::new(pool.clone());
        let mut data = Map::new();
        data.insert("title".to_string(), Value::String("hello".to_string()));
        data.insert("views".to_string(), Value::Number(1.into()));
        let create_req = CreateRecordRequest { data: data.clone() };
        let created = repo
            .create_record("articles".to_string(), create_req)
            .await
            .unwrap();
        assert_eq!(
            created.data.get("title").and_then(|v| v.as_str()),
            Some("hello")
        );
    }

    #[tokio::test]
    async fn test_get_record() {
        let pool = setup_pool("rec_get_record").await;
        let col_repo = CollectionRepository::new(pool.clone());
        let columns = vec![Column {
            name: "title".into(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];
        let create_col = CreateCollectionRequest {
            name: "items".into(),
            columns: columns.clone(),
        };
        col_repo.create(create_col).await.unwrap();

        let repo = RecordsRepository::new(pool.clone());
        let mut data = Map::new();
        data.insert("title".to_string(), Value::String("hello".to_string()));
        let create_req = CreateRecordRequest { data: data.clone() };
        let created = repo
            .create_record("items".to_string(), create_req)
            .await
            .unwrap();

        let got = repo
            .get_record("items", &created.id.to_string())
            .await
            .unwrap();
        assert_eq!(got.id, created.id);
    }

    #[tokio::test]
    async fn test_update_record() {
        let pool = setup_pool("rec_update_record").await;
        let col_repo = CollectionRepository::new(pool.clone());
        let columns = vec![Column {
            name: "title".into(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];
        let create_col = CreateCollectionRequest {
            name: "items".into(),
            columns: columns.clone(),
        };
        col_repo.create(create_col).await.unwrap();

        let repo = RecordsRepository::new(pool.clone());
        let mut data = Map::new();
        data.insert("title".to_string(), Value::String("hello".to_string()));
        let create_req = CreateRecordRequest { data: data.clone() };
        let created = repo
            .create_record("items".to_string(), create_req)
            .await
            .unwrap();

        let mut upd_map = Map::new();
        upd_map.insert("title".to_string(), Value::String("updated".to_string()));
        let upd = UpdateRecordRequest { data: upd_map };
        let updated = repo
            .update_record("items", &created.id.to_string(), upd)
            .await
            .unwrap();
        assert_eq!(
            updated.data.get("title").and_then(|v| v.as_str()),
            Some("updated")
        );
    }

    #[tokio::test]
    async fn test_list_records() {
        let pool = setup_pool("rec_list_records").await;
        let col_repo = CollectionRepository::new(pool.clone());
        let columns = vec![
            Column {
                name: "title".into(),
                data_type: DataTypes::PlainText,
                index: false,
                related_to: None,
            },
            Column {
                name: "views".into(),
                data_type: DataTypes::Number,
                index: false,
                related_to: None,
            },
        ];
        let create_col = CreateCollectionRequest {
            name: "blogs".into(),
            columns: columns.clone(),
        };
        col_repo.create(create_col).await.unwrap();

        let repo = RecordsRepository::new(pool.clone());
        for i in 0..3 {
            let mut data = Map::new();
            data.insert("title".to_string(), Value::String(format!("t{}", i)));
            data.insert("views".to_string(), Value::Number((i as i64).into()));
            let create_req = CreateRecordRequest { data };
            repo.create_record("blogs".to_string(), create_req)
                .await
                .unwrap();
        }

        let listed = repo
            .list(
                "blogs",
                1,
                10,
                crabbase_core::rules::compiler::SqlContext {
                    auth: None,
                    query: std::collections::HashMap::new(),
                },
            )
            .await
            .unwrap();
        assert!(listed.items.len() >= 3);
    }

    #[tokio::test]
    async fn test_delete_record() {
        let pool = setup_pool("rec_delete_record").await;
        let col_repo = CollectionRepository::new(pool.clone());
        let columns = vec![Column {
            name: "title".into(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];
        let create_col = CreateCollectionRequest {
            name: "trash".into(),
            columns: columns.clone(),
        };
        col_repo.create(create_col).await.unwrap();

        let repo = RecordsRepository::new(pool.clone());
        let mut data = Map::new();
        data.insert("title".to_string(), Value::String("bye".to_string()));
        let create_req = CreateRecordRequest { data };
        let created = repo
            .create_record("trash".to_string(), create_req)
            .await
            .unwrap();

        let deleted = repo
            .delete_record("trash", &created.id.to_string())
            .await
            .unwrap();
        assert!(deleted);

        let res = repo.get_record("trash", &created.id.to_string()).await;
        assert!(matches!(
            res,
            Err(crabbase_core::errors::RepositoryError::NotFound(_))
        ));
    }
}
