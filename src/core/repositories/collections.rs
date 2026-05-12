use sqlx::{Pool, Sqlite};

use crate::api::models::{Collection, CollectionListResponse};

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

    pub async fn create(&self, name: String) {
        let query = r#"INSERT INTO collections (name, description) VALUES ($1, $2)
            "#;

        let r = sqlx::query(query)
            .bind(name)
            .bind("Test")
            .execute(&self.db)
            .await;

        println!("Collection created {:?}", r)
    }

    pub async fn list(&self, page: u64, per_page: u64) -> Option<CollectionListResponse> {
        let q = r#"SELECT * FROM collections LIMIT ? OFFSET ?"#;

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
