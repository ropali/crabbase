use sqlx::{Any, Pool, any::AnyPoolOptions};
use tracing::info;

pub async fn pool() -> Result<Pool<Any>, sqlx::Error> {
    // 1. Mandatory driver registry initialization for Any
    sqlx::any::install_default_drivers();

    // 2. Configure SQLite parameters strictly via the generic connection URL.
    // SQLx's SQLite driver interprets query parameters matching native settings.
    // - `mode=rwc` grants read, write, and create capabilities.
    // - `journal_mode=wal` activates Write-Ahead Logging.
    let connection_str = "sqlite://app.db?mode=rwc";

    // 3. Connect using the abstract pool architecture directly
    let pool = AnyPoolOptions::new()
        .max_connections(5) // Tune connection constraints for your environment
        .connect(connection_str)
        .await?;

    info!("Connected to database via Any driver architecture successfully!");

    Ok(pool)
}
