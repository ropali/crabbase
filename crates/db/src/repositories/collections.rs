use chrono::Utc;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crabbase_core::{
    errors::RepositoryError,
    models::{
        Collection, CollectionListResponse, CollectionOptions, Column, CreateCollectionRequest,
        UpdateCollectionRequest,
    },
    utils::string_utils::random_str,
};

#[derive(Debug, Clone)]
pub struct CollectionRepository {
    db: Pool<Postgres>,
}

impl CollectionRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    pub async fn exists(&self, name: &str) -> bool {
        sqlx::query("SELECT name from _collections WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.db)
            .await
            .map(|row| row.is_some())
            .unwrap_or(false)
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
            .map(|c| c.to_sql_definition())
            .collect::<Vec<String>>()
            .join(", ");

        let table_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS "{}"
            (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                {},
                created TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated TIMESTAMPTZ NOT NULL DEFAULT now()
            );
        "#,
            collection.name, columns
        );

        let mut tx = self.db.begin().await?;

        sqlx::query(&table_sql).execute(&mut *tx).await?;

        for c in collection.columns.iter().filter(|c| c.index) {
            let index_sql = format!(
                "CREATE INDEX IF NOT EXISTS idx_{0}_{1} ON \"{0}\" (\"{1}\");",
                collection.name, c.name
            );
            sqlx::query(&index_sql).execute(&mut *tx).await?;
        }

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

        let mut options = serde_json::Map::new();
        options.insert("secret".to_string(), random_str(None).into());

        options.insert("duration".to_string(), 432000.into());

        let options_json =
            serde_json::to_string(&options).expect("Failed to serialized optional data.");

        let col_id = Uuid::new_v4().to_string();

        let sql = format!(
            r#"
                INSERT INTO _collections(id, system, name, type, fields, indexes, options)
                VALUES ('{}', {}, '{}', '{}', '{}'::jsonb, '{}'::jsonb, '{}'::jsonb)
            "#,
            col_id,
            0,
            collection.name,
            collection.collection_type,
            columns_json,
            indexs_json,
            options_json
        );

