use sqlx::{Pool, Sqlite, SqlitePool, sqlite::SqliteConnectOptions};
use tracing::info;

pub async fn pool() -> Result<Pool<Sqlite>, sqlx::Error> {
    let opts = SqliteConnectOptions::new()
        .filename("app.db")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(opts).await?;

    info!("Connected to SQLite!");

    Ok(pool)
}
