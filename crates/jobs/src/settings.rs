use std::path::PathBuf;

use crabbase_core::errors::RepositoryError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupSettings {
    pub enabled: bool,
    pub dir: PathBuf,
    pub pg_dump_path: String,
    pub retention_days: i64,
    pub max_concurrent: usize,
    pub dump_schema: bool,
    pub schema: String,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: PathBuf::from("./backups"),
            pg_dump_path: "pg_dump".to_string(),
            retention_days: 14,
            max_concurrent: 1,
            dump_schema: false,
            schema: "crabbase".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct JobsSettings {
    pub enabled: bool,
    pub worker: bool,
    pub scheduler: bool,
    pub tick_seconds: u64,
    pub lease_seconds: u64,
    pub concurrency: usize,
}

impl Default for JobsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            worker: true,
            scheduler: true,
            tick_seconds: 30,
            lease_seconds: 900,
            concurrency: 1,
        }
    }
}

/// The parts of the connection `pg_dump` needs. Kept separate from
/// `DATABASE_URL` so the password is never placed on the command line.
#[derive(Debug, Clone)]
pub struct DatabaseParts {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

pub async fn load_backup_settings(db: &PgPool) -> Result<BackupSettings, RepositoryError> {
    Ok(load_json(db, "backups").await?.unwrap_or_default())
}

pub async fn load_jobs_settings(db: &PgPool) -> Result<JobsSettings, RepositoryError> {
    Ok(load_json(db, "jobs").await?.unwrap_or_default())
}

/// Reads a `_settings` row and parses its JSON. A corrupt value falls back to
/// the caller's default rather than preventing the process from starting.
pub async fn load_json<T: DeserializeOwned>(
    db: &PgPool,
    name: &str,
) -> Result<Option<T>, RepositoryError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM _settings WHERE name = $1")
        .bind(name)
        .fetch_optional(db)
        .await?;

    let Some((raw,)) = row else {
        return Ok(None);
    };

    match serde_json::from_str::<T>(&raw) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            warn!(setting = name, error = %e, "invalid settings JSON; using defaults");
            Ok(None)
        }
    }
}
