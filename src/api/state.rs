use sqlx::{Pool, Sqlite};

use crate::core::repositories::{collections::CollectionRepository, records::RecordsRepository};

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Pool<Sqlite>,
}

impl AppState {
    pub fn collection_repo(&self) -> CollectionRepository {
        CollectionRepository::new(self.db.clone())
    }

    pub fn records_repo(&self) -> RecordsRepository {
        RecordsRepository::new(self.db.clone())
    }
}
