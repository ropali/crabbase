use std::collections::{HashMap, HashSet};

use serde_json::Value;
use sqlx::{Pool, Postgres, Row};
use tracing::Level;
use uuid::Uuid;

use crate::repositories::collections::CollectionRepository;
use crabbase_core::{
    errors::RepositoryError,
    models::{
        Collection, Column, CreateRecordRequest, DataTypes, PaginationParams, Record,
        RecordListResponse, UpdateRecordRequest,
    },
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

pub struct CompiledRule {
    pub sql_clause: String,
    pub bindings: Vec<String>,
}

impl RecordsRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    // Compiles a collection rule expression into a parameterized SQL WHERE clause.
    /// Returns:
    /// - `Ok(Some(clause))` if a rule expression is present and compiled.
    /// - `Ok(None)` if the rule is empty/public or the user is an admin.
    /// - `Err(RepositoryError::Forbidden)` if rule is None and caller is not an admin.
    pub fn compile_rule(
        rule: &Option<String>,
        sql_context: &SqlContext,
        param_offset: usize,
    ) -> Result<Option<CompiledRule>, RepositoryError> {
        if sql_context.is_admin() {
            return Ok(None);
        }

        match rule {
            None => Err(RepositoryError::Forbidden(
                "Only admin can perform this action".to_string(),
            )),
            Some(r) if r.trim().is_empty() => Ok(None),
            Some(expr) => {
                let tokens = tokenize(expr);
                let mut parser = RuleParser::new(tokens);

                let ast = parser.parse().map_err(|e| RepositoryError::Validation {
                    message: format!("Invalid rule expression: {}", e),
                    field: None,
                })?;

                let mut compiler = RulesSqlCompiler::new(sql_context.clone());
                compiler.binding_offset = param_offset;

                let sql_clause =
                    compiler
                        .compile(&ast)
                        .map_err(|e| RepositoryError::Validation {
                            message: format!("Failed to compile rule: {}", e),
                            field: None,
                        })?;

                Ok(Some(CompiledRule {
                    sql_clause,
                    bindings: compiler.bindings,
                }))
            }
        }
    }
    /// Batch-expands relation columns in-place across a slice of records,
    /// strictly enforcing target collection view rules and redacting hidden fields.
    pub async fn expand_records(
        &self,
        records: &mut [Record],
        collection: &Collection,
        expand_param: Option<&str>,
        sql_context: &SqlContext,
    ) -> Result<(), RepositoryError> {
        let Some(expand_str) = expand_param.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(());
        };

        if records.is_empty() {
            return Ok(());
        }

        let is_admin = sql_context.is_admin();
        let col_repo = CollectionRepository::new(self.db.clone());

        //1. parse and deduplicate request field names (?expand=author,category)
        let requested_fields: HashSet<&str> = expand_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        // 2. Identify relation columns in parent collection
        let relation_columns: Vec<&Column> = requested_fields
            .into_iter()
            .filter_map(|field_name| {
                collection
                    .fields
                    .iter()
                    .find(|col| col.name == field_name && col.data_type == DataTypes::Relation)
            })
            .collect();

        // 3. identify each relation sequanetially
        for col in relation_columns {
            let Some(target_table) = col.related_to.as_deref().filter(|s| !s.is_empty()) else {
                continue;
            };

            // Fetch target collection schema to verify existence & read view_rule / hidden flags
            let target_col = match col_repo.get_by_name(target_table).await {
                Ok(c) => c,
                Err(e) => return Err(e),
            };

            // Collect unique foreign key UUIDs across all records on this page
            let mut unique_uuids: HashSet<Uuid> = HashSet::new();

            for record in records.iter() {
                if let Some(val) = record.data.get(&col.name) {
                    if let Some(id_str) = val.as_str() {
                        if let Ok(parsed_uuid) = Uuid::parse_str(id_str) {
                            unique_uuids.insert(parsed_uuid);
                        }
                    }
                }
            }

            // Short-circuit: if no records have foreign keys, skip DB round-trip
            if unique_uuids.is_empty() {
                continue;
            }

            let uuid_list: Vec<Uuid> = unique_uuids.into_iter().collect();

            // Build base query: WHERE id = ANY($1)
            let mut sql = format!(
                "SELECT * FROM {} WHERE id = ANY($1)",
                quote_ident(target_table)
            );

            // Compile target view_rule if not admin
            let mut bindings: Vec<String> = Vec::new();
            if !is_admin {
                if let Some(rule_expr) = &target_col.view_rule {
                    if !rule_expr.trim().is_empty() {
                        if let Ok(Some(compiled)) =
                            Self::compile_rule(&target_col.view_rule, sql_context, 1)
                        {
                            sql.push_str(&format!(" AND ({})", compiled.sql_clause));
                            bindings = compiled.bindings;
                        }
                    }
                }
            }

            let mut query = sqlx::query(&sql).bind(&uuid_list);
            for b in &bindings {
                query = query.bind(b);
            }

            let rows = query.fetch_all(&self.db).await?;

            // Determine hidden columns on target collection
            let hidden_fields: HashSet<&str> = target_col
                .fields
                .iter()
                .filter(|f| f.hidden)
                .map(|f| f.name.as_str())
                .collect();

            // Build lookup map and redact hidden fields
            let mut lookup: HashMap<String, Record> = HashMap::with_capacity(rows.len());
            for row in rows {
                let mut rec = Record::from_row(&row)?;
                if !hidden_fields.is_empty() {
                    rec.data.retain(|k, _| !hidden_fields.contains(k.as_str()));
                }
                lookup.insert(rec.id.clone(), rec);
            }

            // Attach expanded record into record.expand["<col_name>"]
            for record in records.iter_mut() {
                if let Some(val) = record.data.get(&col.name) {
                    if let Some(fk_str) = val.as_str() {
                        if let Some(related_record) = lookup.get(fk_str) {
                            let expand_obj = record.expand.get_or_insert_with(serde_json::Map::new);
                            let serialized =
                                serde_json::to_value(related_record).unwrap_or(Value::Null);
                            expand_obj.insert(col.name.clone(), serialized);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn list(
        &self,
        collection: &str,
        sql_context: SqlContext,
        params: PaginationParams,
    ) -> Result<RecordListResponse, RepositoryError> {
        let is_admin = sql_context.is_admin();
        let page = params.page.unwrap_or(1);
        let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

        let (order, key) = params
            .sort
            .as_deref()
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                let first = chars.next().unwrap_or_default().to_string();
                let rest = chars.as_str().to_string();
                (first, rest)
            })
            .unwrap_or_else(|| ("".to_string(), "".to_string()));

        let col = CollectionRepository::new(self.db.clone())
            .get_by_name(collection)
            .await?;

        let client_filter = params.filter.as_deref().filter(|s| !s.is_empty());

        let effective_rule: Option<String> = if is_admin {
            client_filter.map(|f| f.to_string())
        } else {
            match &col.list_rule {
                None => {
                    return Err(RepositoryError::Forbidden(
                        "Only admin can perform this action".to_string(),
                    ));
                }
                Some(rule) if rule.trim().is_empty() => client_filter.map(|f| f.to_string()),
                Some(rule) => match client_filter {
                    Some(cf) => Some(format!("({}) && ({})", rule, cf)),
                    None => Some(rule.clone()),
                },
            }
        };

        let mut base_query = format!("SELECT * FROM {}", quote_ident(collection));
        let mut count_base_query = format!("SELECT COUNT(id) FROM {}", quote_ident(collection));
        let mut bindings: Vec<String> = vec![];

        if let Some(rule_expr) = effective_rule {
            let tokens = tokenize(&rule_expr);

            let mut parser = RuleParser::new(tokens);

            let ast = parser.parse().map_err(|e| RepositoryError::Validation {
                message: format!("Invalid filter expression: {}", e),
                field: None,
            })?;

            let mut compiler = RulesSqlCompiler::new(sql_context.clone());

            let sql_clause = compiler
                .compile(&ast)
                .map_err(|e| RepositoryError::Validation {
                    message: format!("failed to compile filter: {}", e),
                    field: None,
                })?;

            base_query.push_str(&format!(" WHERE {}", sql_clause));
            count_base_query.push_str(&format!(" WHERE {}", sql_clause));
            bindings = compiler.bindings;
        }

        if !order.is_empty() && !key.is_empty() {
            let order_dir = if order == "-" { "DESC" } else { "ASC" };

            base_query.push_str(&format!(" ORDER BY {} {}", quote_ident(&key), order_dir));
        }

        let limit_idx = bindings.len() + 1;
        let offset_idx = bindings.len() + 2;
        base_query.push_str(&format!(" LIMIT ${limit_idx} OFFSET ${offset_idx}"));

        let offset = (page - 1) * per_page;

        let mut query = sqlx::query(&base_query);
        let mut count_query = sqlx::query_scalar(&count_base_query);

        for bind_val in &bindings {
            query = query.bind(bind_val);
            count_query = count_query.bind(bind_val);
        }

        let query = query.bind(per_page as i64).bind(offset as i64);

        let result = query.fetch_all(&self.db).await?;

        let total_count: i64 = count_query.fetch_one(&self.db).await?;
        let mut items = result
            .iter()
            .filter_map(|r| Record::from_row(r).ok())
            .collect::<Vec<Record>>();

        // In-place expand
        self.expand_records(&mut items, &col, params.expand.as_deref(), &sql_context)
            .await?;

        Ok(RecordListResponse {
            items,
            total: total_count as u64,
            page,
            per_page,
        })
    }

    pub async fn get_record(
        &self,
        collection: &str,
        id: &str,
        expand: Option<&str>,
        sql_context: &SqlContext,
    ) -> Result<Record, RepositoryError> {
        let col_repo = CollectionRepository::new(self.db.clone());

        let col = col_repo.get_by_name(collection).await?;

        let id_uuid = uuid::Uuid::parse_str(id).ok();

        let mut sql = format!("SELECT * FROM {} WHERE id = $1", quote_ident(collection));
        let mut bindings: Vec<String> = Vec::new();

        if !sql_context.is_admin() {
            if let Some(ref rule_expr) = col.view_rule {
                if !rule_expr.trim().is_empty() {
                    if let Ok(Some(compiled)) = Self::compile_rule(&col.view_rule, sql_context, 1) {
                        sql.push_str(&format!(" AND ({})", compiled.sql_clause));
                        bindings = compiled.bindings;
                    }
                }
            }
        }

        let mut query = sqlx::query(&sql);
        if let Some(uuid) = id_uuid {
            query = query.bind(uuid);
        } else {
            query = query.bind(id.to_string());
        };

        for b in &bindings {
            query = query.bind(b)
        }

        let row = query.fetch_one(&self.db).await?;
        let mut record = Record::from_row(&row)?;

        // Redact hidden fields on primary record
        let hidden_fields: HashSet<&str> = col
            .fields
            .iter()
            .filter(|f| f.hidden)
            .map(|f| f.name.as_str())
            .collect();

        if !hidden_fields.is_empty() {
            record
                .data
                .retain(|k, _| !hidden_fields.contains(k.as_str()));
        }

        // In place expand single record
        self.expand_records(std::slice::from_mut(&mut record), &col, expand, sql_context)
            .await?;

        Ok(record)
    }

    pub async fn create_record(
        &self,
        collection: String,
        mut body: CreateRecordRequest,
        sql_context: SqlContext,
    ) -> Result<Record, RepositoryError> {
        let _is_admin = sql_context.is_admin();

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

        let is_uuid_col = |col_name: &str| -> bool {
            col_name == "id"
                || col
                    .fields
                    .iter()
                    .any(|c| c.name == col_name && c.data_type == DataTypes::Relation)
        };

        let entries: Vec<(&String, &Value)> = obj.iter().collect();

        let quoted_table = quote_ident(&collection);
        let mut query_builder =
            sqlx::QueryBuilder::<Postgres>::new(format!("INSERT INTO {} (", quoted_table));

        // Add columns with proper sepration
        let mut separated = query_builder.separated(",");

        for (col, _) in &entries {
            separated.push(quote_ident(col));
        }

        separated.push_unseparated(") VALUES (");

        // Add values as bound parameters
        let mut separated_values = query_builder.separated(",");
        for (col, v) in &entries {
            if is_uuid_col(col) {
                match v {
                    Value::String(s) => {
                        let parsed =
                            Uuid::parse_str(s).map_err(|e| RepositoryError::Validation {
                                message: format!("invalid UUID for column '{}': {}", col, e),
                                field: Some(col.to_string()),
                            })?;
                        separated_values.push_bind(parsed);
                    }
                    Value::Null => {
                        separated_values.push_bind(Option::<Uuid>::None);
                    }
                    other => {
                        let parsed = Uuid::parse_str(&other.to_string()).map_err(|e| {
                            RepositoryError::Validation {
                                message: format!("invalid UUID for column '{}': {}", col, e),
                                field: Some(col.to_string()),
                            }
                        })?;
                        separated_values.push_bind(parsed);
                    }
                }
            } else {
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
                    expand: None,
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
        sql_context: &SqlContext,
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
        let existing_record = self.get_record(collection, id, None, &sql_context).await?;

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

        let is_uuid_col = |col_name: &str| -> bool {
            col_name == "id"
                || col
                    .fields
                    .iter()
                    .any(|c| c.name == col_name && c.data_type == DataTypes::Relation)
        };

        let mut first = true;
        for (k, v) in &payload.data {
            if !first {
                query_builder.push(", ");
            }
            first = false;

            query_builder.push(quote_ident(k)).push(" = ");
            if is_uuid_col(k) {
                match v {
                    Value::String(s) => {
                        let parsed =
                            Uuid::parse_str(s).map_err(|e| RepositoryError::Validation {
                                message: format!("invalid UUID for column '{}': {}", k, e),
                                field: Some(k.to_string()),
                            })?;
                        query_builder.push_bind(parsed);
                    }
                    Value::Null => {
                        query_builder.push_bind(Option::<Uuid>::None);
                    }
                    other => {
                        let parsed = Uuid::parse_str(&other.to_string()).map_err(|e| {
                            RepositoryError::Validation {
                                message: format!("invalid UUID for column '{}': {}", k, e),
                                field: Some(k.to_string()),
                            }
                        })?;
                        query_builder.push_bind(parsed);
                    }
                }
            } else {
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

        self.get_record(collection, id, None, &sql_context).await
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
            .create_record("articles".to_string(), create_req, SqlContext::default())
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
            .create_record("items".to_string(), create_req, SqlContext::default())
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
            .create_record("items".to_string(), create_req, SqlContext::default())
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
            .create_record("users".to_string(), create_req, SqlContext::default())
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

        col_repo
            .update(
                "blogs".to_string(),
                crabbase_core::models::UpdateCollectionRequest {
                    list_rule: Some("".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let repo = RecordsRepository::new(pool.clone());
        for i in 0..3 {
            let mut data = Map::new();
            data.insert("title".to_string(), Value::String(format!("t{}", i)));
            data.insert("views".to_string(), Value::Number((i as i64).into()));
            let create_req = CreateRecordRequest { data };
            repo.create_record("blogs".to_string(), create_req, SqlContext::default())
                .await
                .unwrap();
        }

        let listed = repo
            .list(
                "blogs",
                SqlContext::default(),
                PaginationParams {
                    page: Some(1),
                    per_page: Some(10),
                    filter: None,
                    sort: None,
                    expand: None,
                    fields: None,
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
            .create_record("trash".to_string(), create_req, SqlContext::default())
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
