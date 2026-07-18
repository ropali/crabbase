use serde_json::Value;
use sqlx::{Pool, Postgres, Row};
use tracing::info;

use crate::repositories::collections::CollectionRepository;
use crabbase_core::{
    errors::RepositoryError,
    models::{Collection, CreateRecordRequest, Record, RecordListResponse, UpdateRecordRequest},
    rules::{
        compiler::{RulesSqlCompiler, SqlContext},
        parser::{RuleParser, tokenize},
    },
    utils::string_utils::{quote_ident, random_str},
};

use bcrypt;

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
        let mut count_base_query = format!("SELECT COUNT(id) FROM {}", collection);
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

                    count_base_query.push_str(" WHERE ");
                    count_base_query.push_str(&sql_clause);
                    bindings = compiler.bindings;
                }
            }
        }

        let limit_idx = bindings.len() + 1;
        let offset_idx = bindings.len() + 2;
        base_query.push_str(&format!(" LIMIT ${limit_idx} OFFSET ${offset_idx}"));

        let offset = (page - 1) * per_page;

        let mut query = sqlx::query(&base_query);
        let mut count_query = sqlx::query_scalar(&count_base_query);

        for bind_val in bindings {
            query = query.bind(bind_val.clone());
            count_query = count_query.bind(bind_val);
        }

        let query = query.bind(per_page as i64).bind(offset as i64);

        let result = query.fetch_all(&self.db).await?;

        let total_count: i64 = count_query.fetch_one(&self.db).await?;
        let items = result
            .iter()
            .filter_map(|r| Record::from_row(r).ok())
            .collect::<Vec<Record>>();

        Ok(RecordListResponse {
            items,
            total: total_count as u64,
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

        let id_uuid = uuid::Uuid::parse_str(id).ok();

        let sql = format!("SELECT * FROM {collection} WHERE id = $1");
        let query = sqlx::query(&sql);
        let query = if let Some(uuid) = id_uuid {
            query.bind(uuid)
        } else {
            query.bind(id.to_string())
        };

        let row = query.fetch_one(&self.db).await?;

        Ok(Record::from_row(&row)?)
    }

    pub async fn create_record(
        &self,
        collection: String,
        mut body: CreateRecordRequest,
    ) -> Result<Record, RepositoryError> {
        let obj = &mut body.data;

        if obj.is_empty() {
            return Err(RepositoryError::NotFound("Empty Input".to_string()));
        }

        let col_repo = CollectionRepository::new(self.db.clone());
        let exist = col_repo.exists(&collection).await;

        if !exist {
            return Err(RepositoryError::NotFound(
                "Collection does not exist".to_string(),
            ));
        }

        let col = col_repo.get_by_name(&collection).await?;

        if col.collection_type.eq_ignore_ascii_case("auth") {
            if let Some(serde_json::Value::String(plain_pw)) = obj.get("password") {
                let hashed_pw = bcrypt::hash(plain_pw, bcrypt::DEFAULT_COST).map_err(|e| {
                    RepositoryError::OtherError(format!("Failed to hash password: {e}"))
                })?;

                obj.insert("password".to_string(), serde_json::Value::String(hashed_pw));
            }

            // Normalize "tokenKey" payload field to "token_key"
            if let Some(val) = obj.remove("tokenKey") {
                obj.insert("token_key".to_string(), val);
            }

            // IF token_key value is not present then generate one
            let is_missing_or_null = obj.get("token_key").map_or(true, |v| v.is_null());

            if is_missing_or_null {
                obj.insert(
                    "token_key".to_string(),
                    serde_json::Value::String(random_str(None)),
                );
            }
        }

        let columns: Vec<&String> = obj.keys().collect();

        let values: Vec<&Value> = obj.values().collect();

        let quoted_table = quote_ident(&collection);
        let mut query_builder =
            sqlx::QueryBuilder::<Postgres>::new(format!("INSERT INTO {} (", quoted_table));

        // Add columns with proper sepration
        let mut separated = query_builder.separated(",");

        for col in &columns {
            separated.push(quote_ident(col));
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
            Ok(row) => {
                let id: String = if let Ok(id_str) = row.try_get::<String, _>("id") {
                    id_str
                } else {
                    row.try_get::<uuid::Uuid, _>("id")?.to_string()
                };
                Ok(Record {
                    id,
                    data: body.data,
                    created: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created")?,
                    updated: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated")?,
                })
            }
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
        mut payload: UpdateRecordRequest,
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

        let col = col_repo.get_by_name(&collection).await?;

        // Validate record existence and return NotFound before attempting update.
        let existing_record = self.get_record(collection, id).await?;

        if col.collection_type.eq_ignore_ascii_case("auth") {
            if let Some(serde_json::Value::String(plain_pw)) = payload.data.get("password") {
                let existing_pw_hash = existing_record
                    .data
                    .get("password")
                    .and_then(|v| v.as_str());

                if existing_pw_hash != Some(plain_pw) {
                    let hashed_pw = bcrypt::hash(plain_pw, bcrypt::DEFAULT_COST).map_err(|e| {
                        RepositoryError::OtherError(format!("Failed to hash password: {e}"))
                    })?;

                    payload
                        .data
                        .insert("password".to_string(), serde_json::Value::String(hashed_pw));
                }
            }
        }

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

        let id_uuid = uuid::Uuid::parse_str(id).ok();

        query_builder.push(" WHERE id = ");
        if let Some(uuid) = id_uuid {
            query_builder.push_bind(uuid);
        } else {
            query_builder.push_bind(id.to_string());
        }

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

        let id_uuid = uuid::Uuid::parse_str(id).ok();

        let sql = format!("DELETE FROM {collection} WHERE id = $1");
        let query = sqlx::query(&sql);
        let query = if let Some(uuid) = id_uuid {
            query.bind(uuid)
        } else {
            query.bind(id.to_string())
        };

        query.execute(&self.db).await?;

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
                ..Default::default()
            },
            Column {
                name: "views".into(),
                data_type: DataTypes::Number,
                index: false,
                related_to: None,
                ..Default::default()
            },
        ];
        let create_col = CreateCollectionRequest {
            name: "articles".into(),
            columns: columns.clone(),
            collection_type: None,
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
            ..Default::default()
        }];
        let create_col = CreateCollectionRequest {
            name: "items".into(),
            columns: columns.clone(),
            collection_type: None,
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
            ..Default::default()
        }];
        let create_col = CreateCollectionRequest {
            name: "items".into(),
            columns: columns.clone(),
            collection_type: None,
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
    async fn test_update_record_password_hashing() {
        let pool = setup_pool("rec_update_pw_hash").await;
        let col_repo = CollectionRepository::new(pool.clone());
        let columns = vec![
            Column {
                name: "email".into(),
                data_type: DataTypes::Email,
                index: true,
                ..Default::default()
            },
            Column {
                name: "password".into(),
                data_type: DataTypes::PlainText,
                ..Default::default()
            },
            Column {
                name: "token_key".into(),
                data_type: DataTypes::PlainText,
                ..Default::default()
            },
        ];
        let create_col = CreateCollectionRequest {
            name: "users".into(),
            columns,
            collection_type: Some("auth".to_string()),
        };
        col_repo.create(create_col).await.unwrap();

        let repo = RecordsRepository::new(pool.clone());
        let mut data = Map::new();
        data.insert(
            "email".to_string(),
            Value::String("test@crabbase.io".to_string()),
        );
        data.insert(
            "password".to_string(),
            Value::String("mysecretpw".to_string()),
        );
        let create_req = CreateRecordRequest { data };
        let created = repo
            .create_record("users".to_string(), create_req)
            .await
            .unwrap();

        let first_pw_hash = created
            .data
            .get("password")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(first_pw_hash, "mysecretpw");
        assert!(bcrypt::verify("mysecretpw", &first_pw_hash).unwrap());

        // Now update the password
        let mut upd_map = Map::new();
        upd_map.insert(
            "password".to_string(),
            Value::String("newsecretpw".to_string()),
        );
        let upd = UpdateRecordRequest { data: upd_map };
        let updated = repo
            .update_record("users", &created.id.to_string(), upd)
            .await
            .unwrap();

        let second_pw_hash = updated
            .data
            .get("password")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(second_pw_hash, "newsecretpw");
        assert_ne!(second_pw_hash, first_pw_hash);
        assert!(bcrypt::verify("newsecretpw", &second_pw_hash).unwrap());

        // Now perform an update with the same hash
        let mut upd_same_map = Map::new();
        upd_same_map.insert(
            "password".to_string(),
            Value::String(second_pw_hash.clone()),
        );
        let upd_same = UpdateRecordRequest { data: upd_same_map };
        let updated_same = repo
            .update_record("users", &created.id.to_string(), upd_same)
            .await
            .unwrap();

        let third_pw_hash = updated_same
            .data
            .get("password")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(third_pw_hash, second_pw_hash);
        assert!(bcrypt::verify("newsecretpw", &third_pw_hash).unwrap());
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
                ..Default::default()
            },
            Column {
                name: "views".into(),
                data_type: DataTypes::Number,
                index: false,
                related_to: None,
                ..Default::default()
            },
        ];
        let create_col = CreateCollectionRequest {
            name: "blogs".into(),
            columns: columns.clone(),
            collection_type: None,
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
            ..Default::default()
        }];
        let create_col = CreateCollectionRequest {
            name: "trash".into(),
            columns: columns.clone(),
            collection_type: None,
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
