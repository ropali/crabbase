//! Shared test infrastructure for all crabbase_api integration tests.
//!
//! This module provides:
//! - `TestDb`   : a schema-isolated Postgres pool that auto-drops when the test ends
//! - `TestApp`  : a fully-wired Axum router + typed HTTP helpers
//! - Fixture builders: `insert_superuser`, `login`, `create_base_collection`, `create_record`

#![allow(dead_code)]

use axum::http::{Method, Request, Response, StatusCode, Uri};
use axum::{Router, body::Body};
use crabbase_api::{get_app_routes, state::AppState};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use tower::ServiceExt;

// ─── Database isolation ──────────────────────────────────────────────────────

/// Wraps a Postgres pool bound to a unique schema.
/// The schema is dropped when this struct is dropped.
pub struct TestDb {
    pub pool: Pool<Postgres>,
    pub schema: String,
    root_pool: Pool<Postgres>,
}

impl TestDb {
    pub async fn new(schema_prefix: &str) -> Self {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

        // Make schema name unique per-test so tests run in parallel safely
        let schema = format!("{}_{}", schema_prefix, uuid::Uuid::new_v4().simple());

        let root_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&db_url)
            .await
            .expect("root pool connect");

        let ident = format!("\"{}\"", schema);
        let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE", ident))
            .execute(&root_pool)
            .await;
        sqlx::query(&format!("CREATE SCHEMA {}", ident))
            .execute(&root_pool)
            .await
            .expect("create test schema");

        let mut opts: sqlx::postgres::PgConnectOptions = db_url.parse().expect("parse db url");
        opts = opts.options([("search_path", schema.as_str())]);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .expect("test pool connect");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        Self {
            pool,
            schema,
            root_pool,
        }
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let pool = self.root_pool.clone();
        let ident = format!("\"{}\"", self.schema);
        tokio::spawn(async move {
            let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE", ident))
                .execute(&pool)
                .await;
        });
    }
}

/// A fully-wired Axum router scoped to an isolated test DB.
pub struct TestApp {
    pub router: Router,
    pub db: TestDb,
}

impl TestApp {
    pub async fn new(schema_prefix: &str) -> Self {
        let db = TestDb::new(schema_prefix).await;
        let state = AppState {
            db: db.pool.clone(),
        };
        let router = get_app_routes(state);
        Self { router, db }
    }

    /// Fire any request through the router in-process (no port binding).
    pub async fn request(&self, req: Request<Body>) -> Response<axum::body::Body> {
        self.router
            .clone()
            .oneshot(req)
            .await
            .expect("router oneshot")
    }

    /// GET without auth
    pub async fn get(&self, path: &str) -> Response<axum::body::Body> {
        let uri = safe_uri(path);
        self.request(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .expect("failed to send request"),
        )
        .await
    }

