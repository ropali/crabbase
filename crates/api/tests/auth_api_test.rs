//! HTTP API integration tests for auth endpoints.
//!
//! Tests cover:
//!   - Login happy path → get access + refresh tokens
//!   - Wrong credentials → 401
//!   - Access token required for protected endpoints (/profile)
//!   - Refresh token rotation
//!   - Logout invalidates refresh token
//!   - Unverified users cannot login (403)
//!   - MVP Target Features & Gaps:
//!     - Public user self-registration via `POST /api/collections/:auth_col/records` when `create_rule = ""` (GAP: MVP_ROADMAP.md §2.3 / §Phase 2.2)
//!     - Password reset OTP request & confirm

mod common;

use axum::http::StatusCode;
use common::{TestApp, expect, insert_superuser, login};
use serde_json::json;

// ─── Setup helpers ────────────────────────────────────────────────────────────

async fn setup() -> TestApp {
    TestApp::new("auth_api").await
}

// ─── Login ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_login_happy_path_returns_tokens() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "alice@test.com", "password123").await;

    let res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "alice@test.com", "password": "password123" }),
            None,
        )
        .await;

    let body = expect(res, StatusCode::OK).await;

    let access = body["tokens"]["accessToken"].as_str();
    let refresh = body["tokens"]["refreshToken"].as_str();

    assert!(access.is_some(), "accessToken missing: {body}");
    assert!(refresh.is_some(), "refreshToken missing: {body}");

    // JWT structure: 3 base64 segments separated by dots
    let token = access.unwrap();
    assert_eq!(
        token.split('.').count(),
        3,
        "accessToken is not a valid JWT: {token}"
    );
}

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "bob@test.com", "correct_pw").await;

    let res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "bob@test.com", "password": "wrong_pw" }),
            None,
        )
        .await;

    expect(res, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_login_unknown_email_returns_error() {
    let app = setup().await;

    let res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "nobody@test.com", "password": "anything" }),
            None,
        )
        .await;

    let status = res.status();
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::UNAUTHORIZED,
        "expected 404 or 401, got {status}"
    );
}

#[tokio::test]
async fn test_login_nonexistent_collection_returns_404() {
    let app = setup().await;

    let res = app
        .post_json(
            "/api/auth/ghost_collection/login",
            json!({ "email": "x@x.com", "password": "x" }),
            None,
        )
        .await;

    expect(res, StatusCode::NOT_FOUND).await;
}

// ─── Profile endpoint (requires auth) ────────────────────────────────────────

#[tokio::test]
async fn test_profile_requires_auth_token() {
    let app = setup().await;

    let res = app.get("/api/auth/profile").await;
    expect(res, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_profile_returns_user_info_when_authenticated() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "carol@test.com", "pw456").await;
    let token = login(&app, "_superusers", "carol@test.com", "pw456").await;

    let res = app.get_auth("/api/auth/profile", &token).await;
    let body = expect(res, StatusCode::OK).await;

    assert_eq!(
        body["email"], "carol@test.com",
        "profile should return authenticated user's email: {body}"
    );
    assert!(body["id"].as_str().is_some(), "id missing from profile");
}

// ─── Unverified user login ────────────────────────────────────────────────────

#[tokio::test]
async fn test_unverified_user_cannot_login() {
    let app = setup().await;

    let hashed = bcrypt::hash("pw", 4).expect("hash");
    sqlx::query(
        "INSERT INTO _superusers (email, password, token_key, verified)
         VALUES ($1, $2, 'some_token_key', false)",
    )
    .bind("unverified@test.com")
    .bind(&hashed)
    .execute(&app.db.pool)
    .await
    .expect("insert unverified user");

    let res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "unverified@test.com", "password": "pw" }),
            None,
        )
        .await;

    expect(res, StatusCode::FORBIDDEN).await;
}

// ─── Refresh token rotation ────────────────────────────────────────────────────

#[tokio::test]
async fn test_refresh_token_returns_new_access_token() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "dave@test.com", "refresh_test_pw").await;

    let login_res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "dave@test.com", "password": "refresh_test_pw" }),
            None,
        )
        .await;
    let login_body = expect(login_res, StatusCode::OK).await;
    let access_token = login_body["tokens"]["accessToken"]
        .as_str()
        .unwrap()
        .to_string();
    let refresh_token = login_body["tokens"]["refreshToken"]
        .as_str()
        .unwrap()
        .to_string();

    let refresh_res = app
        .post_json(
            "/api/auth/_superusers/auth-refresh",
            json!({ "refreshToken": refresh_token }),
            Some(&access_token),
        )
        .await;
    let refresh_body = expect(refresh_res, StatusCode::OK).await;

    let new_access = refresh_body["tokens"]["accessToken"]
        .as_str()
        .expect("new accessToken");
    let new_refresh = refresh_body["tokens"]["refreshToken"]
        .as_str()
        .expect("new refreshToken");

    assert_ne!(new_access, access_token, "access token was not rotated");
    assert_ne!(new_refresh, refresh_token, "refresh token was not rotated");
}

