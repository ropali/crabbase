use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::error::Error;
use std::fmt;
use tracing::error;
use uuid::Uuid;

#[derive(Debug)]
pub enum RepositoryError {
    Sqlx(sqlx::Error),
    InvalidInput(String),
    NotFound,
    // TODO: add other specific errors here later, e.g., NotFound, InvalidInput
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RepositoryError::Sqlx(sql_err) => write!(f, "Database error: {}", sql_err),
            RepositoryError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            RepositoryError::NotFound => write!(f, "Resource not found"),
        }
    }
}

impl Error for RepositoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RepositoryError::Sqlx(sql_err) => Some(sql_err),
            RepositoryError::InvalidInput(_) | RepositoryError::NotFound => None,
        }
    }
}

// Conversion from sqlx::Error to RepositoryError allows using the `?` operator
impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        RepositoryError::Sqlx(err)
    }
}

use crate::api::models::{
    Collection, CollectionListResponse, Column, CreateCollectionRequest, UpdateCollectionRequest,
};

#[derive(Debug, Clone)]
pub struct CollectionRepository {
    db: Pool<Sqlite>,
}

impl CollectionRepository {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn exists(&self, name: &str) -> Result<bool, RepositoryError> {
        let col = sqlx::query("SELECT name from collections WHERE name = $1")
            .bind(name)
            .fetch_one(&self.db)
            .await?;

        Ok(true)
    }

    pub async fn create(
        &self,
        collection: CreateCollectionRequest,
    ) -> Result<Collection, RepositoryError> {
        validate_identifier(&collection.name)?;
        validate_columns(&collection.columns)?;

        let columns = collection
            .columns
            .iter()
            .map(|c| format!("\"{}\" {}", c.name, c.data_type.clone().to_db_type()))
            .collect::<Vec<String>>()
            .join(", ");

        let indexes = collection
            .columns
            .iter()
            .filter(|c| c.index)
            .map(|c| {
                format!(
                    "CREATE INDEX idx_{0}_{1} ON \"{0}\" (\"{1}\");",
                    collection.name, c.name
                )
            })
            .collect::<Vec<String>>()
            .join(" ");

        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS "{}"
            (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                {},
                created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
                updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
            );
            {}
        "#,
            collection.name, columns, indexes
        );

        let mut tx = self.db.begin().await?;

        sqlx::query(&sql).execute(&mut *tx).await?;

        let columns_json =
            serde_json::to_string(&collection.columns).expect("Failed to serialize columns");
        let indexs_json = serde_json::to_string(
            &collection
                .columns
                .iter()
                .filter(|c| c.index)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .expect("Failed to serialize indexes");

        let sql = format!(
            r#"
                INSERT INTO _collections(id, system, name, fields, indexes)
                VALUES ('{}', {}, '{}', '{}', '{}')
            "#,
            Uuid::new_v4(),
            false,
            collection.name,
            columns_json,
            indexs_json
        );

        sqlx::query(&sql).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name: collection.name,
            fields: collection.columns.clone(),
            indexes: collection
                .columns
                .iter()
                .filter(|c| c.index)
                .cloned()
                .collect(),
            created: Utc::now().to_string(),
            updated: Utc::now().to_string(),
        })
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Collection, RepositoryError> {
        let sql = "SELECT * FROM _collections WHERE name = $1;";

        let res = sqlx::query_as::<_, Collection>(sql)
            .bind(name)
            .fetch_one(&self.db)
            .await;

        match res {
            Ok(c) => Ok(c),
            Err(err) => {
                error!("Error: {}", err);
                Err(RepositoryError::Sqlx(err))
            }
        }
    }

    pub async fn list(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<CollectionListResponse, RepositoryError> {
        let q = r#"SELECT * FROM _collections LIMIT ? OFFSET ?"#;

        let offset = (page - 1) * per_page;

        let result = sqlx::query_as::<_, Collection>(q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await?;

        let total = result.len();

        Ok(CollectionListResponse {
            items: result,
            total: total as u64,
            page,
            per_page,
        })
    }

    pub async fn update(
        &self,
        current_name: String,
        payload: UpdateCollectionRequest,
    ) -> Result<Collection, RepositoryError> {
        let mut tx = self.db.begin().await?;

        let current = sqlx::query_as::<_, Collection>("SELECT * FROM _collections WHERE name = ?")
            .bind(&current_name)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(RepositoryError::NotFound)?;

        let next_name = payload.name.unwrap_or_else(|| current.name.clone());
        validate_identifier(&next_name)?;

        let next_fields = payload.columns.unwrap_or_else(|| current.fields.clone());
        validate_columns(&next_fields)?;

        let next_indexes: Vec<Column> = next_fields.iter().filter(|c| c.index).cloned().collect();

        if current.name != next_name || current.fields != next_fields {
            rebuild_collection_table(
                &mut tx,
                &current.name,
                &next_name,
                &current.fields,
                &next_fields,
            )
            .await?;
        }

        let next_fields_json = serde_json::to_string(&next_fields).map_err(|e| {
            RepositoryError::InvalidInput(format!("unable to serialize fields: {e}"))
        })?;
        let next_indexes_json = serde_json::to_string(&next_indexes).map_err(|e| {
            RepositoryError::InvalidInput(format!("unable to serialize indexes: {e}"))
        })?;

        sqlx::query(
            "UPDATE _collections SET name = ?, fields = ?, indexes = ?, updated = strftime('%Y-%m-%d %H:%M:%fZ') WHERE id = ?",
        )
        .bind(&next_name)
        .bind(&next_fields_json)
        .bind(&next_indexes_json)
        .bind(&current.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_by_name(&next_name).await
    }

    pub async fn delete(&self, name: String) -> Result<bool, RepositoryError> {
        let sql = "DELETE FROM _collections WHERE name = $1";

        let result = sqlx::query(sql).bind(name).execute(&self.db).await;

        match result {
            Ok(_) => Ok(true),
            Err(err) => {
                eprintln!("Error: {}", err);

                Err(RepositoryError::Sqlx(err))
            }
        }
    }
}

fn validate_identifier(identifier: &str) -> Result<(), RepositoryError> {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return Err(RepositoryError::InvalidInput(
            "identifier cannot be empty".to_string(),
        ));
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(RepositoryError::InvalidInput(format!(
            "identifier '{identifier}' must start with a letter or underscore"
        )));
    }

    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(RepositoryError::InvalidInput(format!(
            "identifier '{identifier}' can contain only letters, numbers, and underscores"
        )));
    }

    Ok(())
}

