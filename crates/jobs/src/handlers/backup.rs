use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use crabbase_db::repositories::{files::FilesRepository, settings::SettingsRepository};
use crabbase_files::{FileStorage, FileUploadService};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncReadExt, process::Command};
use uuid::Uuid;

use crate::{
    handler::{JobContext, JobError, JobHandler},
    models::{Backup, Job},
    repository::BackupRepository,
    settings::{BackupSettings, DatabaseParts, load_backup_settings},
};

pub struct BackupCreateHandler;

#[async_trait::async_trait]
impl JobHandler for BackupCreateHandler {
    fn kind(&self) -> &'static str {
        "backup.create"
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(6 * 60 * 60)
    }

    async fn run(&self, ctx: &JobContext, job: &Job) -> Result<Value, JobError> {
        let cfg = load_backup_settings(&ctx.pool)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        if !cfg.enabled {
            return Err(JobError::Permanent(
                "backups are disabled in the `backups` setting".to_string(),
            ));
        }

        let repo = BackupRepository::new(ctx.pool.clone());

        let backup = match job
            .payload
            .get("backupId")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            Some(id) => match repo
                .mark_running(id)
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?
            {
                Some(backup) => backup,
                None => {
                    // A previous attempt may have completed the artifact and
                    // crashed before recording the job result.
                    if let Some(existing) = repo
                        .get(id)
                        .await
                        .map_err(|e| JobError::Retryable(e.to_string()))?
                    {
                        if existing.status == "completed" {
                            return Ok(completed_result(&existing));
                        }
                    }
                    return Err(JobError::Permanent("backup row not found".to_string()));
                }
            },
            None => repo
                .create_running("scheduled")
                .await
                .map_err(|e| JobError::Retryable(e.to_string()))?,
        };

        let result = dump_and_record(ctx, &repo, &backup, &cfg).await;

        if let Err(err) = &result {
            // Best effort: keep the catalog truthful even when the job will retry.
            let _ = repo.mark_failed(backup.id, &err.to_string()).await;
        }

        result
    }
}

fn completed_result(backup: &Backup) -> Value {
    json!({
        "backupId": backup.id,
        "filename": backup.filename,
        "sizeBytes": backup.size_bytes,
        "checksum": backup.checksum,
    })
}

async fn dump_and_record(
    ctx: &JobContext,
    repo: &BackupRepository,
    backup: &Backup,
    cfg: &BackupSettings,
) -> Result<Value, JobError> {
    let path = dump_database(cfg, &ctx.database, &backup.filename).await?;

    let size = tokio::fs::metadata(&path)
        .await
        .map_err(|e| JobError::Retryable(format!("stat backup: {e}")))?
        .len() as i64;

    let checksum = sha256_file(&path)
        .await
        .map_err(|e| JobError::Retryable(format!("hash backup: {e}")))?;

    let storage_backend = if &cfg.storage_type == "cloud" {
        upload_backup(ctx, &backup.filename, &path).await?
    } else {
        "local".to_string()
    };

    let version = pg_dump_version(&cfg.pg_dump_path).await?;

    repo.mark_completed(backup.id, size, &checksum, &version, &storage_backend)
        .await
        .map_err(|e| JobError::Retryable(e.to_string()))?;

    Ok(json!({
        "backupId": backup.id,
        "filename": backup.filename,
        "sizeBytes": size,
        "checksum": checksum,
    }))
}

async fn resolve_storage(ctx: &JobContext) -> Result<Box<dyn FileStorage>, JobError> {
    let service = FileUploadService::new(
        SettingsRepository::new(ctx.pool.clone()),
        FilesRepository::new(ctx.pool.clone()),
    );

    service
        .resolve_storage()
        .await
        .map_err(|e| JobError::Retryable(format!("resolve backup storage: {e:?}")))
}

async fn upload_backup(
    ctx: &JobContext,
    filename: &str,
    path: &PathBuf,
) -> Result<String, JobError> {
    let storage = resolve_storage(ctx).await?;

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| JobError::Retryable(format!("read backup for upload: {e}")))?;

    storage
        .put_object(
            &crate::repository::backup_object_key(filename),
            bytes,
            "application/octet-stream",
        )
        .await
        .map_err(|e| JobError::Retryable(format!("upload backup: {e}")))?;

    Ok(storage.backend().to_string())
}