#[tokio::test]
async fn test_refresh_token_reuse_is_rejected() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "eve@test.com", "reuse_test_pw").await;

    let login_res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "eve@test.com", "password": "reuse_test_pw" }),
            None,
        )
        .await;
    let login_body = expect(login_res, StatusCode::OK).await;
    let access_token = login_body["tokens"]["accessToken"]
        .as_str()
        .unwrap()
        .to_string();
    let refresh_token = login_body["tokens"]["refreshToken"]
        .as_str()
        .unwrap()
        .to_string();

    let first_refresh = app
        .post_json(
            "/api/auth/_superusers/auth-refresh",
            json!({ "refreshToken": refresh_token }),
            Some(&access_token),
        )
        .await;
    expect(first_refresh, StatusCode::OK).await;

    // Second refresh using the same consumed token must fail
    let second_refresh = app
        .post_json(
            "/api/auth/_superusers/auth-refresh",
            json!({ "refreshToken": refresh_token }),
            Some(&access_token),
        )
        .await;

    assert!(
        second_refresh.status() == StatusCode::UNAUTHORIZED
            || second_refresh.status() == StatusCode::FORBIDDEN,
        "expected 401/403 on reused refresh token, got {}",
        second_refresh.status()
    );
}

// ─── Logout ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_logout_invalidates_refresh_token() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "frank@test.com", "logout_pw").await;

    let login_res = app
        .post_json(
            "/api/auth/_superusers/login",
            json!({ "email": "frank@test.com", "password": "logout_pw" }),
            None,
        )
        .await;
    let login_body = expect(login_res, StatusCode::OK).await;
    let access_token = login_body["tokens"]["accessToken"]
        .as_str()
        .unwrap()
        .to_string();
    let refresh_token = login_body["tokens"]["refreshToken"]
        .as_str()
        .unwrap()
        .to_string();

    let logout_res = app
        .post_json(
            "/api/auth/_superusers/logout",
            json!({ "email": "frank@test.com", "refreshToken": refresh_token }),
            Some(&access_token),
        )
        .await;
    expect(logout_res, StatusCode::OK).await;

    let reuse_res = app
        .post_json(
            "/api/auth/_superusers/auth-refresh",
            json!({ "refreshToken": refresh_token }),
            Some(&access_token),
        )
        .await;

    assert!(
        reuse_res.status() == StatusCode::UNAUTHORIZED
            || reuse_res.status() == StatusCode::FORBIDDEN,
        "revoked token should be rejected after logout, got {}",
        reuse_res.status()
    );
}

// ─── Public User Registration (MVP Phase 2.2) ────────────────────────────────

/// [MVP GAP / Phase 2.2]: Public User Self-Registration.
/// When `create_rule = ""` on an auth collection (e.g. `users`), unauthenticated visitors
/// can register via `POST /api/collections/:auth_col/records`.
/// Backend must: hash password, generate token_key, create record.
///
/// Ref: MVP_ROADMAP.md §2.3 / §Phase 2.2
#[tokio::test]
async fn test_public_user_registration_via_create_rule_empty() {
    let app = setup().await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let admin_token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;

    // 1. Create `users` auth collection with create_rule = "" (public signup allowed)
    let col_res = app
        .post_json(
            "/api/collections",
            json!({
                "name": "app_users",
                "collection_type": "auth",
                "create_rule": "", // public registration allowed!
                "columns": [
                    { "name": "name", "type": "text" },
                    { "name": "email", "type": "email", "index": true },
                    { "name": "password", "type": "text" },
                    { "name": "token_key", "type": "text" }
                ]
            }),
            Some(&admin_token),
        )
        .await;
    expect(col_res, StatusCode::OK).await;

    // 2. Unauthenticated user signs up
    let signup_res = app
        .post_json(
            "/api/collections/app_users/records",
            json!({
                "data": {
                    "name": "New User",
                    "email": "newuser@example.com",
                    "password": "securepassword123"
                }
            }),
            None, // No auth token!
        )
        .await;

    let status = signup_res.status();

    // FAIL REASON IF NOT IMPLEMENTED: If `create_rule` checking is missing or blocks unauthenticated
    // without respecting `create_rule = ""`, signup will fail with 401/403.
    assert_eq!(
        status,
        StatusCode::OK,
        "MVP Gap: Public self-registration on auth collection with create_rule='' must return 200 OK, got {}",
        status
    );

    let body = common::body_json(signup_res).await;
    assert_eq!(body["data"]["email"], "newuser@example.com");
}
