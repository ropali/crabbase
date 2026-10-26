//! Protected file download tests.
//!
//! Covers the PocketBase-style file token flow described in
//! `PROTECTED_FILE_ACCESS.md`: public/protected matrices, token issuance,
//! `?download=1`, cross-collection access, and tampering.

mod common;

use axum::http::{StatusCode, header};
use common::{TestApp, create_collection, expect, file_col, insert_superuser, login, text_col};
use serde_json::json;

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("file_access").await;
    insert_superuser(&app.db.pool, "admin@test.com", "secret").await;
    let token = login(&app, "_superusers", "admin@test.com", "secret").await;
    (app, token)
}

/// Uploads one file to `collection` and returns `(file_id, file_key)`.
async fn upload(
    app: &TestApp,
    collection: &str,
    token: &str,
    name: &str,
    bytes: &[u8],
) -> (String, String) {
    let res = app
        .post_multipart(
            &format!("/api/collections/{collection}/records/upload"),
            name,
            "text/plain",
            bytes,
            Some(token),
        )
        .await;
    let body = expect(res, StatusCode::OK).await;
    (
        body["files"][0]["id"]
            .as_str()
            .expect("file id")
            .to_string(),
        body["files"][0]["key"]
            .as_str()
            .expect("file key")
            .to_string(),
    )
}

#[tokio::test]
async fn file_token_requires_authentication() {
    let (app, _admin_token) = setup().await;

    let res = app.post_json("/api/files/token", json!({}), None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_files_require_a_file_token() {
    let (app, admin_token) = setup().await;

    // No rules supplied on create => view_rule/create_rule are NULL => admin-only.
    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "protected_docs",
                "columns": [text_col("title"), file_col("attachment")]
            }),
            Some(&admin_token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let (file_id, file_key) = upload(
        &app,
        "protected_docs",
        &admin_token,
        "secret.txt",
        b"top secret",
    )
    .await;

    // 1. No token -> 403 on both routes.
    let res = app.get(&format!("/api/files/{file_key}")).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = app
        .get(&format!("/api/collections/protected_docs/files/{file_id}"))
        .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Issue a file token as the authenticated user.
    let res = app
        .post_json("/api/files/token", json!({}), Some(&admin_token))
        .await;
    let body = expect(res, StatusCode::OK).await;
    let file_token = body["token"].as_str().expect("file token").to_string();
    assert_eq!(body["expiresIn"], json!(180));

    // A file token is not a request auth token: it must not authenticate the
    // records API (the collection is admin-only anyway, so this is a 403).
    let res = app
        .get(&format!(
            "/api/collections/protected_docs/records?token={file_token}"
        ))
        .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. The same token works on both download routes.
    let res = app
        .get(&format!("/api/files/{file_key}?token={file_token}"))
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .get(&format!(
            "/api/collections/protected_docs/files/{file_id}?token={file_token}"
        ))
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    // 4. download=1 switches to an attachment.
    let res = app
        .get(&format!(
            "/api/files/{file_key}?download=1&token={file_token}"
        ))
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let disposition = res.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .expect("content-disposition");
    assert!(disposition.starts_with("attachment;"), "{disposition}");

    // 5. A file cannot be fetched through another collection's path.
    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "other_docs",
                "columns": [text_col("title"), file_col("attachment")]
            }),
            Some(&admin_token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let res = app
        .get(&format!(
            "/api/collections/other_docs/files/{file_id}?token={file_token}"
        ))
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 6. A tampered token is rejected.
    let res = app
        .get(&format!("/api/files/{file_key}?token={file_token}x"))
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn public_files_stay_public() {
    let (app, admin_token) = setup().await;

    // The helper patches all rules to "" => public.
    create_collection(
        &app,
        "public_docs",
        vec![text_col("title"), file_col("attachment")],
        &admin_token,
    )
    .await;

    let (_file_id, key) = upload(&app, "public_docs", &admin_token, "public.txt", b"public").await;

    let res = app.get(&format!("/api/files/{key}")).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_bearer_auth_can_download_protected_files() {
    let (app, admin_token) = setup().await;

    app.post_json(
        "/api/collections",
        json!({
            "name": "admin_docs",
            "columns": [text_col("title"), file_col("attachment")]
        }),
        Some(&admin_token),
    )
    .await;

    let (_file_id, key) =
        upload(&app, "admin_docs", &admin_token, "admin.txt", b"admin only").await;

    let res = app
        .get_auth(&format!("/api/files/{key}"), &admin_token)
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}
