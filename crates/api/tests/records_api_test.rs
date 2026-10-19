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
    TestApp, body_json, create_collection, create_record_api, create_record_auth, expect, file_col,
    file_col_multi, insert_superuser, login, number_col, text_col,
};
use serde_json::json;
use sqlx::Row;

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

    // Allow public listing of records on the `posts` collection
    app.patch_json("/api/collections/posts", json!({ "list_rule": "" }), token)
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
    assert_eq!(body["per_page"], 2);
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

    // Response sanitization ensures password hash is never exposed in API responses
    assert!(
        record["data"].get("password").is_none() || record["data"]["password"].is_null(),
        "password hash must not be leaked in API response"
    );

    // Verify the password was hashed securely in the database
    let id = record["id"].as_str().expect("record id");
    let row = sqlx::query("SELECT password FROM customers WHERE id = $1")
        .bind(uuid::Uuid::parse_str(id).unwrap())
        .fetch_one(&app.db.pool)
        .await
        .expect("fetch record from db");

    let stored_pw: String = row.try_get("password").expect("password column in db");

    assert_ne!(
        stored_pw, "my_secret_pw",
        "password was stored as plaintext!"
    );
    assert!(
        bcrypt::verify("my_secret_pw", &stored_pw).unwrap_or(false),
        "stored value is not a valid bcrypt hash"
    );
}

// ─── MVP Query Engine: Filter, Sort, Expand, Fields ──────────────────────────

/// Records list MUST filter results according to the filter expression.
/// E.g. `?filter=(views > 50)` must only return records where views > 50.
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

    assert_eq!(
        items.len(),
        1,
        "MVP Gap: ?filter=(views > 50) must return exactly 1 item, but got {}",
        items.len()
    );
    assert_eq!(items[0]["data"]["title"], "Published");
}

#[tokio::test]
async fn test_filter_parameter_string_comparison_and_substring() {
    let (app, token) = setup().await;
    setup_posts(&app, &token).await;

    create_record_api(
        &app,
        "posts",
        json!({ "title": "Rust Programming", "views": 100 }),
    )
    .await;
    create_record_api(&app, "posts", json!({ "title": "Go Guide", "views": 200 })).await;

    // String equality filter
    let res = app
        .get("/api/collections/posts/records?filter=title = 'Rust Programming'")
        .await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["data"]["title"], "Rust Programming");

    // Substring ILIKE search filter
    let res_like = app
        .get("/api/collections/posts/records?filter=title ~ 'guide'")
        .await;
    let body_like = expect(res_like, StatusCode::OK).await;
    let items_like = body_like["items"].as_array().expect("items");
    assert_eq!(items_like.len(), 1);
    assert_eq!(items_like[0]["data"]["title"], "Go Guide");

    // Superuser filter
    let res_admin = app
        .get_auth("/api/collections/posts/records?filter=views >= 200", &token)
        .await;
    let body_admin = expect(res_admin, StatusCode::OK).await;
    let items_admin = body_admin["items"].as_array().expect("items");
    assert_eq!(items_admin.len(), 1);
    assert_eq!(items_admin[0]["data"]["title"], "Go Guide");
}

/// Records list MUST sort results according to the sort expression.
/// E.g. `?sort=-views` sorts descending (highest views first).
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

/// When a record contains a relation column pointing to another collection,
/// `?expand=author` must populate the target record in `record.expand.author`.
///
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
    app.patch_json(
        "/api/collections/books",
        json!({ "list_rule": "", "view_rule": "", "create_rule": "" }),
        &token,
    )
    .await;

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

