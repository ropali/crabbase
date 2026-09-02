//! HTTP API integration tests for the Records endpoint.
//!
//! Tests cover:
//!   - Full CRUD lifecycle (create, list, get, update, delete)
//!   - Pagination parameters (`page`, `per_page`)
//!   - Password hashing for auth collections
//!   - 404 on unknown collection / record
//!   - MVP Target Features & Gaps:
//!     - `?filter` query parameter evaluation (GAP: MVP_ROADMAP.md §1.1 / §Phase 1.1)
//!     - `?sort` query parameter ordering (GAP: MVP_ROADMAP.md §1.2 / §Phase 1.2)
//!     - `?expand` relation field expansion (GAP: MVP_ROADMAP.md §1.3 / §Phase 3.2)
//!     - `?fields` field selection / column projection (GAP: MVP_ROADMAP.md §2.2)
//!     - `update_record` response structure returning full Record JSON (GAP / BUG: MVP_ROADMAP.md §2.1 Bug: records.rs:72)

mod common;

use axum::http::StatusCode;
use common::{
    TestApp, create_collection, create_record_api, create_record_auth, expect, insert_superuser,
    login, number_col, text_col,
};
use serde_json::json;

// ─── Shared setup ─────────────────────────────────────────────────────────────

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("rec_api").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;
    (app, token)
}

/// Create a simple `posts` collection with title + views columns.
async fn setup_posts(app: &TestApp, token: &str) {
    create_collection(
        app,
        "posts",
        vec![text_col("title"), number_col("views")],
        token,
    )
    .await;
}

// ─── Create record ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_record_returns_record_with_id() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let record = create_record_api(
        &app,
        "posts",
        json!({ "title": "Hello, Crabbase!", "views": 0 }),
    )
    .await;

    assert!(record["id"].as_str().is_some(), "id missing: {record}");
    assert!(record["created"].as_str().is_some(), "created missing");
    assert_eq!(record["data"]["title"], "Hello, Crabbase!");
}

#[tokio::test]
async fn test_create_record_on_unknown_collection_returns_404() {
    let app = TestApp::new("rec_api_unknown_col").await;

    let res = app
        .post_json(
            "/api/collections/ghost_col/records",
            json!({ "data": { "title": "x" } }),
            None,
        )
        .await;

    expect(res, StatusCode::NOT_FOUND).await;
}

// ─── Get record ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_record_by_id() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let created =
        create_record_api(&app, "posts", json!({ "title": "Test Post", "views": 5 })).await;
    let id = created["id"].as_str().expect("id");

    let res = app
        .get(&format!("/api/collections/posts/records/{id}"))
        .await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["id"], id);
    assert_eq!(body["data"]["title"], "Test Post");
}

#[tokio::test]
async fn test_get_nonexistent_record_returns_404() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let res = app
        .get("/api/collections/posts/records/00000000-0000-0000-0000-000000000000")
        .await;
    expect(res, StatusCode::NOT_FOUND).await;
}

// ─── List records ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_records_returns_all_inserted() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    for i in 1..=3 {
        create_record_api(
            &app,
            "posts",
            json!({ "title": format!("Post {i}"), "views": i * 10 }),
        )
        .await;
    }

    let res = app.get("/api/collections/posts/records").await;
    let body = expect(res, StatusCode::OK).await;

    let items = body["items"].as_array().expect("items array");
    assert!(items.len() >= 3, "expected >= 3 items, got {}", items.len());
    assert!(body["page"].as_i64().is_some(), "page field missing");
    assert!(body["perPage"].as_i64().is_some(), "perPage field missing");
    assert!(body["total"].as_i64().is_some(), "total field missing");
}

#[tokio::test]
async fn test_list_records_per_page_limiting() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    for i in 1..=5 {
        create_record_api(
            &app,
            "posts",
            json!({ "title": format!("P{i}"), "views": i }),
        )
        .await;
    }

    let res = app.get("/api/collections/posts/records?per_page=2").await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 2, "per_page=2 should return 2 items");
    assert_eq!(body["perPage"], 2);
}

