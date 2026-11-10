#![cfg(unix)]

use std::path::{Path, PathBuf};

use crabbase_jobs::handlers::backup::BackupCreateHandler;
use crabbase_jobs::{
    BackupRepository, BackupSettings, DatabaseParts, JobContext, JobHandler, JobRepository, NewJob,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, postgres::PgConnectOptions};

async fn test_pool(schema: &str) -> PgPool {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

    let init = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("connect to postgres");

    let schema = format!("{schema}_{}", uuid::Uuid::new_v4().simple());
    let ident = format!("\"{schema}\"");
    sqlx::query(&format!("CREATE SCHEMA {ident}"))
        .execute(&init)
        .await
        .expect("create schema");
    init.close().await;

    let mut options: PgConnectOptions = db_url.parse().unwrap();
    options = options.options([("search_path", schema.as_str())]);

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("connect scoped pool");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    pool
}

fn write_fake_pg_dump(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("fake_pg_dump");
    std::fs::write(
        &path,
        "#!/bin/sh\nout=\"\"\nwhile [ \"$#\" -gt 0 ]; do case \"$1\" in --file) shift; out=\"$1\" ;; esac; shift; done\nprintf 'fake-custom-dump' > \"$out\"\n",
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[tokio::test]
async fn backup_handler_writes_file_and_catalog_row() {
    let pool = test_pool("backup_handler").await;
    let dir = std::env::temp_dir().join(format!(
        "crabbase-backup-test-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let script = write_fake_pg_dump(&dir);
    let cfg = BackupSettings {
        pg_dump_path: script.to_string_lossy().into(),
        dir: dir.clone(),
        ..BackupSettings::default()
    };

    sqlx::query(
        "INSERT INTO _settings (name, value) VALUES ('backups', $1)
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(serde_json::to_string(&cfg).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    let ctx = JobContext {
        pool: pool.clone(),
        database: DatabaseParts {
            host: "localhost".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "crabbase".to_string(),
        },
    };

    let backup = BackupRepository::new(pool.clone())
        .create_pending("manual")
        .await
        .unwrap();

    let job = JobRepository::new(pool.clone())
        .enqueue(NewJob {
            payload: serde_json::json!({ "backupId": backup.id }),
            ..NewJob::new("backup.create")
        })
        .await
        .unwrap()
        .unwrap();

    BackupCreateHandler.run(&ctx, &job).await.unwrap();

    let saved = BackupRepository::new(pool)
        .get(backup.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.status, "completed");
    assert_eq!(saved.size_bytes, Some(16));
    assert!(dir.join(saved.filename).exists());

    let _ = std::fs::remove_dir_all(&dir);
}