async fn dump_database(
    cfg: &BackupSettings,
    db: &DatabaseParts,
    filename: &str,
) -> Result<PathBuf, JobError> {
    tokio::fs::create_dir_all(&cfg.dir)
        .await
        .map_err(|e| JobError::Permanent(format!("create backup dir: {e}")))?;

    let final_path = cfg.dir.join(filename);
    let tmp_path = cfg.dir.join(format!(".{filename}.part"));

    let mut cmd = Command::new(&cfg.pg_dump_path);
    cmd.arg("--format=custom") // one file, compressed, pg_restore-able
        .arg("--compress=9")
        .arg("--no-owner") // portable across roles
        .arg("--no-privileges")
        .arg("--no-password") // never hang waiting for a prompt
        .arg("--lock-wait-timeout=30s")
        .arg("--file")
        .arg(&tmp_path)
        .arg("--host")
        .arg(&db.host)
        .arg("--port")
        .arg(db.port.to_string())
        .arg("--username")
        .arg(&db.user)
        .arg("--dbname")
        .arg(&db.database)
        // Password in the environment, never argv (argv is visible in `ps`).
        .env("PGPASSWORD", &db.password)
        .env("PGCONNECT_TIMEOUT", "30")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true); // cancel/timeout kills the child

    if cfg.dump_schema {
        cmd.arg("--schema").arg(&cfg.schema);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| JobError::Permanent(format!("spawn pg_dump: {e}")))?;

    if !output.status.success() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(JobError::Retryable(format!(
            "pg_dump exited with {}: {}",
            output.status,
            stderr.trim()
        )));
    }

    // Atomic publish: readers never see a partial file.
    tokio::fs::rename(&tmp_path, &final_path)
        .await
        .map_err(|e| JobError::Retryable(format!("publish backup: {e}")))?;

    Ok(final_path)
}

async fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];

    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

async fn pg_dump_version(pg_dump_path: &str) -> Result<String, JobError> {
    let out = Command::new(pg_dump_path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| JobError::Permanent(format!("pg_dump --version: {e}")))?;

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub struct BackupPruneHandler;

#[async_trait::async_trait]
impl JobHandler for BackupPruneHandler {
    fn kind(&self) -> &'static str {
        "backups.prune"
    }

    async fn run(&self, ctx: &JobContext, job: &Job) -> Result<Value, JobError> {
        let cfg = load_backup_settings(&ctx.pool)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        let keep_days = job
            .payload
            .get("keepDays")
            .and_then(Value::as_i64)
            .unwrap_or(cfg.retention_days);

        let repo = BackupRepository::new(ctx.pool.clone());
        let removed = repo
            .prune(keep_days)
            .await
            .map_err(|e| JobError::Retryable(e.to_string()))?;

        let needs_cloud = removed.iter().any(|(_, backend)| backend == "cloud");
        let storage = if needs_cloud {
            Some(resolve_storage(ctx).await?)
        } else {
            None
        };

        for (filename, backend) in &removed {
            let _ = tokio::fs::remove_file(cfg.dir.join(filename)).await;

            if backend == "cloud" {
                if let Some(storage) = &storage {
                    if let Err(e) = storage
                        .delete_object(&crate::repository::backup_object_key(filename))
                        .await
                    {
                        tracing::warn!(backup = %filename, error = %e, "failed to delete cloud backup");
                    }
                }
            }
        }

        let filenames: Vec<&String> = removed.iter().map(|(filename, _)| filename).collect();

        Ok(json!({
            "keepDays": keep_days,
            "removed": removed.len(),
            "filenames": filenames,
        }))
    }
}

pub async fn preflight(pool: &sqlx::PgPool) -> Result<(), JobError> {
    let cfg = load_backup_settings(pool)
        .await
        .map_err(|e| JobError::Retryable(e.to_string()))?;

    if !cfg.enabled {
        return Ok(());
    }

    let out = Command::new(&cfg.pg_dump_path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| {
            JobError::Permanent(format!(
                "pg_dump not found at `{}`: {e}. Install postgresql-client or set pgDumpPath in the `backups` setting.",
                cfg.pg_dump_path
            ))
        })?;

    if !out.status.success() {
        return Err(JobError::Permanent(format!(
            "`{} --version` failed: {}",
            cfg.pg_dump_path,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }

    let client = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let server: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(pool)
        .await
        .map_err(|e| JobError::Retryable(e.to_string()))?;

    if let (Some(client_major), Some(server_major)) =
        (major_version(&client), major_version(&server))
    {
        if client_major < server_major {
            return Err(JobError::Permanent(format!(
                "pg_dump client is too old ({client}) for server {server}. Install a matching postgresql-client major."
            )));
        }
    }

    tracing::info!(client = %client, server = %server, "backup preflight ok");
    Ok(())
}

fn major_version(version: &str) -> Option<u32> {
    let digits: String = version
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}