        sqlx::query(&sql).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(Collection {
            id: col_id,
            name: collection.name.clone(),
            system: collection.name.starts_with("_"),
            fields: collection.columns.clone(),
            indexes: collection
                .columns
                .iter()
                .filter(|c| c.index)
                .cloned()
                .collect(),
            options: CollectionOptions {
                auth_token: Some(options),
            },
            created: Utc::now(),
            updated: Utc::now(),
            list_rule: None,
            view_rule: None,
            create_rule: None,
            update_rule: None,
            delete_rule: None,
            collection_type: collection.collection_type.to_string(),
        })
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Collection, RepositoryError> {
        let sql = "SELECT * FROM _collections WHERE name = $1;";

        sqlx::query_as::<_, Collection>(sql)
            .bind(name)
            .fetch_one(&self.db)
            .await
            .map_err(|_| RepositoryError::NotFound(name.to_string()))
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Collection, RepositoryError> {
        let sql = "SELECT * FROM _collections WHERE id = $1;";

        sqlx::query_as::<_, Collection>(sql)
            .bind(id)
            .fetch_one(&self.db)
            .await
            .map_err(|_| RepositoryError::NotFound(id.to_string()))
    }
    pub async fn list(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<CollectionListResponse, RepositoryError> {
        let q = r#"SELECT * FROM _collections LIMIT $1 OFFSET $2"#;

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

        let current = sqlx::query_as::<_, Collection>("SELECT * FROM _collections WHERE name = $1")
            .bind(&current_name)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(RepositoryError::NotFound(
                "Table does not exist".to_string(),
            ))?;

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

        let next_fields_json = serde_json::to_string(&next_fields)
            .map_err(|e| RepositoryError::OtherError(format!("unable to serialize fields: {e}")))?;
        let next_indexes_json = serde_json::to_string(&next_indexes).map_err(|e| {
            RepositoryError::OtherError(format!("unable to serialize indexes: {e}"))
        })?;

        sqlx::query(
            "UPDATE _collections SET name = $1, fields = $2::jsonb, indexes = $3::jsonb, updated = now() WHERE id = $4",
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

        // Begin a transaction and run all mutating queries within it using the same connection
        let mut tx = self.db.begin().await?;

        // Execute the delete within the transaction
        let affected = sqlx::query(sql).bind(&name).execute(&mut *tx).await?;

        if affected.rows_affected() == 0 {
            // rollback and return not found
            tx.rollback().await.ok();
            return Err(RepositoryError::NotFound(name));
        }

        // Drop the collection table within the same transaction
        let drop_sql = format!("DROP TABLE \"{}\"", name);
        sqlx::query(&drop_sql).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(true)
    }

    pub async fn truncate(&self, name: String) -> Result<bool, RepositoryError> {
        let sql = format!("DELETE FROM {};", name);

        sqlx::query(&sql).execute(&self.db).await?;
        Ok(true)
    }
}

fn validate_identifier(identifier: &str) -> Result<(), RepositoryError> {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return Err(RepositoryError::Validation {
            message: "identifier cannot be empty".to_string(),
            field: Some("some".to_string()),
        });
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(RepositoryError::Validation {
            message: format!("identifier '{identifier}' must start with a letter or underscore"),
            field: Some("name".to_string()),
        });
    }

    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(RepositoryError::Validation {
            message: format!(
                "identifier '{identifier}' can contain only letters, numbers, and underscores"
            ),
            field: Some("name".to_string()),
        });
    }

    Ok(())
}

fn validate_columns(columns: &[Column]) -> Result<(), RepositoryError> {
    if columns.is_empty() {
        return Err(RepositoryError::OtherError(
            "at least one column is required".to_string(),
        ));
    }

    let mut seen = std::collections::HashSet::new();

    for column in columns {
        validate_identifier(&column.name)?;

        if matches!(column.name.as_str(), "id" | "created" | "updated") {
            return Err(RepositoryError::OtherError(format!(
                "column '{}' is reserved",
                column.name
            )));
        }

        if !seen.insert(column.name.clone()) {
            return Err(RepositoryError::OtherError(format!(
                "duplicate column '{}'",
                column.name
            )));
        }
    }

    Ok(())
}

async fn rebuild_collection_table(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    current_name: &str,
    next_name: &str,
    current_fields: &[Column],
    next_fields: &[Column],
) -> Result<(), RepositoryError> {
    let temp_name = format!("{}_tmp_{}", next_name, Uuid::new_v4().simple());

    let next_column_defs = next_fields
        .iter()
        .map(|c| c.to_sql_definition())
        .collect::<Vec<_>>()
        .join(", ");

    let create_sql = format!(
        "CREATE TABLE \"{}\" (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), {}, created TIMESTAMPTZ NOT NULL DEFAULT now(), updated TIMESTAMPTZ NOT NULL DEFAULT now())",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crabbase_core::errors::RepositoryError;
    use crabbase_core::models::{
        Column, CreateCollectionRequest, DataTypes, UpdateCollectionRequest,
    };
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
        // Clean seeded collections (migrations insert a default 'users' entry) to keep tests deterministic
        sqlx::query("DELETE FROM _collections;")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_collection() {
        let pool = setup_pool("col_create_collection").await;
        let repo = CollectionRepository::new(pool.clone());

        let columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];

        let req = CreateCollectionRequest {
            name: "testcol".to_string(),
            columns: columns.clone(),
        };

        let created = repo.create(req).await.unwrap();
        assert_eq!(created.name, "testcol");
        assert_eq!(created.fields, columns);
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let pool = setup_pool("col_get_by_name").await;
        let repo = CollectionRepository::new(pool.clone());

        let columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];

        let req = CreateCollectionRequest {
            name: "getcol".to_string(),
            columns: columns.clone(),
        };

        repo.create(req).await.unwrap();

        let fetched = repo.get_by_name("getcol").await.unwrap();
        assert_eq!(fetched.name, "getcol");
    }

    #[tokio::test]
    async fn test_list_collections() {
        let pool = setup_pool("col_list_collections").await;
        let repo = CollectionRepository::new(pool.clone());

        let columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];

        let req = CreateCollectionRequest {
            name: "listcol".to_string(),
            columns: columns.clone(),
        };

        repo.create(req).await.unwrap();