fn validate_columns(columns: &[Column]) -> Result<(), RepositoryError> {
    if columns.is_empty() {
        return Err(RepositoryError::InvalidInput(
            "at least one column is required".to_string(),
        ));
    }

    let mut seen = std::collections::HashSet::new();

    for column in columns {
        validate_identifier(&column.name)?;

        if matches!(column.name.as_str(), "id" | "created" | "updated") {
            return Err(RepositoryError::InvalidInput(format!(
                "column '{}' is reserved",
                column.name
            )));
        }

        if !seen.insert(column.name.clone()) {
            return Err(RepositoryError::InvalidInput(format!(
                "duplicate column '{}'",
                column.name
            )));
        }
    }

    Ok(())
}

async fn rebuild_collection_table(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    current_name: &str,
    next_name: &str,
    current_fields: &[Column],
    next_fields: &[Column],
) -> Result<(), RepositoryError> {
    let temp_name = format!("{}_tmp_{}", next_name, Uuid::new_v4().simple());

    let next_column_defs = next_fields
        .iter()
        .map(|c| format!("\"{}\" {}", c.name, c.data_type.clone().to_db_type()))
        .collect::<Vec<_>>()
        .join(", ");

    let create_sql = format!(
        "CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT, {}, created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')), updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')))",
        temp_name, next_column_defs
    );
    sqlx::query(&create_sql).execute(&mut **tx).await?;

    let current_set: std::collections::HashSet<&str> =
        current_fields.iter().map(|c| c.name.as_str()).collect();
    let shared = next_fields
        .iter()
        .filter(|c| current_set.contains(c.name.as_str()))
        .map(|c| format!("\"{}\"", c.name))
        .collect::<Vec<_>>();

    let mut insert_columns = vec!["\"id\"".to_string()];
    insert_columns.extend(shared.clone());
    insert_columns.push("\"created\"".to_string());
    insert_columns.push("\"updated\"".to_string());
    let insert_clause = insert_columns.join(", ");

    let copy_sql = format!(
        "INSERT INTO \"{}\" ({}) SELECT {} FROM \"{}\"",
        temp_name, insert_clause, insert_clause, current_name
    );
    sqlx::query(&copy_sql).execute(&mut **tx).await?;

    let drop_sql = format!("DROP TABLE \"{}\"", current_name);
    sqlx::query(&drop_sql).execute(&mut **tx).await?;

    let rename_sql = format!("ALTER TABLE \"{}\" RENAME TO \"{}\"", temp_name, next_name);
    sqlx::query(&rename_sql).execute(&mut **tx).await?;

    let index_sql = next_fields
        .iter()
        .filter(|c| c.index)
        .map(|c| {
            format!(
                "CREATE INDEX IF NOT EXISTS \"idx_{}_{}\" ON \"{}\" (\"{}\")",
                next_name, c.name, next_name, c.name
            )
        })
        .collect::<Vec<_>>();

    for sql in index_sql {
        sqlx::query(&sql).execute(&mut **tx).await?;
    }

    Ok(())
}
