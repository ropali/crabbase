//! API Rule access-control matrix tests.
//!
//! These tests verify the **5-rule security model**:
//!   - `list_rule = null`  → admin-only (unauthenticated must get 403 Forbidden)
//!   - `list_rule = ""`    → public (unauthenticated allowed)
//!   - `list_rule = expr`  → row-level filter applied
//!   - `view_rule`  enforced on GET /records/:id (null → 403)
//!   - `create_rule` enforced on POST /records (null → 403)
//!   - `update_rule` enforced on PATCH /records/:id (null → 403)
//!   - `delete_rule` enforced on DELETE /records/:id (null → 403)
//!   - `@request.data.*` context evaluation on create/update rules
//!
//! Any test asserting an un-implemented rule enforcement will STRICTLY FAIL
//! to highlight remaining MVP security requirements.
//! Ref: MVP_ROADMAP.md §2.2 / §Phase 1.4

mod common;

use axum::http::StatusCode;
use common::{TestApp, create_record_api, expect, file_col, insert_superuser, login, text_col};
use serde_json::json;

// ─── Setup helpers ────────────────────────────────────────────────────────────

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("rules").await;
    insert_superuser(&app.db.pool, "admin@test.com", "secret").await;
    let token = login(&app, "_superusers", "admin@test.com", "secret").await;
    (app, token)
}

/// Create a collection and patch its rules.
async fn create_collection_with_rules(
    app: &TestApp,
    name: &str,
    list_rule: serde_json::Value,
    view_rule: serde_json::Value,
    create_rule: serde_json::Value,
    update_rule: serde_json::Value,
    delete_rule: serde_json::Value,
    token: &str,
) {
    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": name,
                "columns": [
                    text_col("content"),
                    text_col("status")
                ]
            }),
            Some(token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let patch_res = app
        .patch_json(
            &format!("/api/collections/{name}"),
            json!({
                "list_rule":   list_rule,
                "view_rule":   view_rule,
                "create_rule": create_rule,
                "update_rule": update_rule,
                "delete_rule": delete_rule
            }),
            token,
        )
        .await;
    expect(patch_res, StatusCode::OK).await;
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. LIST RULE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// `list_rule = ""` (empty string) → public access allowed.
#[tokio::test]
async fn test_list_rule_empty_string_is_public() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "public_col",
        json!(""), // public
        json!(null),
        json!(""), // create is public to seed
        json!(null),
        json!(null),
        &token,
    )
    .await;

    create_record_api(
        &app,
        "public_col",
        json!({ "content": "visible to all", "status": "active" }),
    )
    .await;

    let res = app.get("/api/collections/public_col/records").await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "public collection returned no items");
}

/// `list_rule = null` MUST require superuser authentication (403 Forbidden for public/unauthenticated).
#[tokio::test]
async fn test_list_rule_null_requires_admin() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "admin_only_list",
        json!(null), // null -> Admin-Only
        json!(null),
        json!(""), // create is public to seed
        json!(null),
        json!(null),
        &token,
    )
    .await;

    create_record_api(
        &app,
        "admin_only_list",
        json!({ "content": "secret", "status": "private" }),
    )
    .await;

    // 1. Unauthenticated request MUST get 403 Forbidden
    let res = app.get("/api/collections/admin_only_list/records").await;
    let status = res.status();

    // FAIL REASON IF NOT FIXED: Crabbase currently executes `SELECT * FROM table` with no WHERE clause
    // when `list_rule` is None, making it public instead of Admin-Only (403).
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "MVP Security Bug: list_rule=null must return 403 FORBIDDEN for unauthenticated users, but got {}",
        status
    );

    // 2. Authenticated superuser MUST get 200 OK
    let authed_res = app
        .get_auth("/api/collections/admin_only_list/records", &token)
        .await;
    let body = expect(authed_res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items");
    assert!(!items.is_empty(), "Admin should be able to view records");
}