/// When a record contains a one-to-many relation column pointing to another collection (`max > 1`),
/// it can store multiple foreign key IDs, and `?expand=tags` must populate all target records
/// as an array in `record.expand.tags`.
#[tokio::test]
async fn test_one_to_many_relation_expand() {
    let (app, token) = setup().await;

    // 1. Create tags collection (target of relation)
    create_collection(&app, "tags", vec![text_col("name")], &token).await;

    // 2. Create posts collection with one-to-many relation to tags
    let posts_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "posts_multi_rel",
                "columns": [
                    { "name": "title", "type": "text", "index": false },
                    { "name": "tags", "type": "relation", "related_to": "tags", "max": 10, "index": false }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(posts_res, StatusCode::OK).await;

    // Allow public listing of records on the `posts_multi_rel` collection
    app.patch_json(
        "/api/collections/posts_multi_rel",
        json!({ "list_rule": "", "view_rule": "", "create_rule": "" }),
        &token,
    )
    .await;

    // 3. Create two tag records
    let tag1 = create_record_api(&app, "tags", json!({ "name": "Rust" })).await;
    let tag1_id = tag1["id"].as_str().expect("tag1 id");

    let tag2 = create_record_api(&app, "tags", json!({ "name": "Web" })).await;
    let tag2_id = tag2["id"].as_str().expect("tag2 id");

    // 4. Create a post record linking to both tags (one-to-many relation)
    let post_res = app
        .post_json(
            "/api/collections/posts_multi_rel/records",
            json!({
                "data": {
                    "title": "Building Web Services in Rust",
                    "tags": [tag1_id, tag2_id]
                }
            }),
            None,
        )
        .await;

    // FAIL REASON IF NOT IMPLEMENTED: Currently relation fields only accept a single UUID string,
    // rejecting JSON arrays of UUIDs for one-to-many relationships.
    assert_eq!(
        post_res.status(),
        StatusCode::OK,
        "MVP Gap: creating record with one-to-many relation array of UUIDs must return 200 OK. Got: {}",
        post_res.status()
    );
    let post = body_json(post_res).await;
    let post_id = post["id"].as_str().expect("post id");

    // 5. Fetch single record with ?expand=tags
    let res = app
        .get(&format!(
            "/api/collections/posts_multi_rel/records/{post_id}?expand=tags"
        ))
        .await;
    let body = expect(res, StatusCode::OK).await;

    // FAIL REASON IF NOT IMPLEMENTED: `expand` object missing or `expand.tags` is not an array.
    assert!(
        body["expand"].is_object(),
        "MVP Gap: 'expand' object missing in record response when ?expand=tags is requested. Got: {:?}",
        body
    );
    assert!(
        body["expand"]["tags"].is_array(),
        "MVP Gap: expanded one-to-many relation 'tags' must be a JSON array of records. Got: {:?}",
        body["expand"]["tags"]
    );

    let expanded_tags = body["expand"]["tags"].as_array().expect("tags array");
    assert_eq!(
        expanded_tags.len(),
        2,
        "MVP Gap: expanded tags must contain 2 records"
    );

    let tag_names: Vec<&str> = expanded_tags
        .iter()
        .filter_map(|t| t["data"]["name"].as_str())
        .collect();
    assert!(
        tag_names.contains(&"Rust") && tag_names.contains(&"Web"),
        "MVP Gap: expanded tags array must contain 'Rust' and 'Web', got: {:?}",
        tag_names
    );

    // 6. List records with ?expand=tags
    let list_res = app
        .get("/api/collections/posts_multi_rel/records?expand=tags")
        .await;
    let list_body = expect(list_res, StatusCode::OK).await;
    let items = list_body["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "expected items in list response");
    assert!(
        items[0]["expand"]["tags"].is_array(),
        "MVP Gap: list items must have expanded tags array. Got: {:?}",
        items[0]["expand"]
    );
}

/// `?fields` parameter column projection.
/// Only requested fields should be included in the response payload.
///
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
}

#[tokio::test]
async fn test_delete_record_deletes_associated_file() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "media_posts",
        vec![text_col("title"), file_col("attachment")],
        &token,
    )
    .await;

    // 1. Upload file
    let upload_res = app
        .post_multipart(
            "/api/collection/media_posts/record/upload",
            "document.pdf",
            "application/pdf",
            b"test content 1",
            None,
        )
        .await;
    let body = expect(upload_res, StatusCode::OK).await;
    let file_key = body["files"][0]["key"]
        .as_str()
        .expect("file key")
        .to_string();

    // 2. Verify file exists in storage
    let file_res = app.get(&format!("/api/files/{file_key}")).await;
    assert_eq!(file_res.status(), StatusCode::OK);

    // 3. Create record referencing the file
    let record = create_record_api(
        &app,
        "media_posts",
        json!({ "title": "My Post", "attachment": file_key }),
    )
    .await;
    let record_id = record["id"].as_str().expect("id");

    // 4. Delete the record
    let del_res = app
        .delete_auth(
            &format!("/api/collections/media_posts/records/{record_id}"),
            &token,
        )
        .await;
    assert_eq!(del_res.status(), StatusCode::OK);

    // 5. Verify the file was automatically deleted from storage
    let file_after_del = app.get(&format!("/api/files/{file_key}")).await;
    assert_eq!(file_after_del.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_record_replaces_and_deletes_old_file() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "avatar_profiles",
        vec![text_col("username"), file_col("avatar")],
        &token,
    )
    .await;

    // 1. Upload first file
    let upload1 = app
        .post_multipart(
            "/api/collection/avatar_profiles/record/upload",
            "first.png",
            "image/png",
            b"first avatar",
            None,
        )
        .await;
    let key1 = expect(upload1, StatusCode::OK).await["files"][0]["key"]
        .as_str()
        .expect("key1")
        .to_string();

    // 2. Create record with first file
    let record = create_record_api(
        &app,
        "avatar_profiles",
        json!({ "username": "alice", "avatar": key1 }),
    )
    .await;
    let record_id = record["id"].as_str().expect("id");

    // 3. Upload second file
    let upload2 = app
        .post_multipart(
            "/api/collection/avatar_profiles/record/upload",
            "second.png",
            "image/png",
            b"second avatar",
            None,
        )
        .await;
    let key2 = expect(upload2, StatusCode::OK).await["files"][0]["key"]
        .as_str()
        .expect("key2")
        .to_string();

    // Both files currently exist
    assert_eq!(
        app.get(&format!("/api/files/{key1}")).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        app.get(&format!("/api/files/{key2}")).await.status(),
        StatusCode::OK
    );

    // 4. Update record to replace avatar with key2
    let update_res = app
        .patch_json(
            &format!("/api/collections/avatar_profiles/records/{record_id}"),
            json!({ "data": { "avatar": key2 } }),
            &token,
        )
        .await;
    assert_eq!(update_res.status(), StatusCode::OK);

    // 5. Verify old file (key1) is DELETED and new file (key2) STILL EXISTS
    assert_eq!(
        app.get(&format!("/api/files/{key1}")).await.status(),
        StatusCode::NOT_FOUND,
        "Old file must be deleted when replaced"
    );
    assert_eq!(
        app.get(&format!("/api/files/{key2}")).await.status(),
        StatusCode::OK,
        "New file must still exist"
    );

    // 6. Update record to remove avatar (set to null)
    let clear_res = app
        .patch_json(
            &format!("/api/collections/avatar_profiles/records/{record_id}"),
            json!({ "data": { "avatar": null } }),
            &token,
        )
        .await;
    assert_eq!(clear_res.status(), StatusCode::OK);

    // 7. Verify key2 is now deleted
    assert_eq!(
        app.get(&format!("/api/files/{key2}")).await.status(),
        StatusCode::NOT_FOUND,
        "File must be deleted when cleared to null"
    );
}