#[tokio::test]
async fn test_list_records_page_2() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    for i in 1..=4 {
        create_record_api(
            &app,
            "posts",
            json!({ "title": format!("P{i}"), "views": i }),
        )
        .await;
    }

    let res = app
        .get("/api/collections/posts/records?per_page=2&page=2")
        .await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["page"], 2);
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 2);
}

// ─── Update record ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_record_returns_full_record_json() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let created =
        create_record_api(&app, "posts", json!({ "title": "Old Title", "views": 0 })).await;
    let id = created["id"].as_str().expect("id");

    let res = app
        .patch_json(
            &format!("/api/collections/posts/records/{id}"),
            json!({ "data": { "title": "New Title", "views": 42 } }),
            &token,
        )
        .await;

    let body = expect(res, StatusCode::OK).await;

    // FAIL REASON IF NOT FIXED: body is currently `{"details":"record updatedsuccessfully."}`
    // It must return the full updated Record object containing `id` and `data`.
    assert_eq!(
        body["id"].as_str(),
        Some(id),
        "MVP Bug: PATCH response must return full Record with 'id', but got: {:?}",
        body
    );
    assert_eq!(
        body["data"]["title"], "New Title",
        "MVP Bug: PATCH response must include updated 'data' fields"
    );
    assert_eq!(body["data"]["views"], 42);
}

#[tokio::test]
async fn test_update_nonexistent_record_returns_error() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let res = app
        .patch_json(
            "/api/collections/posts/records/00000000-0000-0000-0000-000000000000",
            json!({ "data": { "title": "Ghost" } }),
            &token,
        )
        .await;

    assert!(
        res.status().is_client_error() || res.status().is_server_error(),
        "expected non-2xx for missing record, got {}",
        res.status()
    );
}

// ─── Delete record ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_record_removes_it() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let created =
        create_record_api(&app, "posts", json!({ "title": "To Delete", "views": 0 })).await;
    let id = created["id"].as_str().expect("id");

    let del_res = app
        .delete_auth(&format!("/api/collections/posts/records/{id}"), &token)
        .await;
    expect(del_res, StatusCode::OK).await;

    let get_res = app
        .get(&format!("/api/collections/posts/records/{id}"))
        .await;
    expect(get_res, StatusCode::NOT_FOUND).await;
}

// ─── Auth collection password hashing ────────────────────────────────────────

#[tokio::test]
async fn test_create_auth_record_hashes_password() {
    let (app, token) = setup().await;

    let res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "customers",
                "collection_type": "auth",
                "columns": [
                    { "name": "email",    "type": "email",  "index": true  },
                    { "name": "password", "type": "text",   "index": false },
                    { "name": "token_key","type": "text",   "index": false }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(res, StatusCode::OK).await;

    let record = create_record_auth(
        &app,
        "customers",
        json!({ "email": "alice@example.com", "password": "my_secret_pw" }),
        &token,
    )
    .await;

    let stored_pw = record["data"]["password"]
        .as_str()
        .expect("password field in response");

    assert_ne!(
        stored_pw, "my_secret_pw",
        "password was stored as plaintext!"
    );
    assert!(
        bcrypt::verify("my_secret_pw", stored_pw).unwrap_or(false),
        "stored value is not a valid bcrypt hash"
    );
}

// ─── MVP Query Engine: Filter, Sort, Expand, Fields ──────────────────────────

/// [MVP GAP / Phase 1.1]: `?filter` query parameter.
/// Records list MUST filter results according to the filter expression.
/// E.g. `?filter=(views > 50)` must only return records where views > 50.
///
/// Ref: MVP_ROADMAP.md §1.1 / §Phase 1.1
#[tokio::test]
async fn test_filter_parameter_evaluates_where_clause() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    create_record_api(&app, "posts", json!({ "title": "Published", "views": 100 })).await;
    create_record_api(&app, "posts", json!({ "title": "Draft", "views": 10 })).await;

    let res = app
        .get("/api/collections/posts/records?filter=(views > 50)")
        .await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items");

    // FAIL REASON IF NOT IMPLEMENTED: `?filter` is currently ignored by PaginationParams and repository,
    // returning all 2 records instead of only the 1 record where views > 50.
    assert_eq!(
        items.len(),
        1,
        "MVP Gap: ?filter=(views > 50) must return exactly 1 item, but got {}",
        items.len()
    );
    assert_eq!(items[0]["data"]["title"], "Published");
}