/// `list_rule` expression filters rows correctly.
#[tokio::test]
async fn test_list_rule_expression_filters_rows() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "filtered_posts",
        json!("status = 'published'"),
        json!(null),
        json!(""), // create is public to seed
        json!(null),
        json!(null),
        &token,
    )
    .await;

    create_record_api(
        &app,
        "filtered_posts",
        json!({ "content": "Post 1", "status": "published" }),
    )
    .await;
    create_record_api(
        &app,
        "filtered_posts",
        json!({ "content": "Post 2", "status": "draft" }),
    )
    .await;

    let res = app.get("/api/collections/filtered_posts/records").await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items");

    assert_eq!(items.len(), 1, "only 1 published record should be returned");
    assert_eq!(items[0]["data"]["content"], "Post 1");
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. VIEW RULE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// `view_rule = null` must reject unauthenticated `GET /records/:id` with 403 Forbidden.
///
#[tokio::test]
async fn test_view_rule_null_blocks_unauthenticated_get() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "secret_docs",
        json!(""),   // list is public
        json!(null), // view is Admin-only
        json!(""),   // create is public to seed
        json!(null),
        json!(null),
        &token,
    )
    .await;

    let created = create_record_api(
        &app,
        "secret_docs",
        json!({ "content": "top secret", "status": "confidential" }),
    )
    .await;
    let id = created["id"].as_str().expect("id");

    // 1. Unauthenticated GET by ID MUST be rejected with 403
    let res = app
        .get(&format!("/api/collections/secret_docs/records/{id}"))
        .await;
    let status = res.status();

    // FAIL REASON IF NOT IMPLEMENTED: `view_rule` is stored but not evaluated in `get_record`.
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "view_rule=null must return 403 FORBIDDEN for unauthenticated GET /records/:id, but got {}",
        status
    );

    // 2. Admin GET by ID MUST succeed
    let admin_res = app
        .get_auth(
            &format!("/api/collections/secret_docs/records/{id}"),
            &token,
        )
        .await;
    expect(admin_res, StatusCode::OK).await;
}

/// [MVP GAP / Phase 1.4]: `view_rule = "status = 'public'"` allows reading public records,
/// rejects draft records for unauthenticated requests.
#[tokio::test]
async fn test_view_rule_expression_enforced() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "articles_view",
        json!(""),
        json!("status = 'public'"), // only public status can be viewed
        json!(""),                  // create is public to seed
        json!(null),
        json!(null),
        &token,
    )
    .await;

    let pub_rec = create_record_api(
        &app,
        "articles_view",
        json!({ "content": "Pub", "status": "public" }),
    )
    .await;
    let pub_id = pub_rec["id"].as_str().unwrap();

    let draft_rec = create_record_api(
        &app,
        "articles_view",
        json!({ "content": "Draft", "status": "draft" }),
    )
    .await;
    let draft_id = draft_rec["id"].as_str().unwrap();

    // Public record: 200 OK
    let pub_res = app
        .get(&format!("/api/collections/articles_view/records/{pub_id}"))
        .await;
    expect(pub_res, StatusCode::OK).await;

    // Draft record: 403 or 404
    let draft_res = app
        .get(&format!(
            "/api/collections/articles_view/records/{draft_id}"
        ))
        .await;
    let status = draft_res.status();

    // FAIL REASON IF NOT IMPLEMENTED: `view_rule` expression is not evaluated.
    assert!(
        status == StatusCode::FORBIDDEN || status == StatusCode::NOT_FOUND,
        "view_rule expression should block viewing draft records, got {}",
        status
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. CREATE RULE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// `create_rule = null` must reject unauthenticated record creation with 403.
///
#[tokio::test]
async fn test_create_rule_null_blocks_unauthenticated_create() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "admin_posts",
        json!(""),
        json!(""),
        json!(null), // create is Admin-only
        json!(null),
        json!(null),
        &token,
    )
    .await;

    // Unauthenticated POST
    let res = app
        .post_json(
            "/api/collections/admin_posts/records",
            json!({ "data": { "content": "unauthorized attempt", "status": "draft" } }),
            None,
        )
        .await;

    let status = res.status();

    // FAIL REASON IF NOT IMPLEMENTED: `create_rule` is not evaluated in `create_record`.
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "MVP Gap: create_rule=null must return 403 FORBIDDEN for unauthenticated POST /records, but got {}",
        status
    );
}

