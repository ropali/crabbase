mod common;

use axum::http::StatusCode;
use common::{TestApp, expect, insert_superuser, login};
use serde_json::json;

async fn admin_token(app: &TestApp) -> String {
    insert_superuser(&app.db.pool, "admin@example.com", "secret123").await;
    login(app, "_superusers", "admin@example.com", "secret123").await
}

#[tokio::test]
async fn backups_require_admin() {
    let app = TestApp::new("backups_auth").await;

    let res = app.post_json("/api/backups", json!({}), None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = app.get("/api/backups").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_list_and_conflict_download() {
    let app = TestApp::new("backups_flow").await;
    let token = admin_token(&app).await;

    let res = app.post_json("/api/backups", json!({}), Some(&token)).await;
    let body = expect(res, StatusCode::ACCEPTED).await;
    let backup_id = body["backup"]["id"]
        .as_str()
        .expect("backup id")
        .to_string();
    let job_id = body["jobId"].as_str().expect("job id").to_string();

    let row: (String, String) = sqlx::query_as(
        "SELECT b.status, j.kind FROM _backups b JOIN _jobs j ON j.id = b.job_id WHERE b.id = $1",
    )
    .bind(uuid::Uuid::parse_str(&backup_id).unwrap())
    .fetch_one(&app.db.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "pending");
    assert_eq!(row.1, "backup.create");

    let _: uuid::Uuid = uuid::Uuid::parse_str(&job_id).unwrap();

    let res = app.get_auth("/api/backups", &token).await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["status"], "pending");

    let res = app
        .get_auth(&format!("/api/backups/{backup_id}/download"), &token)
        .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn cron_validation_and_run_conflict() {
    let app = TestApp::new("crons_flow").await;
    let token = admin_token(&app).await;

    let res = app.get_auth("/api/crons", &token).await;
    let body = expect(res, StatusCode::OK).await;
    let crons = body.as_array().expect("cron list");
    let backup_cron = crons
        .iter()
        .find(|c| c["name"] == "db.backup")
        .expect("db.backup seed");

    let id = backup_cron["id"].as_str().unwrap();

    let res = app
        .patch_json(
            &format!("/api/crons/{id}"),
            json!({ "schedule": "not a cron" }),
            &token,
        )
        .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = app
        .patch_json(
            &format!("/api/crons/{id}"),
            json!({ "schedule": "0 3 * * *", "enabled": true }),
            &token,
        )
        .await;
    expect(res, StatusCode::OK).await;

    let first = app
        .post_json(&format!("/api/crons/{id}/run"), json!({}), Some(&token))
        .await;
    expect(first, StatusCode::ACCEPTED).await;

    let second = app
        .post_json(&format!("/api/crons/{id}/run"), json!({}), Some(&token))
        .await;
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn backup_and_job_settings_are_stored_in_settings_table() {
    let app = TestApp::new("jobs_settings").await;
    let token = admin_token(&app).await;

    let res = app.get_auth("/api/settings/backups", &token).await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["enabled"], true);
    assert_eq!(body["pgDumpPath"], "pg_dump");

    let res = app
        .post_json(
            "/api/settings/backups",
            json!({
                "enabled": true,
                "dir": "/tmp/crabbase-test-backups",
                "pgDumpPath": "/usr/bin/pg_dump",
                "retentionDays": 7,
                "maxConcurrent": 1,
                "dumpSchema": false,
                "schema": "crabbase"
            }),
            Some(&token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let res = app.get_auth("/api/settings/backups", &token).await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["retentionDays"], 7);
    assert_eq!(body["dir"], "/tmp/crabbase-test-backups");

    let res = app
        .post_json(
            "/api/settings/jobs",
            json!({
                "enabled": true,
                "worker": false,
                "scheduler": false,
                "tickSeconds": 15,
                "leaseSeconds": 60,
                "concurrency": 2
            }),
            Some(&token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let res = app.get_auth("/api/settings/jobs", &token).await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["tickSeconds"], 15);
    assert_eq!(body["worker"], false);
}
