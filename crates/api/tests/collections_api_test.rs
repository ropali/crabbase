//! HTTP API integration tests for the Collections endpoint.
//!
//! Tests cover:
//!   - CRUD lifecycle over HTTP (create, list, get, update, delete)
//!   - Admin-only enforcement (no token → 401/403)
//!   - Validation errors (invalid names, missing columns, duplicate names 409)
//!   - Schema migration through the API (add / drop columns)
//!   - Collection truncation (`POST /api/collections/:name/truncate`)
//!   - MVP Target Features & Gaps:
//!     - Auto-inject system auth columns on `collection_type = "auth"` (GAP: MVP_ROADMAP.md §2.3 / §Phase 2)
//!     - Collection pagination total count (GAP / BUG: MVP_ROADMAP.md §2.1 Bug: collections.rs:185)

mod common;

use axum::http::StatusCode;
use common::{TestApp, create_collection, expect, insert_superuser, login, number_col, text_col};
use serde_json::json;

// ─── Helper: bootstrap a superuser and get a token ───────────────────────────

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("col_api").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;
    (app, token)
}

// ─── Create collection ────────────────────────────────────────────────────────

/// Creating a collection requires a superuser token.
#[tokio::test]
async fn test_create_collection_requires_auth() {
    let app = TestApp::new("col_api_no_auth").await;

    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "posts",
                "columns": [{ "name": "title", "type": "text" }]
            }),
            None, // no token
        )
        .await;

    // Should be 401 Unauthorized or 403 Forbidden
    assert!(
        res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::FORBIDDEN,
        "expected 401 or 403, got {}",
        res.status()
    );
}

/// Happy path: create a collection and verify the response shape.
#[tokio::test]
async fn test_create_collection_success() {
    let (app, token) = setup().await;

    let body = create_collection(
        &app,
        "articles",
        vec![text_col("title"), number_col("views")],
        &token,
    )
    .await;

    assert_eq!(body["name"], "articles");
    // fields array should contain our two columns
    let fields = body["fields"].as_array().expect("fields array");
    let names: Vec<&str> = fields.iter().filter_map(|f| f["name"].as_str()).collect();
    assert!(names.contains(&"title"), "title column missing");
    assert!(names.contains(&"views"), "views column missing");
}

/// Collection names must follow identifier rules (no spaces, no hyphens, etc.)
#[tokio::test]
async fn test_create_collection_invalid_name_rejected() {
    let (app, token) = setup().await;

    // Name starts with a digit — invalid identifier
    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "1invalid",
                "columns": [{ "name": "title", "type": "text" }]
            }),
            Some(&token),
        )
        .await;

    let body = expect(res, StatusCode::BAD_REQUEST).await;
    assert!(
        body["message"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("identifier")
            || body["code"].as_str().unwrap_or("") == "VALIDATION_ERROR",
        "expected validation error, got: {body}"
    );
}

/// Creating a collection with a name that already exists must return 409 Conflict.
#[tokio::test]
async fn test_create_collection_duplicate_name_conflict_409() {
    let (app, token) = setup().await;

    create_collection(&app, "dup_col", vec![text_col("title")], &token).await;

    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "dup_col",
                "columns": [{ "name": "title", "type": "text" }]
            }),
            Some(&token),
        )
        .await;

    // Should return 409 Conflict
    let status = res.status();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "expected 409 CONFLICT for duplicate collection name, got {}",
        status
    );
}

/// Creating a collection without columns must fail.
#[tokio::test]
async fn test_create_collection_empty_columns_rejected() {
    let (app, token) = setup().await;

    let res = app
        .post_json(
            "/api/collections",
            json!({ "name": "empty_col", "columns": [] }),
            Some(&token),
        )
        .await;

    assert!(
        res.status().is_client_error() || res.status().is_server_error(),
        "expected non-2xx, got {}",
        res.status()
    );
}

/// [MVP GAP / Phase 2.1]: Auto-inject auth system columns for auth collections.
/// When `collection_type = "auth"`, backend must automatically inject `email`, `emailVisibility`,
/// `verified`, `token_key`, and `password` columns.
///
/// Ref: MVP_ROADMAP.md §2.3 and §Phase 2.1
#[tokio::test]
async fn test_create_auth_collection_auto_injects_system_columns() {
    let (app, token) = setup().await;

    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "members",
                "collection_type": "auth",
                "columns": [
                    { "name": "display_name", "type": "text" }
                ]
            }),
            Some(&token),
        )
        .await;

    let body = expect(res, StatusCode::OK).await;
    let fields = body["fields"].as_array().expect("fields array in response");
    let names: Vec<&str> = fields.iter().filter_map(|f| f["name"].as_str()).collect();

    // FAIL REASON IF NOT IMPLEMENTED: Backend does not auto-inject auth fields when collection_type="auth".
    // Currently only Admin UI JavaScript client-side injected these fields.
    assert!(
        names.contains(&"email"),
        "MVP Gap: 'email' column must be auto-injected for auth collection. Got: {:?}",
        names
    );
    assert!(
        names.contains(&"password"),
        "MVP Gap: 'password' column must be auto-injected for auth collection. Got: {:?}",
        names
    );
    assert!(
        names.contains(&"token_key"),
        "MVP Gap: 'token_key' column must be auto-injected for auth collection. Got: {:?}",
        names
    );
}