        let list = repo.list(1, 10).await.unwrap();
        assert!(list.items.iter().any(|c| c.name == "listcol"));
    }

    #[tokio::test]
    async fn test_update_collection_rename() {
        let pool = setup_pool("col_update_collection_rename").await;
        let repo = CollectionRepository::new(pool.clone());

        let columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];

        let req = CreateCollectionRequest {
            name: "oldname".to_string(),
            columns: columns.clone(),
        };

        repo.create(req).await.unwrap();

        let update_req = UpdateCollectionRequest {
            name: Some("newname".to_string()),
            columns: None,
        };

        let updated = repo
            .update("oldname".to_string(), update_req)
            .await
            .unwrap();
        assert_eq!(updated.name, "newname");
    }

    #[tokio::test]
    async fn test_delete_collection() {
        let pool = setup_pool("col_delete_collection").await;
        let repo = CollectionRepository::new(pool.clone());

        let columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];

        let req = CreateCollectionRequest {
            name: "todelete".to_string(),
            columns: columns.clone(),
        };

        repo.create(req).await.unwrap();

        let deleted = repo.delete("todelete".to_string()).await.unwrap();
        assert!(deleted);

        let res = repo.get_by_name("todelete").await;
        assert!(matches!(res, Err(RepositoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_with_various_data_types() {
        let pool = setup_pool("col_create_with_various_data_types").await;
        let repo = CollectionRepository::new(pool.clone());

        // Create the related collection first
        let related_columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];
        let req_related = CreateCollectionRequest {
            name: "other_table".to_string(),
            columns: related_columns,
        };
        repo.create(req_related).await.unwrap();

        let columns = vec![
            Column {
                name: "text_field".into(),
                data_type: DataTypes::PlainText,
                index: false,
                related_to: None,
            },
            Column {
                name: "rich_field".into(),
                data_type: DataTypes::RichText,
                index: false,
                related_to: None,
            },
            Column {
                name: "num_field".into(),
                data_type: DataTypes::Number,
                index: false,
                related_to: None,
            },
            Column {
                name: "bool_field".into(),
                data_type: DataTypes::Bool,
                index: false,
                related_to: None,
            },
            Column {
                name: "email_field".into(),
                data_type: DataTypes::Email,
                index: false,
                related_to: None,
            },
            Column {
                name: "url_field".into(),
                data_type: DataTypes::Url,
                index: false,
                related_to: None,
            },
            Column {
                name: "dt_field".into(),
                data_type: DataTypes::Datetime,
                index: false,
                related_to: None,
            },
            Column {
                name: "autodt_field".into(),
                data_type: DataTypes::AutoDatetime("now".into()),
                index: false,
                related_to: None,
            },
            Column {
                name: "file_field".into(),
                data_type: DataTypes::File,
                index: false,
                related_to: None,
            },
            Column {
                name: "relation_field".into(),
                data_type: DataTypes::Relation,
                index: false,
                related_to: Some("other_table".into()),
            },
            Column {
                name: "select_field".into(),
                data_type: DataTypes::Select,
                index: false,
                related_to: None,
            },
            Column {
                name: "json_field".into(),
                data_type: DataTypes::Json,
                index: false,
                related_to: None,
            },
            Column {
                name: "geo_field".into(),
                data_type: DataTypes::GeoPoint,
                index: false,
                related_to: None,
            },
            // include an indexed column to ensure indexes are recorded
            Column {
                name: "indexed".into(),
                data_type: DataTypes::PlainText,
                index: true,
                related_to: None,
            },
        ];

        let req = CreateCollectionRequest {
            name: "types_test".to_string(),
            columns: columns.clone(),
        };

        let created = repo.create(req).await.unwrap();
        // Ensure created fields match the input columns
        assert_eq!(created.fields, columns);

        // Ensure indexes captured correctly
        assert!(created.indexes.iter().any(|c| c.name == "indexed"));

        // Fetch from DB and ensure it deserializes correctly
        let fetched = repo.get_by_name("types_test").await.unwrap();
        assert_eq!(fetched.fields, columns);
    }

    #[tokio::test]
    async fn test_relation_type_creation() {
        let pool = setup_pool("col_relation_type_creation").await;
        let repo = CollectionRepository::new(pool.clone());

        // Create the related collection first
        let related_columns = vec![Column {
            name: "title".to_string(),
            data_type: DataTypes::PlainText,
            index: false,
            related_to: None,
        }];
        let req_related = CreateCollectionRequest {
            name: "other_table".to_string(),
            columns: related_columns,
        };
        repo.create(req_related).await.unwrap();

        // Create the main collection with the relation column and a dummy column to avoid empty column SQL error
        let columns = vec![
            Column {
                name: "dummy".to_string(),
                data_type: DataTypes::PlainText,
                index: false,
                related_to: None,
            },
            Column {
                name: "relation_field".to_string(),
                data_type: DataTypes::Relation,
                index: false,
                related_to: Some("other_table".to_string()),
            },
        ];
        let req = CreateCollectionRequest {
            name: "my_table".to_string(),
            columns,
        };
        repo.create(req).await.unwrap();

        #[derive(sqlx::FromRow, Debug)]
        struct ForeignKeyInfo {
            table_name: String,
            column_name: String,
            referenced_table: String,
        }

        let fks: Vec<ForeignKeyInfo> = sqlx::query_as::<_, ForeignKeyInfo>(
            r#"
            SELECT
                kcu.table_name as table_name,
                kcu.column_name as column_name,
                ccu.table_name AS referenced_table
            FROM
                information_schema.table_constraints AS tc
                JOIN information_schema.key_column_usage AS kcu
                  ON tc.constraint_name = kcu.constraint_name
                JOIN information_schema.constraint_column_usage AS ccu
                  ON ccu.constraint_name = tc.constraint_name
            WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_name = 'my_table'
            "#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        // Verify that there is a foreign key pointing to "other_table"
        assert!(!fks.is_empty(), "No foreign keys found on my_table");
        let fk = &fks[0];
        assert_eq!(fk.referenced_table, "other_table");
        assert_eq!(fk.column_name, "relation_field");
    }
}
