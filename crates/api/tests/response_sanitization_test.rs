//! HTTP API integration tests for Response Field Sanitization.
//!
//! MVP Feature & Security Requirement:
//!   - `Record` responses must NEVER expose `"password"` hash or `"token_key"`
//!     in public or user-facing JSON responses (`GET /records`, `GET /records/:id`, `PATCH /records/:id`).
//!   - Private email addresses must be stripped when `emailVisibility == false`
//!     unless requester is the record owner or a superuser.
//!
//! Ref: MVP_ROADMAP.md §2.3 and §Phase 2.3

mod common;

use axum::http::StatusCode;
use common::{TestApp, create_record_auth, expect, insert_superuser, login};
use serde_json::json;

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("sanitize").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;
    (app, token)
}

/// [MVP SECURITY GAP / Phase 2.3]: Password Hash & Token Key must be stripped from Record responses.
/// Currently `Record::from_row` returns whatever is stored in the Postgres row, leaking
/// the bcrypt password hash and token_key in JSON payloads.
///
/// Ref: MVP_ROADMAP.md §2.3 / §Phase 2.3
#[tokio::test]
async fn test_record_responses_strip_password_and_token_key() {
    let (app, token) = setup().await;

    // 1. Create an auth collection
    let col_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "accounts",
                "collection_type": "auth",
                "columns": [
                    { "name": "email", "type": "email", "index": true },
                    { "name": "password", "type": "text" },
                    { "name": "token_key", "type": "text" }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(col_res, StatusCode::OK).await;

    // 2. Create a record in `accounts`
    let created = create_record_auth(
        &app,
        "accounts",
        json!({
            "email": "user@example.com",
            "password": "secret_password"
        }),
        &token,
    )
    .await;
    let id = created["id"].as_str().expect("record id");

    // Check create response
    assert!(
        created["data"].get("password").is_none() || created["data"]["password"].is_null(),
        "MVP Security Gap: 'password' hash must NEVER be present in CREATE record response! Got: {:?}",
        created["data"]["password"]
    );
    assert!(
        created["data"].get("token_key").is_none() || created["data"]["token_key"].is_null(),
        "MVP Security Gap: 'token_key' must NEVER be present in CREATE record response! Got: {:?}",
        created["data"]["token_key"]
    );

    // 3. Check GET /records/:id response
    let get_res = app
        .get_auth(&format!("/api/collections/accounts/records/{id}"), &token)
        .await;
    let get_body = expect(get_res, StatusCode::OK).await;

    assert!(
        get_body["data"].get("password").is_none() || get_body["data"]["password"].is_null(),
        "MVP Security Gap: 'password' hash must NEVER be returned in GET /records/:id response!"
    );
    assert!(
        get_body["data"].get("token_key").is_none() || get_body["data"]["token_key"].is_null(),
        "MVP Security Gap: 'token_key' must NEVER be returned in GET /records/:id response!"
    );

    // 4. Check GET /records list response
    let list_res = app
        .get_auth("/api/collections/accounts/records", &token)
        .await;
    let list_body = expect(list_res, StatusCode::OK).await;
    let items = list_body["items"].as_array().expect("items");

    for item in items {
        assert!(
            item["data"].get("password").is_none() || item["data"]["password"].is_null(),
            "MVP Security Gap: 'password' hash found in list records item!"
        );
        assert!(
            item["data"].get("token_key").is_none() || item["data"]["token_key"].is_null(),
            "MVP Security Gap: 'token_key' found in list records item!"
        );
    }
}
