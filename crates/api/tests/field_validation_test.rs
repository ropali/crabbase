//! HTTP API integration tests for Server-Side Field Validation Engine.
//!
//! MVP Feature & Data Integrity Requirement:
//!   - `required: true` fields cannot be null/missing on record creation.
//!   - String `min` and `max` character length boundaries are enforced.
//!   - Number `min` and `max` value boundaries are enforced.
//!   - Regex `pattern` matching is enforced.
//!   - Relation foreign key references are validated before insert.
//!
//! Ref: MVP_ROADMAP.md §2.4 and §Phase 3.1

mod common;

use axum::http::StatusCode;
use common::{TestApp, expect, insert_superuser, login};
use serde_json::json;

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("validation").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;
    (app, token)
}

/// [MVP GAP / Phase 3.1]: `required: true` field check.
/// Creating a record with a missing or null `required` field must return 400 Bad Request (Validation Error).
///
/// Ref: MVP_ROADMAP.md §2.4 / §Phase 3.1
#[tokio::test]
async fn test_validation_required_field_missing_on_create_rejected() {
    let (app, token) = setup().await;

    // 1. Create collection where `title` is required
    let col_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "projects",
                "columns": [
                    { "name": "title", "type": "text", "required": true },
                    { "name": "description", "type": "text", "required": false }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(col_res, StatusCode::OK).await;

    // 2. Attempt to create a record without `title`
    let res = app
        .post_json(
            "/api/collections/projects/records",
            json!({
                "data": {
                    "description": "Missing title"
                }
            }),
            None,
        )
        .await;

    let status = res.status();

    // FAIL REASON IF NOT IMPLEMENTED: `required: true` is currently stored in _collections.fields
    // but never validated before SQL INSERT, resulting in successful 200 insert of null.
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "MVP Gap: Missing required field must return 400 BAD_REQUEST, but got {}",
        status
    );
}

/// [MVP GAP / Phase 3.1]: String `min` and `max` length validation.
#[tokio::test]
async fn test_validation_string_min_max_length_enforced() {
    let (app, token) = setup().await;

    let col_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "usernames",
                "columns": [
                    { "name": "handle", "type": "text", "min": 3, "max": 10 }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(col_res, StatusCode::OK).await;

    // 1. Handle too short (< 3 chars): "ab"
    let too_short_res = app
        .post_json(
            "/api/collections/usernames/records",
            json!({ "data": { "handle": "ab" } }),
            None,
        )
        .await;

    assert_eq!(
        too_short_res.status(),
        StatusCode::BAD_REQUEST,
        "MVP Gap: String shorter than min length (3) must return 400, got {}",
        too_short_res.status()
    );

    // 2. Handle too long (> 10 chars): "this_handle_is_too_long"
    let too_long_res = app
        .post_json(
            "/api/collections/usernames/records",
            json!({ "data": { "handle": "this_handle_is_too_long" } }),
            None,
        )
        .await;

    assert_eq!(
        too_long_res.status(),
        StatusCode::BAD_REQUEST,
        "MVP Gap: String longer than max length (10) must return 400, got {}",
        too_long_res.status()
    );

    // 3. Valid handle (3-10 chars): "valid_user"
    let valid_res = app
        .post_json(
            "/api/collections/usernames/records",
            json!({ "data": { "handle": "valid_user" } }),
            None,
        )
        .await;
    expect(valid_res, StatusCode::OK).await;
}

/// [MVP GAP / Phase 3.1]: Number `min` and `max` range validation.
#[tokio::test]
async fn test_validation_number_min_max_range_enforced() {
    let (app, token) = setup().await;

    let col_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "ratings",
                "columns": [
                    { "name": "score", "type": "number", "min": 1, "max": 5 }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(col_res, StatusCode::OK).await;

    // Score < 1 (0) -> 400
    let low_res = app
        .post_json(
            "/api/collections/ratings/records",
            json!({ "data": { "score": 0 } }),
            None,
        )
        .await;

    assert_eq!(
        low_res.status(),
        StatusCode::BAD_REQUEST,
        "MVP Gap: Number below min (1) must return 400, got {}",
        low_res.status()
    );

    // Score > 5 (10) -> 400
    let high_res = app
        .post_json(
            "/api/collections/ratings/records",
            json!({ "data": { "score": 10 } }),
            None,
        )
        .await;

    assert_eq!(
        high_res.status(),
        StatusCode::BAD_REQUEST,
        "MVP Gap: Number above max (5) must return 400, got {}",
        high_res.status()
    );
}

/// [MVP GAP / Phase 3.1]: Regex `pattern` validation.
#[tokio::test]
async fn test_validation_pattern_regex_enforced() {
    let (app, token) = setup().await;

    let col_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "zip_codes",
                "columns": [
                    { "name": "code", "type": "text", "pattern": "^[0-9]{5}$" }
                ]
            }),
            Some(&token),
        )
        .await;
    expect(col_res, StatusCode::OK).await;

    // Invalid pattern ("abcde") -> 400
    let invalid_res = app
        .post_json(
            "/api/collections/zip_codes/records",
            json!({ "data": { "code": "abcde" } }),
            None,
        )
        .await;

    assert_eq!(
        invalid_res.status(),
        StatusCode::BAD_REQUEST,
        "MVP Gap: Value failing regex pattern must return 400, got {}",
        invalid_res.status()
    );

    // Valid pattern ("90210") -> 200
    let valid_res = app
        .post_json(
            "/api/collections/zip_codes/records",
            json!({ "data": { "code": "90210" } }),
            None,
        )
        .await;
    expect(valid_res, StatusCode::OK).await;
}