    /// GET with Bearer token
    pub async fn get_auth(&self, path: &str, token: &str) -> Response<axum::body::Body> {
        let uri = safe_uri(path);
        self.request(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    /// POST JSON body (token optional)
    pub async fn post_json(
        &self,
        path: &str,
        body: Value,
        token: Option<&str>,
    ) -> Response<axum::body::Body> {
        let uri = safe_uri(path);
        let mut b = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("Content-Type", "application/json");
        if let Some(t) = token {
            b = b.header("Authorization", format!("Bearer {t}"));
        }
        self.request(b.body(Body::from(body.to_string())).unwrap())
            .await
    }

    /// PATCH JSON body with Bearer token
    pub async fn patch_json(
        &self,
        path: &str,
        body: Value,
        token: &str,
    ) -> Response<axum::body::Body> {
        let uri = safe_uri(path);
        self.request(
            Request::builder()
                .method(Method::PATCH)
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    /// DELETE with Bearer token
    pub async fn delete_auth(&self, path: &str, token: &str) -> Response<axum::body::Body> {
        let uri = safe_uri(path);
        self.request(
            Request::builder()
                .method(Method::DELETE)
                .uri(uri)
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }
}

fn safe_uri(path: &str) -> String {
    if path.parse::<Uri>().is_ok() {
        return path.to_string();
    }
    if let Some((base, query)) = path.split_once('?') {
        let mut encoded_query = String::new();
        for byte in query.bytes() {
            match byte {
                b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'='
                | b'&'
                | b'/'
                | b'?'
                | b':'
                | b'@'
                | b'!'
                | b'$'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'%' => {
                    encoded_query.push(byte as char);
                }
                _ => {
                    use std::fmt::Write;
                    let _ = write!(encoded_query, "%{:02X}", byte);
                }
            }
        }
        format!("{base}?{encoded_query}")
    } else {
        path.to_string()
    }
}

// ─── Response helpers ─────────────────────────────────────────────────────────

/// Read entire body bytes and parse as JSON.
pub async fn body_json(res: Response<axum::body::Body>) -> Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(json!(null))
}

/// Assert response has `expected` status, then return parsed body.
/// On mismatch the test fails with a readable message showing the actual body.
pub async fn expect(res: Response<axum::body::Body>, expected: StatusCode) -> Value {
    let status = res.status();
    let body = body_json(res).await;
    assert_eq!(
        status, expected,
        "expected HTTP {expected}, got {status}. body: {body:#}"
    );
    body
}

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// Insert a superuser row directly via SQL (bypasses the API).
/// The migration already seeds a `_superusers` collection entry in `_collections`
/// with secret `"my-secret-key"`, so the JWT signing just works.
///
/// Returns the new user's UUID as a String.
pub async fn insert_superuser(pool: &Pool<Postgres>, email: &str, password_plain: &str) -> String {
    // Cost 4 = fastest bcrypt in tests while still being real bcrypt
    let hashed = bcrypt::hash(password_plain, 4).expect("bcrypt hash");
    let token_key = format!("tk_{}", uuid::Uuid::new_v4().simple());

    sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO _superusers (email, password, token_key, verified)
         VALUES ($1, $2, $3, true)
         RETURNING id",
    )
    .bind(email)
    .bind(&hashed)
    .bind(&token_key)
    .fetch_one(pool)
    .await
    .expect("insert superuser")
    .to_string()
}

/// Call the HTTP login endpoint and return the `accessToken` string.
/// Panics if login fails.
pub async fn login(app: &TestApp, collection: &str, email: &str, password: &str) -> String {
    let res = app
        .post_json(
            &format!("/api/auth/{collection}/login"),
            json!({ "email": email, "password": password }),
            None,
        )
        .await;

    let body = expect(res, StatusCode::OK).await;
    body["tokens"]["accessToken"]
        .as_str()
        .expect("accessToken present in login response")
        .to_string()
}

/// Create a collection via the API using a superuser token.
/// Returns the response body (the created Collection JSON).
pub async fn create_collection(
    app: &TestApp,
    name: &str,
    columns: Vec<Value>,
    token: &str,
) -> Value {
    let res = app
        .post_json(
            "/api/collections",
            json!({ "name": name, "columns": columns }),
            Some(token),
        )
        .await;
    expect(res, StatusCode::OK).await
}

/// Create a record via the API (no auth — for public/base collections in tests).
/// Returns the created Record JSON.
pub async fn create_record_api(app: &TestApp, collection: &str, data: Value) -> Value {
    let res = app
        .post_json(
            &format!("/api/collections/{collection}/records"),
            json!({ "data": data }),
            None,
        )
        .await;
    expect(res, StatusCode::OK).await
}

/// Create a record with an auth token.
pub async fn create_record_auth(
    app: &TestApp,
    collection: &str,
    data: Value,
    token: &str,
) -> Value {
    let res = app
        .post_json(
            &format!("/api/collections/{collection}/records"),
            json!({ "data": data }),
            Some(token),
        )
        .await;
    expect(res, StatusCode::OK).await
}

/// Standard column definitions for quick test collection setup.
pub fn text_col(name: &str) -> Value {
    json!({ "name": name, "type": "text", "index": false })
}

pub fn number_col(name: &str) -> Value {
    json!({ "name": name, "type": "number", "index": false })
}

pub fn bool_col(name: &str) -> Value {
    json!({ "name": name, "type": "bool", "index": false })
}