/// [MVP GAP / Phase 1.4 & §2.2]: `create_rule = "@request.data.status = 'draft'"`
/// evaluates rule against incoming body data.
#[tokio::test]
async fn test_create_rule_with_request_data_context() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "submissions",
        json!(""),
        json!(""),
        json!("@request.data.status = 'draft'"), // only drafts allowed
        json!(null),
        json!(null),
        &token,
    )
    .await;

    // 1. Creating a draft should succeed (200)
    let draft_res = app
        .post_json(
            "/api/collections/submissions/records",
            json!({ "data": { "content": "My Draft", "status": "draft" } }),
            None,
        )
        .await;
    expect(draft_res, StatusCode::OK).await;

    // 2. Creating a published post directly must be rejected with 403
    let pub_res = app
        .post_json(
            "/api/collections/submissions/records",
            json!({ "data": { "content": "Direct Publish", "status": "published" } }),
            None,
        )
        .await;

    let status = pub_res.status();
    // FAIL REASON IF NOT IMPLEMENTED: `@request.data.*` context not supported in rule compiler.
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "MVP Gap: create_rule with @request.data check must reject non-matching payload with 403, got {}",
        status
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. UPDATE RULE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// `update_rule = null` must reject unauthenticated PATCH with 403.
#[tokio::test]
async fn test_update_rule_null_blocks_unauthenticated_update() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "locked_items",
        json!(""),
        json!(""),
        json!(""),   // create is public to seed
        json!(null), // update is Admin-only
        json!(null),
        &token,
    )
    .await;

    let created = create_record_api(
        &app,
        "locked_items",
        json!({ "content": "Initial", "status": "fixed" }),
    )
    .await;
    let id = created["id"].as_str().expect("id");

    // Unauthenticated PATCH attempt
    let patch_res = app
        .patch_json(
            &format!("/api/collections/locked_items/records/{id}"),
            json!({ "data": { "content": "Hacked" } }),
            "", // no token
        )
        .await;

    let status = patch_res.status();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "update_rule=null must return 403 FORBIDDEN for unauthenticated PATCH /records/:id, but got {}",
        status
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. DELETE RULE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// [MVP GAP / Phase 1.4]: `delete_rule = null` must reject unauthenticated DELETE with 403.
#[tokio::test]
async fn test_delete_rule_null_blocks_unauthenticated_delete() {
    let (app, token) = setup().await;

    create_collection_with_rules(
        &app,
        "guarded_trash",
        json!(""),
        json!(""),
        json!(""), // create is public to seed
        json!(null),
        json!(null), // delete is Admin-only
        &token,
    )
    .await;

    let created = create_record_api(
        &app,
        "guarded_trash",
        json!({ "content": "Keep Me", "status": "active" }),
    )
    .await;
    let id = created["id"].as_str().expect("id");

    // Unauthenticated DELETE
    let del_res = app
        .request(
            axum::http::Request::builder()
                .method(axum::http::Method::DELETE)
                .uri(&format!("/api/collections/guarded_trash/records/{id}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;

    let status = del_res.status();
    // FAIL REASON IF NOT IMPLEMENTED: `delete_rule` not evaluated in `delete_record`.
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "MVP Gap: delete_rule=null must return 403 FORBIDDEN for unauthenticated DELETE, but got {}",
        status
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Files API Unauthenticated Access Tests
// ══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_files_api_download_public_delete_admin_only() {
    let (app, token) = setup().await;

    let state = crabbase_api::state::AppState {
        db: app.db.pool.clone(),
    };
    let upload_result = state
        .file_service()
        .upload_bytes("public.txt", "text/plain", b"public file content".to_vec())
        .await
        .expect("upload file bytes");
    let file_key = upload_result.key;

    // Unauthenticated GET /api/files/{file_key} succeeds
    let res = app.get(&format!("/api/files/{file_key}")).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Unauthenticated DELETE /api/files/{file_key} is rejected
    let del_res = app
        .request(
            axum::http::Request::builder()
                .method(axum::http::Method::DELETE)
                .uri(&format!("/api/files/{file_key}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(del_res.status(), StatusCode::UNAUTHORIZED);

    // Admin DELETE /api/files/{file_key} succeeds
    let del_res = app
        .request(
            axum::http::Request::builder()
                .method(axum::http::Method::DELETE)
                .uri(&format!("/api/files/{file_key}"))
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(del_res.status(), StatusCode::NO_CONTENT);
}

/// The download key is caller input. `..` segments (literal or percent-encoded)
/// must never escape the local storage root (`$HOME/crabbase/files/`), which
/// would otherwise let anyone read arbitrary files as `$HOME/...`.
#[tokio::test]
async fn test_file_download_rejects_path_traversal() {
    let app = TestApp::new("files_traversal").await;

    let home = std::env::var("HOME").expect("HOME env");
    let marker_dir = std::path::PathBuf::from(&home).join("crabbase");
    std::fs::create_dir_all(&marker_dir).unwrap();
    let marker = marker_dir.join("traversal-marker.txt");
    std::fs::write(&marker, b"outside the storage root").unwrap();

    for path in [
        "/api/files/../traversal-marker.txt",
        "/api/files/%2e%2e%2ftraversal-marker.txt",
    ] {
        let res = app.get(path).await;
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "traversal via {path} must not serve a file outside the storage root"
        );
        assert!(
            !body
                .windows(b"outside the storage root".len())
                .any(|w| w == b"outside the storage root"),
            "traversal via {path} leaked file contents"
        );
    }

    let _ = std::fs::remove_file(&marker);
}

#[tokio::test]
async fn test_record_upload_enforces_collection_access_rules() {
    let (app, token) = setup().await;

    // 1. Create collection with create_rule = null (Admin-only)
    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "vault_docs",
                "columns": [text_col("title"), file_col("document")]
            }),
            Some(&token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let patch_res = app
        .patch_json(
            "/api/collections/vault_docs",
            json!({
                "list_rule": "",
                "view_rule": null,
                "create_rule": null,
                "update_rule": null,
                "delete_rule": null,
            }),
            &token,
        )
        .await;
    expect(patch_res, StatusCode::OK).await;

    // 2. Unauthenticated upload to /api/collection/vault_docs/record/upload is BLOCKED (403 Forbidden)
    // because caller does not have permission to create record for this collection
    let unauth_upload_res = app
        .post_multipart(
            "/api/collection/vault_docs/record/upload",
            "doc.pdf",
            "application/pdf",
            b"secret content",
            None,
        )
        .await;
    assert_eq!(unauth_upload_res.status(), StatusCode::FORBIDDEN);

    // 3. Admin upload to /api/collection/vault_docs/record/upload SUCCEEDS
    let admin_upload_res = app
        .post_multipart(
            "/api/collection/vault_docs/record/upload",
            "doc.pdf",
            "application/pdf",
            b"secret content",
            Some(&token),
        )
        .await;
    let body = expect(admin_upload_res, StatusCode::OK).await;
    let files = body["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["collectionName"], "vault_docs");
    let file_id = files[0]["id"].as_str().expect("file id").to_string();
    let file_path = format!("/api/collections/vault_docs/files/{file_id}");

    // 4. Admin creates record referencing the uploaded file ID
    let admin_record_res = app
        .post_json(
            "/api/collections/vault_docs/records",
            json!({ "data": { "title": "Admin Doc", "document": file_id } }),
            Some(&token),
        )
        .await;
    let rec_body = expect(admin_record_res, StatusCode::OK).await;
    assert_eq!(rec_body["data"]["document"]["id"], file_id);
    assert_eq!(rec_body["data"]["document"]["originalName"], "doc.pdf");

    // 5. Download via collection-scoped GET /api/collections/vault_docs/files/{file_id}
    // view_rule is null -> unauthenticated download is 403 Forbidden
    let unauth_download = app.get(&file_path).await;
    assert_eq!(unauth_download.status(), StatusCode::FORBIDDEN);

    // Authenticated download as admin succeeds
    let auth_download = app.get_auth(&file_path, &token).await;
    assert_eq!(auth_download.status(), StatusCode::OK);

    // 6. Public collection with create_rule = "" allows unauthenticated upload
    let pub_patch = app
        .patch_json(
            "/api/collections/vault_docs",
            json!({
                "view_rule": "",
                "create_rule": "",
            }),
            &token,
        )
        .await;
    expect(pub_patch, StatusCode::OK).await;

    let pub_upload = app
        .post_multipart(
            "/api/collections/vault_docs/records/upload",
            "public.txt",
            "text/plain",
            b"hello world",
            None,
        )
        .await;
    assert_eq!(pub_upload.status(), StatusCode::OK);
    let pub_body = expect(pub_upload, StatusCode::OK).await;
    let pub_file_id = pub_body["files"][0]["id"].as_str().expect("pub id");

    // Public view_rule allows unauthenticated download
    let pub_download = app
        .get(&format!("/api/collections/vault_docs/files/{pub_file_id}"))
        .await;
    assert_eq!(pub_download.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_record_upload_nonexistent_collection_returns_404() {
    let (app, _) = setup().await;

    let res = app
        .post_multipart(
            "/api/collection/nonexistent_col/record/upload",
            "avatar.png",
            "image/png",
            b"PNG IMAGE BYTES",
            None,
        )
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_collection_files_enforces_access_rules() {
    let (app, admin_token) = setup().await;

    // 1. Create a collection with default null (admin-only) list_rule
    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "secure_vault",
                "columns": [text_col("title"), file_col("doc")]
            }),
            Some(&admin_token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    // Set create_rule to public so upload can succeed with or without admin
    let patch_res = app
        .patch_json(
            "/api/collections/secure_vault",
            json!({
                "create_rule": ""
            }),
            &admin_token,
        )
        .await;
    expect(patch_res, StatusCode::OK).await;

    // 2. Upload file
    let upload_res = app
        .post_multipart(
            "/api/collections/secure_vault/records/upload",
            "secret.pdf",
            "application/pdf",
            b"top secret",
            Some(&admin_token),
        )
        .await;
    assert_eq!(upload_res.status(), StatusCode::OK);

    // 3. Unauthenticated GET /api/collections/secure_vault/files is BLOCKED (403)
    let unauth_list = app.get("/api/collections/secure_vault/files").await;
    assert_eq!(unauth_list.status(), StatusCode::FORBIDDEN);

    // 4. Admin GET /api/collections/secure_vault/files SUCCEEDS
    let admin_list = app
        .get_auth("/api/collections/secure_vault/files", &admin_token)
        .await;
    assert_eq!(admin_list.status(), StatusCode::OK);
    let body = expect(admin_list, StatusCode::OK).await;
    assert_eq!(body["totalItems"], 1);
    assert_eq!(body["items"][0]["collectionName"], "secure_vault");
}
