use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Pool<Sqlite>,
}