// ─── List collections ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_collections_requires_auth() {
    let app = TestApp::new("col_api_list_no_auth").await;

    let res = app.get("/api/collections").await;
    assert!(
        res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::FORBIDDEN,
        "expected 401/403, got {}",
        res.status()
    );
}

#[tokio::test]
async fn test_list_collections_returns_created_collection() {
    let (app, token) = setup().await;

    create_collection(&app, "blog_posts", vec![text_col("content")], &token).await;

    let res = app.get_auth("/api/collections", &token).await;
    let body = expect(res, StatusCode::OK).await;

    let items = body["items"].as_array().expect("items array");
    assert!(
        items.iter().any(|c| c["name"] == "blog_posts"),
        "created collection missing from list: {body}"
    );
}

/// [MVP BUG / §2.1]: Collection List Pagination Total Count.
/// `CollectionRepository::list` currently sets `total: result.len()` instead of `COUNT(*)`.
/// If there are 3 collections and `per_page=1`, `total` must be 3 (or total in DB), NOT 1.
///
/// Ref: MVP_ROADMAP.md §2.1 Bug (crates/db/src/repositories/collections.rs:185)
#[tokio::test]
async fn test_collection_list_pagination_total_count() {
    let (app, token) = setup().await;

    create_collection(&app, "col_page_1", vec![text_col("a")], &token).await;
    create_collection(&app, "col_page_2", vec![text_col("b")], &token).await;
    create_collection(&app, "col_page_3", vec![text_col("c")], &token).await;

    // Fetch page 1 with per_page = 1
    let res = app
        .get_auth("/api/collections?page=1&per_page=1", &token)
        .await;
    let body = expect(res, StatusCode::OK).await;

    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1, "per_page=1 should return 1 item");

    let total = body["total"].as_i64().expect("total count");
    // FAIL REASON IF NOT FIXED: total count returns 1 (result.len()) instead of total collections (>=3).
    assert!(
        total >= 3,
        "MVP Bug: collection list total should reflect total count in database (>=3), but got {}",
        total
    );
}

// ─── Get single collection ────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_collection_by_name() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "products",
        vec![text_col("sku"), number_col("price")],
        &token,
    )
    .await;

    let res = app.get_auth("/api/collections/products", &token).await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["name"], "products");
}

#[tokio::test]
async fn test_get_nonexistent_collection_returns_404() {
    let (app, token) = setup().await;

    let res = app
        .get_auth("/api/collections/ghost_collection", &token)
        .await;
    expect(res, StatusCode::NOT_FOUND).await;
}

// ─── Update collection ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_collection_add_column() {
    let (app, token) = setup().await;

    create_collection(&app, "tasks", vec![text_col("title")], &token).await;

    // Patch to add a `done` (bool) column
    let res = app
        .patch_json(
            "/api/collections/tasks",
            json!({
                "columns": [
                    { "name": "title", "type": "text", "index": false },
                    { "name": "done",  "type": "bool", "index": false }
                ]
            }),
            &token,
        )
        .await;

    let body = expect(res, StatusCode::OK).await;
    let fields = body["fields"].as_array().expect("fields array");
    let names: Vec<&str> = fields.iter().filter_map(|f| f["name"].as_str()).collect();
    assert!(names.contains(&"done"), "done column not added: {names:?}");
}

// ─── Delete collection ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_collection_success() {
    let (app, token) = setup().await;

    create_collection(&app, "to_delete", vec![text_col("x")], &token).await;

    let del_res = app.delete_auth("/api/collections/to_delete", &token).await;
    expect(del_res, StatusCode::OK).await;

    // Confirm it's gone
    let get_res = app.get_auth("/api/collections/to_delete", &token).await;
    expect(get_res, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn test_delete_nonexistent_collection_returns_404() {
    let (app, token) = setup().await;

    let res = app
        .delete_auth("/api/collections/never_existed", &token)
        .await;
    expect(res, StatusCode::NOT_FOUND).await;
}

// ─── Truncate collection ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_truncate_collection_requires_admin() {
    let app = TestApp::new("col_api_trunc_no_auth").await;

    let res = app
        .post_json("/api/collections/items/truncate", json!({}), None)
        .await;

    assert!(
        res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::FORBIDDEN,
        "expected 401/403 for unauthenticated truncate, got {}",
        res.status()
    );
}

#[tokio::test]
async fn test_truncate_collection_removes_all_records() {
    let (app, token) = setup().await;

    create_collection(&app, "temp_data", vec![text_col("val")], &token).await;

    // Insert 2 records
    app.post_json(
        "/api/collections/temp_data/records",
        json!({ "data": { "val": "row1" } }),
        None,
    )
    .await;
    app.post_json(
        "/api/collections/temp_data/records",
        json!({ "data": { "val": "row2" } }),
        None,
    )
    .await;

    // Verify 2 records exist
    let list_res = app.get("/api/collections/temp_data/records").await;
    let list_body = expect(list_res, StatusCode::OK).await;
    assert_eq!(list_body["items"].as_array().unwrap().len(), 2);

    // Call truncate endpoint
    let trunc_res = app
        .post_json(
            "/api/collections/temp_data/truncate",
            json!({}),
            Some(&token),
        )
        .await;
    expect(trunc_res, StatusCode::OK).await;

    // Verify collection is now empty
    let empty_res = app.get("/api/collections/temp_data/records").await;
    let empty_body = expect(empty_res, StatusCode::OK).await;
    assert_eq!(empty_body["items"].as_array().unwrap().len(), 0);
}