/// [MVP GAP / Phase 1.2]: `?sort` query parameter.
/// Records list MUST sort results according to the sort expression.
/// E.g. `?sort=-views` sorts descending (highest views first).
///
/// Ref: MVP_ROADMAP.md §1.2 / §Phase 1.2
#[tokio::test]
async fn test_sort_parameter_orders_results() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    create_record_api(&app, "posts", json!({ "title": "Post A", "views": 10 })).await;
    create_record_api(&app, "posts", json!({ "title": "Post B", "views": 300 })).await;
    create_record_api(&app, "posts", json!({ "title": "Post C", "views": 50 })).await;

    // Sort descending by views: Post B (300) -> Post C (50) -> Post A (10)
    let res = app.get("/api/collections/posts/records?sort=-views").await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items");

    assert_eq!(items.len(), 3);

    // FAIL REASON IF NOT IMPLEMENTED: `?sort` is not evaluated; rows returned in insertion or DB order.
    assert_eq!(
        items[0]["data"]["title"], "Post B",
        "MVP Gap: ?sort=-views first item should be Post B with highest views (300)"
    );
    assert_eq!(items[1]["data"]["title"], "Post C");
    assert_eq!(items[2]["data"]["title"], "Post A");
}

/// [MVP GAP / Phase 3.2]: `?expand` relation field expansion.
/// When a record contains a relation column pointing to another collection,
/// `?expand=author` must populate the target record in `record.expand.author`.
///
/// Ref: MVP_ROADMAP.md §1.3 / §Phase 3.2
#[tokio::test]
async fn test_expand_parameter_populates_related_record() {
    let (app, token) = setup().await;

    // 1. Create authors collection
    create_collection(&app, "authors", vec![text_col("name")], &token).await;

    // 2. Create books collection with relation to authors
    let books_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "books",
                "columns": [
                    { "name": "title", "type": "text", "index": false },
                    { "name": "author", "type": "relation", "related_to": "authors", "index": false }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(books_res, StatusCode::OK).await;

    // 3. Create an author record
    let author = create_record_api(&app, "authors", json!({ "name": "George Orwell" })).await;
    let author_id = author["id"].as_str().expect("author id");

    // 4. Create a book record linking to author
    let book = create_record_api(
        &app,
        "books",
        json!({ "title": "1984", "author": author_id }),
    )
    .await;
    let book_id = book["id"].as_str().expect("book id");

    // 5. Fetch book with ?expand=author
    let res = app
        .get(&format!(
            "/api/collections/books/records/{book_id}?expand=author"
        ))
        .await;
    let body = expect(res, StatusCode::OK).await;

    // FAIL REASON IF NOT IMPLEMENTED: `expand` object is missing from response.
    assert!(
        body["expand"].is_object(),
        "MVP Gap: 'expand' object missing in record response when ?expand=author is requested. Got: {:?}",
        body
    );
    assert_eq!(
        body["expand"]["author"]["data"]["name"], "George Orwell",
        "MVP Gap: expanded author must contain author's record data"
    );
}

/// [MVP GAP / §2.2]: `?fields` parameter column projection.
/// Only requested fields should be included in the response payload.
///
/// Ref: MVP_ROADMAP.md §2.2
#[tokio::test]
async fn test_fields_parameter_restricts_returned_columns() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    let created = create_record_api(
        &app,
        "posts",
        json!({ "title": "Projection Test", "views": 999 }),
    )
    .await;
    let id = created["id"].as_str().expect("id");

    // Request only id and title
    let res = app
        .get(&format!(
            "/api/collections/posts/records/{id}?fields=id,title"
        ))
        .await;
    let body = expect(res, StatusCode::OK).await;

    // FAIL REASON IF NOT IMPLEMENTED: `?fields` ignored, all columns (`views`) still returned.
    assert!(body.get("id").is_some());
    assert_eq!(body["data"]["title"], "Projection Test");
    assert!(
        body["data"].get("views").is_none(),
        "MVP Gap: ?fields=id,title must not return unrequested 'views' field. Got: {:?}",
        body["data"]
    );
}
