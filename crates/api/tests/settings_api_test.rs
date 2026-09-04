//! HTTP API integration tests for the Settings endpoint.
//!
//! Tests cover:
//!   - Admin-only authorization for all `/api/settings/*` routes (401/403 for unauthenticated)
//!   - Mail settings (`GET` and `POST` at `/api/settings/mail`)
//!   - App settings (`GET` and `POST` at `/api/settings/app`)
//!   - Email templates (`GET` and `POST` at `/api/settings/email-templates`)

mod common;

use axum::http::StatusCode;
use common::{TestApp, expect, insert_superuser, login};
use serde_json::json;

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("settings_api").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;
    (app, token)
}

// ─── Authorization ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_settings_endpoints_require_admin() {
    let app = TestApp::new("settings_no_auth").await;

    // 1. Mail settings without token -> 401/403
    let mail_res = app.get("/api/settings/mail").await;
    assert!(
        mail_res.status() == StatusCode::UNAUTHORIZED || mail_res.status() == StatusCode::FORBIDDEN,
        "expected 401/403 for unauthenticated /api/settings/mail, got {}",
        mail_res.status()
    );

    // 2. App settings without token -> 401/403
    let app_res = app.get("/api/settings/app").await;
    assert!(
        app_res.status() == StatusCode::UNAUTHORIZED || app_res.status() == StatusCode::FORBIDDEN,
        "expected 401/403 for unauthenticated /api/settings/app, got {}",
        app_res.status()
    );

    // 3. Email templates without token -> 401/403
    let tpl_res = app.get("/api/settings/email-templates").await;
    assert!(
        tpl_res.status() == StatusCode::UNAUTHORIZED || tpl_res.status() == StatusCode::FORBIDDEN,
        "expected 401/403 for unauthenticated /api/settings/email-templates, got {}",
        tpl_res.status()
    );
}

// ─── Mail Settings ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mail_settings_get_and_post() {
    let (app, token) = setup().await;

    // 1. Initial GET returns default mail settings
    let get_res = app.get_auth("/api/settings/mail", &token).await;
    let get_body = expect(get_res, StatusCode::OK).await;
    assert!(get_body.get("senderAddress").is_some() || get_body.get("sender_address").is_some());

    // 2. Save new mail settings
    let save_payload = json!({
        "senderName": "Crabbase Admin",
        "senderAddress": "admin@crabbase.io",
        "smtpHost": "smtp.mailgun.org",
        "smtpPort": 587,
        "smtpUsername": "postmaster@crabbase.io",
        "smtpPassword": "super_secret_smtp_password"
    });

    let post_res = app
        .post_json("/api/settings/mail", save_payload, Some(&token))
        .await;
    expect(post_res, StatusCode::OK).await;

    // 3. Fetch again to verify persistence
    let verify_res = app.get_auth("/api/settings/mail", &token).await;
    let verify_body = expect(verify_res, StatusCode::OK).await;

    assert_eq!(verify_body["senderName"], "Crabbase Admin");
    assert_eq!(verify_body["senderAddress"], "admin@crabbase.io");
    assert_eq!(verify_body["smtpHost"], "smtp.mailgun.org");
    assert_eq!(verify_body["smtpPort"], 587);
}

// ─── App Settings ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_app_settings_get_and_post() {
    let (app, token) = setup().await;

    // 1. Save app settings
    let save_payload = json!({
        "appName": "My Awesome App",
        "appUrl": "https://myapp.example.com",
        "contactEmail": "support@myapp.example.com",
        "allowPublicUserRegistration": true
    });

    let post_res = app
        .post_json("/api/settings/app", save_payload, Some(&token))
        .await;
    expect(post_res, StatusCode::OK).await;

    // 2. Fetch to verify persistence
    let get_res = app.get_auth("/api/settings/app", &token).await;
    let body = expect(get_res, StatusCode::OK).await;

    assert_eq!(body["appName"], "My Awesome App");
    assert_eq!(body["appUrl"], "https://myapp.example.com");
    assert_eq!(body["contactEmail"], "support@myapp.example.com");
    assert_eq!(body["allowPublicUserRegistration"], true);
}

// ─── Email Templates ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_email_templates_get_and_post() {
    let (app, token) = setup().await;

    let templates_payload = json!({
        "passwordReset": {
            "subject": "Password Reset for {{app_name}}",
            "bodyHtml": "<h1>Reset</h1><p>Use code: {{otp}}</p>",
            "bodyText": "Use code: {{otp}}"
        },
        "userRegistration": {
            "subject": "Welcome to {{app_name}}",
            "bodyHtml": "<p>Welcome!</p>",
            "bodyText": "Welcome!"
        }
    });

    // 1. Save templates
    let post_res = app
        .post_json(
            "/api/settings/email-templates",
            templates_payload,
            Some(&token),
        )
        .await;
    expect(post_res, StatusCode::OK).await;

    // 2. Fetch templates
    let get_res = app.get_auth("/api/settings/email-templates", &token).await;
    let body = expect(get_res, StatusCode::OK).await;

    assert_eq!(
        body["passwordReset"]["subject"],
        "Password Reset for {{app_name}}"
    );
    assert_eq!(
        body["userRegistration"]["subject"],
        "Welcome to {{app_name}}"
    );
}