#[tokio::test]
async fn test_file_field_stores_file_id_and_enriches_list_paths() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "user_documents",
        vec![text_col("title"), file_col("attachment")],
        &token,
    )
    .await;

    // Set public view and list rules for user_documents
    app.patch_json(
        "/api/collections/user_documents",
        json!({
            "list_rule": "",
            "view_rule": "",
        }),
        &token,
    )
    .await;

    // 1. Upload file 1 via records/upload
    let upload1 = app
        .post_multipart(
            "/api/collections/user_documents/records/upload",
            "report.pdf",
            "application/pdf",
            b"%PDF-1.4 initial report content",
            Some(&token),
        )
        .await;
    let upload1_body = expect(upload1, StatusCode::OK).await;
    let file1_id = upload1_body["files"][0]["id"]
        .as_str()
        .expect("file1 id")
        .to_string();
    let file1_key = upload1_body["files"][0]["key"]
        .as_str()
        .expect("file1 key")
        .to_string();
    let file1_path = format!("/api/collections/user_documents/files/{file1_id}");

    // Verify file is readable via collection-scoped GET API
    let get_file1 = app.get(&file1_path).await;
    assert_eq!(get_file1.status(), StatusCode::OK);

    // 2. Create record referencing file1_id
    let rec = create_record_api(
        &app,
        "user_documents",
        json!({ "title": "Annual Report", "attachment": file1_id }),
    )
    .await;
    let rec_id = rec["id"].as_str().expect("rec id");
    assert_eq!(rec["data"]["attachment"]["id"], file1_id);
    assert_eq!(rec["data"]["attachment"]["originalName"], "report.pdf");
    assert_eq!(rec["data"]["attachment"]["contentType"], "application/pdf");
    assert!(rec["data"]["attachment"]["key"].as_str().is_some());

    // 3. List records and verify the file record is included
    let list_res = app.get("/api/collections/user_documents/records").await;
    let list_body = expect(list_res, StatusCode::OK).await;
    let items = list_body["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["data"]["attachment"]["id"], file1_id);
    assert_eq!(items[0]["data"]["attachment"]["originalName"], "report.pdf");
    assert_eq!(
        items[0]["data"]["attachment"]["collectionName"],
        "user_documents"
    );

    // 4. Upload replacement file 2
    let upload2 = app
        .post_multipart(
            "/api/collections/user_documents/records/upload",
            "report_v2.pdf",
            "application/pdf",
            b"%PDF-1.4 revised report content",
            Some(&token),
        )
        .await;
    let upload2_body = expect(upload2, StatusCode::OK).await;
    let file2_id = upload2_body["files"][0]["id"]
        .as_str()
        .expect("file2 id")
        .to_string();
    let file2_path = format!("/api/collections/user_documents/files/{file2_id}");

    // 5. Update record with replacement file2_id
    let update_res = app
        .patch_json(
            &format!("/api/collections/user_documents/records/{rec_id}"),
            json!({ "data": { "attachment": file2_id } }),
            &token,
        )
        .await;
    assert_eq!(update_res.status(), StatusCode::OK);

    // 6. Verify old file (file1) is purged from _files and disk, new file (file2) exists
    assert_eq!(app.get(&file1_path).await.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        app.get(&format!("/api/files/{file1_key}")).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.get(&file2_path).await.status(), StatusCode::OK);

    // 7. Delete record and verify file2 is purged from _files and disk
    let del_res = app
        .delete_auth(
            &format!("/api/collections/user_documents/records/{rec_id}"),
            &token,
        )
        .await;
    assert_eq!(del_res.status(), StatusCode::OK);

    assert_eq!(app.get(&file2_path).await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_collection_files_association_and_listing() {
    let (app, token) = setup().await;

    // 1. Create a "project_assets" collection
    create_collection(
        &app,
        "project_assets",
        vec![text_col("title"), file_col("files")],
        &token,
    )
    .await;

    // 2. Upload file 1 for "project_assets"
    let up1 = app
        .post_multipart(
            "/api/collections/project_assets/records/upload",
            "doc1.txt",
            "text/plain",
            b"project document 1",
            Some(&token),
        )
        .await;
    let up1_body = expect(up1, StatusCode::OK).await;
    assert_eq!(up1_body["files"][0]["collectionName"], "project_assets");
    assert_eq!(up1_body["files"][0]["originalName"], "doc1.txt");
    assert!(up1_body["files"][0]["checksum"].as_str().is_some());
    assert_eq!(up1_body["files"][0]["sizeBytes"], 18);
    let file1_id = up1_body["files"][0]["id"]
        .as_str()
        .expect("file1 id")
        .to_string();

    // 3. Upload file 2 for "project_assets"
    let up2 = app
        .post_multipart(
            "/api/collections/project_assets/records/upload",
            "image.png",
            "image/png",
            b"\x89PNG\r\n\x1a\nfake image bytes",
            Some(&token),
        )
        .await;
    let up2_body = expect(up2, StatusCode::OK).await;
    assert_eq!(up2_body["files"][0]["collectionName"], "project_assets");
    let file2_id = up2_body["files"][0]["id"]
        .as_str()
        .expect("file2 id")
        .to_string();

    // 4. Create another collection "other_assets" and upload a file there
    create_collection(&app, "other_assets", vec![text_col("notes")], &token).await;

    let up3 = app
        .post_multipart(
            "/api/collections/other_assets/records/upload",
            "other.log",
            "text/plain",
            b"other log data",
            Some(&token),
        )
        .await;
    let up3_body = expect(up3, StatusCode::OK).await;
    assert_eq!(up3_body["files"][0]["collectionName"], "other_assets");
    let file3_id = up3_body["files"][0]["id"]
        .as_str()
        .expect("file3 id")
        .to_string();

    // 5. List files for "project_assets"
    let list_res = app.get("/api/collections/project_assets/files").await;
    let list_body = expect(list_res, StatusCode::OK).await;
    let items = list_body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2);
    assert_eq!(list_body["totalItems"], 2);

    let ids: Vec<&str> = items.iter().filter_map(|i| i["id"].as_str()).collect();
    assert!(ids.contains(&file1_id.as_str()));
    assert!(ids.contains(&file2_id.as_str()));
    assert!(!ids.contains(&file3_id.as_str()));

    for item in items {
        assert_eq!(item["collectionName"], "project_assets");
        assert!(
            item["path"]
                .as_str()
                .unwrap()
                .starts_with("/api/collections/project_assets/files/")
        );
    }

    // 6. List files for "other_assets"
    let list2_res = app.get("/api/collections/other_assets/files").await;
    let list2_body = expect(list2_res, StatusCode::OK).await;
    let items2 = list2_body["items"].as_array().expect("items2 array");
    assert_eq!(items2.len(), 1);
    assert_eq!(items2[0]["id"], file3_id);
    assert_eq!(items2[0]["collectionName"], "other_assets");

    // 7. Verify singular alias /api/collection/{name}/files works as well
    let singular_res = app.get("/api/collection/project_assets/files").await;
    let singular_body = expect(singular_res, StatusCode::OK).await;
    assert_eq!(singular_body["totalItems"], 2);
}

#[tokio::test]
async fn test_upload_multiple_files_at_one_go_via_api() {
    let (app, token) = setup().await;

    // 1. Create a collection
    create_collection(
        &app,
        "gallery_collection",
        vec![text_col("caption"), file_col("photos")],
        &token,
    )
    .await;

    // 2. Upload 3 files in a single multipart POST request
    let upload_files = vec![
        ("photo1.jpg", "image/jpeg", b"jpg1".as_slice()),
        ("photo2.png", "image/png", b"png2".as_slice()),
        ("notes.txt", "text/plain", b"plain3".as_slice()),
    ];

    let upload_res = app
        .post_multipart_files(
            "/api/collections/gallery_collection/records/upload",
            &upload_files,
            Some(&token),
        )
        .await;

    let body = expect(upload_res, StatusCode::OK).await;

    let files = body["files"].as_array().expect("files array");
    assert_eq!(files.len(), 3);

    let f1_id = files[0]["id"].as_str().expect("f1 id");
    let f2_id = files[1]["id"].as_str().expect("f2 id");
    let f3_id = files[2]["id"].as_str().expect("f3 id");

    assert_eq!(files[0]["collectionName"], "gallery_collection");
    assert_eq!(files[0]["originalName"], "photo1.jpg");
    assert_eq!(files[1]["originalName"], "photo2.png");
    assert_eq!(files[2]["originalName"], "notes.txt");
    for file in files {
        assert!(file["key"].as_str().is_some());
        assert!(file["checksum"].as_str().is_some());
        assert!(file["backend"].as_str().is_some());
        assert!(file["created"].as_str().is_some());
    }

    // 3. Download each uploaded file
    let d1 = app
        .get(&format!(
            "/api/collections/gallery_collection/files/{f1_id}"
        ))
        .await;
    assert_eq!(d1.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(d1.into_body(), usize::MAX)
            .await
            .unwrap(),
        b"jpg1".as_slice()
    );

    let d2 = app
        .get(&format!(
            "/api/collections/gallery_collection/files/{f2_id}"
        ))
        .await;
    assert_eq!(d2.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(d2.into_body(), usize::MAX)
            .await
            .unwrap(),
        b"png2".as_slice()
    );

    let d3 = app
        .get(&format!(
            "/api/collections/gallery_collection/files/{f3_id}"
        ))
        .await;
    assert_eq!(d3.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(d3.into_body(), usize::MAX)
            .await
            .unwrap(),
        b"plain3".as_slice()
    );

    // 4. List all files for gallery_collection
    let list_res = app.get("/api/collections/gallery_collection/files").await;
    let list_body = expect(list_res, StatusCode::OK).await;
    assert_eq!(list_body["totalItems"], 3);
}

#[tokio::test]
async fn test_multiple_file_field_full_lifecycle() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "photo_albums",
        vec![text_col("title"), file_col_multi("photos")],
        &token,
    )
    .await;

    // 1. Upload two files named after the file field
    let upload_res = app
        .post_multipart_many_as(
            "/api/collections/photo_albums/records/upload",
            "photos",
            &[
                ("first.png", "image/png", b"first".as_slice()),
                ("second.png", "image/png", b"second".as_slice()),
            ],
            Some(&token),
        )
        .await;
    let upload_body = expect(upload_res, StatusCode::OK).await;
    let files = upload_body["files"].as_array().expect("files array");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["originalName"], "first.png");
    assert_eq!(files[1]["originalName"], "second.png");

    let key1 = files[0]["key"].as_str().expect("key1").to_string();
    let id2 = files[1]["id"].as_str().expect("id2").to_string();

    // 2. Create a record storing the array of file references
    let record = create_record_auth(
        &app,
        "photo_albums",
        json!({ "title": "Trip", "photos": [key1, id2] }),
        &token,
    )
    .await;
    let record_id = record["id"].as_str().expect("record id").to_string();

    let photos = record["data"]["photos"].as_array().expect("photos array");
    assert_eq!(photos.len(), 2);
    assert_eq!(photos[0]["originalName"], "first.png");
    assert_eq!(photos[1]["id"], id2);
    assert_eq!(photos[1]["originalName"], "second.png");
    assert_eq!(photos[1]["collectionName"], "photo_albums");

    // 3. List records expands the array to file records too
    let list_res = app.get("/api/collections/photo_albums/records").await;
    let list_body = expect(list_res, StatusCode::OK).await;
    let listed = list_body["items"][0]["data"]["photos"]
        .as_array()
        .expect("listed photos");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["originalName"], "first.png");
    assert_eq!(listed[1]["id"], id2);

    // 4. Update to keep only the second file: the first is cleaned up
    let update_res = app
        .patch_json(
            &format!("/api/collections/photo_albums/records/{record_id}"),
            json!({ "data": { "photos": [id2] } }),
            &token,
        )
        .await;
    let updated = expect(update_res, StatusCode::OK).await;
    let updated_photos = updated["data"]["photos"]
        .as_array()
        .expect("updated photos");
    assert_eq!(updated_photos.len(), 1);
    assert_eq!(updated_photos[0]["id"], id2);

    assert_eq!(
        app.get(&format!("/api/files/{key1}")).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.get(&format!("/api/collections/photo_albums/files/{id2}"))
            .await
            .status(),
        StatusCode::OK
    );

    // 5. Deleting the record removes the remaining file
    let del_res = app
        .delete_auth(
            &format!("/api/collections/photo_albums/records/{record_id}"),
            &token,
        )
        .await;
    assert_eq!(del_res.status(), StatusCode::OK);
    assert_eq!(
        app.get(&format!("/api/files/{id2}")).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_file_field_rejects_array_for_single_file_column() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "avatars",
        vec![text_col("username"), file_col("avatar")],
        &token,
    )
    .await;

    let res = app
        .post_json(
            "/api/collections/avatars/records",
            json!({ "data": { "username": "bob", "avatar": ["a.png", "b.png"] } }),
            Some(&token),
        )
        .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_file_upload_enforces_column_constraints() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "tiny_uploads",
        vec![
            text_col("title"),
            json!({
                "name": "avatar",
                "type": "file",
                "index": false,
                "max_size": 16,
                "mime_types": ["image/png"]
            }),
        ],
        &token,
    )
    .await;

    // Oversized file is rejected
    let big = app
        .post_multipart_as(
            "/api/collections/tiny_uploads/records/upload",
            "avatar",
            "big.png",
            "image/png",
            &vec![0u8; 32],
            Some(&token),
        )
        .await;
    assert_eq!(big.status(), StatusCode::BAD_REQUEST);

    // Wrong MIME type is rejected
    let wrong_type = app
        .post_multipart_as(
            "/api/collections/tiny_uploads/records/upload",
            "avatar",
            "notes.txt",
            "text/plain",
            b"tiny",
            Some(&token),
        )
        .await;
    assert_eq!(wrong_type.status(), StatusCode::BAD_REQUEST);

    // Compliant file is accepted
    let ok = app
        .post_multipart_as(
            "/api/collections/tiny_uploads/records/upload",
            "avatar",
            "ok.png",
            "image/png",
            b"small",
            Some(&token),
        )
        .await;
    let ok_body = expect(ok, StatusCode::OK).await;
    assert_eq!(ok_body["files"][0]["originalName"], "ok.png");
    assert_eq!(ok_body["files"][0]["sizeBytes"], 5);

    // Two parts for a single-file column are rejected
    let twice = app
        .post_multipart_many_as(
            "/api/collections/tiny_uploads/records/upload",
            "avatar",
            &[
                ("a.png", "image/png", b"one".as_slice()),
                ("b.png", "image/png", b"two".as_slice()),
            ],
            Some(&token),
        )
        .await;
    assert_eq!(twice.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_file_upload_enforces_column_pattern() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "pattern_uploads",
        vec![
            text_col("title"),
            json!({
                "name": "avatar",
                "type": "file",
                "index": false,
                "pattern": "\\.(png|jpe?g)$"
            }),
        ],
        &token,
    )
    .await;

    // Disallowed extension is rejected with the pattern violation message
    let bad = app
        .post_multipart_as(
            "/api/collections/pattern_uploads/records/upload",
            "avatar",
            "notes.pdf",
            "application/pdf",
            b"%PDF-1.4",
            Some(&token),
        )
        .await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    let bad_body = body_json(bad).await;
    let message = bad_body["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("does not match pattern"),
        "expected pattern violation message, got: {message}"
    );

    // Allowed extension passes
    let ok = app
        .post_multipart_as(
            "/api/collections/pattern_uploads/records/upload",
            "avatar",
            "portrait.png",
            "image/png",
            b"png-bytes",
            Some(&token),
        )
        .await;
    let ok_body = expect(ok, StatusCode::OK).await;
    assert_eq!(ok_body["files"][0]["originalName"], "portrait.png");

    // The regex is case-sensitive unless the pattern says otherwise
    let upper = app
        .post_multipart_as(
            "/api/collections/pattern_uploads/records/upload",
            "avatar",
            "portrait.PNG",
            "image/png",
            b"png-bytes",
            Some(&token),
        )
        .await;
    assert_eq!(upper.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_unmatched_multipart_part_name_stays_generic() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "generic_uploads",
        vec![text_col("title"), file_col("attachment")],
        &token,
    )
    .await;

    // Part name does not match any file column: no field-level constraints apply
    let res = app
        .post_multipart_as(
            "/api/collections/generic_uploads/records/upload",
            "file",
            "anything.txt",
            "text/plain",
            b"generic",
            Some(&token),
        )
        .await;
    let body = expect(res, StatusCode::OK).await;
    assert_eq!(body["files"][0]["originalName"], "anything.txt");
    assert_eq!(body["files"][0]["collectionName"], "generic_uploads");
}

#[tokio::test]
async fn test_files_route_only_lists_never_uploads() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "files_route_only",
        vec![text_col("title"), file_col("doc")],
        &token,
    )
    .await;

    // Uploading through the files routes is gone; records/upload is the only way in
    let res = app
        .post_multipart(
            "/api/collections/files_route_only/files",
            "a.txt",
            "text/plain",
            b"hello",
            Some(token.as_str()),
        )
        .await;
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);

    let res = app
        .post_multipart(
            "/api/collections/files_route_only/records/files",
            "a.txt",
            "text/plain",
            b"hello",
            Some(token.as_str()),
        )
        .await;
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Listing still works
    let res = app
        .get_auth("/api/collections/files_route_only/files", &token)
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}
