//! HTTP API integration tests for the activity-log endpoints.
//!
//! The rows live in their own schema (standing in for a dedicated log
//! database), reached through `AppState::logs` rather than the application
//! pool — that indirection is the part worth testing.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use crabbase_api::{get_app_routes, state::AppState};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::{Pool, Postgres};
use tower::ServiceExt;

use common::{TestApp, insert_superuser, login};

fn test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string())
}

/// Create a throwaway schema, apply the real log migration into it and return
/// a pool whose `search_path` points there.
async fn logs_pool(app: &TestApp) -> (String, Pool<Postgres>) {
    let schema = format!("logs_{}", uuid::Uuid::new_v4().simple());

    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&app.db.pool)
        .await
        .expect("create log schema");

    let pool = crabbase_db::connection::pool(&test_db_url(), &schema).expect("log pool");

    sqlx::raw_sql(include_str!(
        "../../../migrations-logs/00001_logs_table.up.sql"
    ))
    .execute(&pool)
    .await
    .expect("apply log migration");

    (schema, pool)
}

async fn drop_schema(app: &TestApp, schema: &str) {
    let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
        .execute(&app.db.pool)
        .await;
}

async fn seed(pool: &Pool<Postgres>) {
    sqlx::query(
        "INSERT INTO _logs
            (id, created, level, severity, action, message, method, path, status_code,
             duration_ms, actor_id, actor_collection, outcome, data)
         VALUES
            ('log_info', now() - interval '2 minutes', 2, 'INFO', 'record.create',
             'http request', 'POST', '/api/collections/demo/records', 201, 12.5,
             'user-1', 'users', 'success',
             '{\"route\":\"/api/collections/{name}/records\"}'),
            ('log_error', now() - interval '1 minute', 4, 'ERROR', 'request.failed',
             'request failed with server error', 'GET',
             '/api/collections/demo/records', 500, 3.25, 'user-1', 'users',
             'failure', '{}'),
            ('log_denied', now() - interval '3 minutes', 3, 'WARN', 'auth.login',
             'permission denied', 'POST', '/api/auth/users/login', 401, 8.0,
             'root', '_superusers', 'denied', '{}')",
    )
    .execute(pool)
    .await
    .expect("seed logs");
}

async fn get_json(router: &Router, path: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(Method::GET).uri(path);
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }

    let response = router
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .expect("router oneshot");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();

    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn logs_api_lists_filters_and_aggregates() {
    let app = TestApp::new("logs_api").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;

    let (schema, pool) = logs_pool(&app).await;
    seed(&pool).await;

    let router = get_app_routes(AppState {
        db: app.db.pool.clone(),
        logs: Some(pool.clone()),
    });

    // Unauthenticated access is rejected before the log target is touched.
    let (status, _) = get_json(&router, "/api/logs", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Full page, newest first.
    let (status, body) = get_json(&router, "/api/logs?range=all", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "body: {body:#}");
    assert_eq!(body["enabled"], json!(true));
    assert_eq!(body["total"], json!(3));
    assert_eq!(body["items"].as_array().unwrap().len(), 3);

    let first = &body["items"][0];
    assert_eq!(first["action"], json!("request.failed"));
    assert_eq!(first["severity"], json!("ERROR"));
    // camelCase contract with the wasm client
    assert!(first.get("statusCode").is_some(), "camelCase statusCode");
    assert!(first.get("actorId").is_some(), "camelCase actorId");
    assert!(first.get("created").is_some());

    // Severity filter accepts the UI label, not just the column value.
    let (_, body) = get_json(&router, "/api/logs?range=all&level=Error", Some(&token)).await;
    assert_eq!(body["total"], json!(1));
    assert_eq!(body["items"][0]["id"], json!("log_error"));

    // Superuser-only filter.
    let (_, body) = get_json(&router, "/api/logs?range=all&superusers=true", Some(&token)).await;
    assert_eq!(body["total"], json!(1));
    assert_eq!(body["items"][0]["actorCollection"], json!("_superusers"));

    // Free-text search spans message, path, action and actor.
    let (_, body) = get_json(&router, "/api/logs?range=all&search=denied", Some(&token)).await;
    assert_eq!(body["total"], json!(1));
    let (_, body) = get_json(
        &router,
        "/api/logs?range=all&search=demo/records",
        Some(&token),
    )
    .await;
    assert_eq!(body["total"], json!(2));

    // Pagination
    let (_, body) = get_json(
        &router,
        "/api/logs?range=all&page=2&per_page=2",
        Some(&token),
    )
    .await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["page"], json!(2));

    // Stats: counters plus a zero-filled hourly series for the whole window.
    let (status, stats) = get_json(&router, "/api/logs/stats?range=24h", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "body: {stats:#}");
    assert_eq!(stats["total"], json!(3));
    assert_eq!(stats["errors"], json!(1));
    assert_eq!(stats["warnings"], json!(1));
    assert!(stats["avgDurationMs"].as_f64().unwrap() > 0.0);
    let buckets = stats["buckets"].as_array().unwrap();
    assert!(
        buckets.len() >= 24,
        "quiet buckets should be zero-filled: {}",
        buckets.len()
    );
    let summed: i64 = buckets
        .iter()
        .map(|bucket| bucket["count"].as_i64().unwrap())
        .sum();
    assert_eq!(summed, 3);
    assert!(buckets.iter().any(|bucket| bucket["count"] == json!(0)));

    drop_schema(&app, &schema).await;
    // `TestDb::drop` spawns its cleanup on a runtime that is about to shut
    // down, so drop the application schema explicitly as well.
    drop_schema(&app, &app.db.schema).await;
}

#[tokio::test]
async fn logs_api_reports_disabled_without_a_log_pool() {
    let app = TestApp::new("logs_off").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;

    // TestApp wires `logs: None`, which is what a deployment with
    // `[logging.database] enabled = false` looks like.
    let (status, body) = get_json(&app.router, "/api/logs", Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], json!(false));
    assert_eq!(body["total"], json!(0));
    assert!(body["items"].as_array().unwrap().is_empty());

    let schema = app.db.schema.clone();
    drop_schema(&app, &schema).await;
}
