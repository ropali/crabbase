use crabbase_auth::{
    repositories::{auth::AuthRepository, otp::OtpRepository},
    service::AuthService,
};
use sqlx::{Pool, Postgres};

use crabbase_db::repositories::{
    auth::UserRepository, collections::CollectionRepository, records::RecordsRepository,
    settings::SettingsRepository,
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

    pub fn user_repo(&self) -> UserRepository {
        UserRepository::new(self.db.clone())
    }

    pub fn auth_repo(&self) -> AuthRepository {
        AuthRepository::new(self.db.clone())
    }

    pub fn settings_repo(&self) -> SettingsRepository {
        SettingsRepository::new(self.db.clone())
    }

    pub fn otp_repo(&self) -> OtpRepository {
        OtpRepository::new(self.db.clone())
    }

    pub fn auth_service(&self) -> AuthService {
        AuthService::new(
            self.user_repo(),
            self.auth_repo(),
            self.settings_repo(),
            self.otp_repo(),
        )
    }
}
