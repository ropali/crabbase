use crabbase_auth::service::AuthService;
use sqlx::{Pool, Postgres};

use crabbase_db::repositories::{
    auth::AuthRepository, collections::CollectionRepository, records::RecordsRepository,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
}

impl AppState {
    pub fn collection_repo(&self) -> CollectionRepository {
        CollectionRepository::new(self.db.clone())
    }

    pub fn records_repo(&self) -> RecordsRepository {
        RecordsRepository::new(self.db.clone())
    }

    pub fn auth_repo(&self) -> AuthRepository {
        AuthRepository::new(self.db.clone())
    }

    pub fn auth_service(&self) -> AuthService {
        AuthService::new(self.auth_repo())
    }
}
