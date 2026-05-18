use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::fmt;
use std::{error::Error, result};
use uuid::Uuid;

#[derive(Debug)]
pub enum RepositoryError {
    Sqlx(sqlx::Error),
    // TODO: add other specific errors here later, e.g., NotFound, InvalidInput
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RepositoryError::Sqlx(sql_err) => write!(f, "Database error: {}", sql_err),
        }
    }
}

impl Error for RepositoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RepositoryError::Sqlx(sql_err) => Some(sql_err),
        }
    }
}

// Conversion from sqlx::Error to RepositoryError allows using the `?` operator
impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        RepositoryError::Sqlx(err)
    }
}

use crate::api::models::{Collection, CollectionListResponse, CreateCollectionRequest};

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
        let columns = collection
            .columns
            .iter()
            .map(|c| format!("{} {}", c.name, c.data_type.clone().to_db_type()))
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
            CREATE TABLE IF NOT EXISTS {}
            (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                {}
                created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
                updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
            );
            {}
        "#,
            collection.name, columns, indexes
        );

        let tx = self.db.begin().await?;

        sqlx::query(&sql).execute(&self.db).await?;

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

        sqlx::query(&sql).execute(&self.db).await?;

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
                eprintln!("Error: {}", err);
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

    pub async fn delete(&self, name: String) -> Result<bool, RepositoryError> {
        let sql = "DELETE FROM _collections WHERE name = $1";

        let result = sqlx::query(sql).bind(name).execute(&self.db).await;

        match result {
            Ok(qs) => Ok(true),
            Err(err) => {
                eprintln!("Error: {}", err);

                Err(RepositoryError::Sqlx(err))
            }
        }
    }
}
