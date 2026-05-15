use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::api::models::{Collection, CollectionListResponse, CreateCollectionRequest};

#[derive(Debug, Clone)]
pub struct CollectionRepository {
    db: Pool<Sqlite>,
}

impl CollectionRepository {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn exists(&self, name: &str) -> bool {
        let col = sqlx::query("SELECT name from collections WHERE name = $1")
            .bind(name)
            .fetch_one(&self.db)
            .await;

        match col {
            Ok(value) => return true,
            Err(e) => return false,
        }
    }

    pub async fn create(&self, collection: CreateCollectionRequest) {
        let columns = collection
            .columns
            .iter()
            .map(|c| format!("{} {}", c.name, c.data_type))
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

        let tx = self.db.begin().await.unwrap();

        let res = sqlx::query(&sql).execute(&self.db).await;

        match res {
            Ok(_) => {
                let columns_json = serde_json::to_string(&collection.columns).unwrap();
                let indexs_json = serde_json::to_string(
                    &collection
                        .columns
                        .iter()
                        .filter(|c| c.index)
                        .cloned()
                        .collect::<Vec<_>>(),
                )
                .unwrap();

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

                let qs = sqlx::query(&sql).execute(&self.db).await;

                println!("Collection created {:?}", qs);
            }
            Err(err) => {
                tx.rollback().await.unwrap();
                eprint!("Collection Creation Error: {:?}", err)
            }
        }
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Collection, sqlx::Error> {
        sqlx::query_as::<_, Collection>("SELECT * FROM collections WHERE name = ?")
            .bind(name)
            .fetch_one(&self.db)
            .await
    }

    pub async fn list(&self, page: u64, per_page: u64) -> Option<CollectionListResponse> {
        let q = r#"SELECT * FROM _collections LIMIT ? OFFSET ?"#;

        let offset = (page - 1) * per_page;
        let result = sqlx::query_as::<_, Collection>(q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await;

        let items = match result {
            Ok(items) => items,
            Err(e) => {
                eprintln!("Error: can't fetch records {:?}", e);
                return Some(CollectionListResponse {
                    items: vec![],
                    total: 0,
                    page,
                    per_page,
                });
            }
        };

        let total = items.len();

        return Some(CollectionListResponse {
            items,
            total: total as u64,
            page,
            per_page,
        });
    }
}
