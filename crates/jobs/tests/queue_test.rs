use std::time::Duration;

use crabbase_jobs::{JobRepository, NewJob};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, postgres::PgConnectOptions};

async fn test_pool(schema: &str) -> PgPool {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

    let init = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("connect to postgres");

    let ident = format!("\"{schema}\"");
    let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {ident} CASCADE"))
        .execute(&init)
        .await;
    sqlx::query(&format!("CREATE SCHEMA {ident}"))
        .execute(&init)
        .await
        .expect("create schema");
    init.close().await;

    let mut options: PgConnectOptions = db_url.parse().unwrap();
    options = options.options([("search_path", schema)]);

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

#[tokio::test]
async fn only_one_worker_claims_a_job() {
    let pool = test_pool("jobs_claim").await;
    let repo = JobRepository::new(pool);

    repo.enqueue(NewJob::new("test.noop")).await.unwrap();

    let lease = Duration::from_secs(60);
    let (a, b) = tokio::join!(repo.claim("worker-a", lease), repo.claim("worker-b", lease),);

    let claimed = a.unwrap().is_some() as u8 + b.unwrap().is_some() as u8;
    assert_eq!(claimed, 1, "a job must be claimed exactly once");
}

#[tokio::test]
async fn dedupe_key_makes_enqueue_idempotent() {
    let pool = test_pool("jobs_dedupe").await;
    let repo = JobRepository::new(pool);

    let job = NewJob {
        dedupe_key: Some("cron:test:2026-10-30T02:00:00Z".to_string()),
        ..NewJob::new("backup.create")
    };

    assert!(repo.enqueue(job.clone()).await.unwrap().is_some());
    assert!(repo.enqueue(job).await.unwrap().is_none());
}

#[tokio::test]
async fn expired_lease_is_reaped() {
    let pool = test_pool("jobs_reap").await;
    let repo = JobRepository::new(pool.clone());

    let job = repo
        .enqueue(NewJob::new("test.noop"))
        .await
        .unwrap()
        .unwrap();
    let claimed = repo
        .claim("dead-worker", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();

    sqlx::query("UPDATE _jobs SET locked_until = now() - interval '1 second' WHERE id = $1")
        .bind(claimed.id)
        .execute(&pool)
        .await
        .unwrap();

    let reaped = repo.reap_expired_leases().await.unwrap();
    assert_eq!(reaped.len(), 1);
    assert_eq!(reaped[0].status, "queued");
    assert_eq!(reaped[0].attempts, job.attempts + 1);
}

#[tokio::test]
async fn due_cron_dispatches_one_job_per_occurrence() {
    let pool = test_pool("jobs_dispatch").await;
    let repo = JobRepository::new(pool.clone());

    sqlx::query(
        "UPDATE _crons
         SET schedule = '*/15 * * * *', enabled = TRUE, next_run = now() - interval '1 minute'
         WHERE name = 'otp.cleanup'",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(repo.dispatch_due(50).await.unwrap(), 1);
    assert_eq!(
        repo.dispatch_due(50).await.unwrap(),
        0,
        "the same occurrence must not dispatch twice"
    );

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM _jobs WHERE kind = 'otp.cleanup'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
