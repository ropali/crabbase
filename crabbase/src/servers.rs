use axum::{
    Router,
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use crabbase_api::{get_app_routes, state::AppState};
use rust_embed::RustEmbed;
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, info_span};

use crate::config::Config;

#[derive(RustEmbed, Clone)]
#[folder = "../crates/admin-ui/dist"]
struct AdminAssets;

/// Body for an unknown `/api/*` route.
///
/// The SPA fallback must never answer for the API: returning `index.html` with
/// a 200 makes clients parse HTML as JSON and fail with serde's opaque
/// "expected value at line 1 column 1". In practice this means the server is
/// running a build older than the admin UI it is serving.
#[derive(serde::Serialize)]
struct ApiNotFound {
    code: &'static str,
    message: &'static str,
}

// Handler to serve static assets or fallback to index.html for SPA routes
async fn static_handler(uri: Uri) -> impl IntoResponse {
    if uri.path() == "/api" || uri.path().starts_with("/api/") {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(ApiNotFound {
                code: "NOT_FOUND",
                message: "Unknown API route. The server is probably running an older build \
                          than the admin UI it is serving.",
            }),
        )
            .into_response();
    }

    let path = uri.path().trim_start_matches('/');

    // Attempt to fetch file from embedded assets
    if let Some(content) = AdminAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();

        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data))
            .unwrap();
    }

    // SPA fallback: return embedded index.html for unknown paths
    if let Some(content) = AdminAssets::get("index.html") {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("404 Not Found"))
        .unwrap()
}

/// Read pool for the audit log target.
///
/// This is a separate pool from the writer's (see `PgLogWriter::spawn`) so a
/// slow admin page cannot starve the log pipeline, and separate from the
/// application pool because the logs may live in another database entirely.
/// A malformed URL already aborted startup in `run_log_migrations`, so a
/// failure here only means "no reader", not "stop the server".
fn logs_pool(config: &Config) -> Option<Pool<Postgres>> {
    let section = config.logging.database.as_ref().filter(|db| db.enabled)?;
    let setup = section.to_setup();

    crabbase_db::connection::pool(&setup.database_url, &setup.schema)
        .map_err(|err| eprintln!("Warning: log reader pool unavailable: {err}"))
        .ok()
}

pub async fn run_server(
    config: &Config,
    db_pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let display_addr = config.server_bind_addr.replace("0.0.0.0", "localhost");
    info!("Starting API server on http://{}/api", display_addr);
    info!("To access Admin Dashboard, run: cargo run -- admin (or make admin)");

    let app_state = AppState {
        db: db_pool,
        logs: logs_pool(config),
    };

    let cors = CorsLayer::new()
        // TODO: Allow specific origin (recommended for production)
        // .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        // Allow any origin (good for development)
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = get_app_routes(app_state).layer(cors).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let listener = TcpListener::bind(&config.server_bind_addr).await?;
    axum::serve(
        listener,
        api.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

pub async fn run_admin(
    config: &Config,
    db_pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let display_addr = config.admin_bind_addr.replace("0.0.0.0", "localhost");
    info!("Starting Admin dashboard on http://{}/", display_addr);
    info!("API endpoints available at http://{}/api", display_addr);

    let app_state = AppState {
        db: db_pool,
        logs: logs_pool(config),
    };
    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let app = Router::new()
        .route(
            "/admin",
            axum::routing::get(|| async { Redirect::temporary("/") }),
        )
        .route(
            "/admin/",
            axum::routing::get(|| async { Redirect::temporary("/") }),
        )
        .merge(api)
        .fallback(static_handler);

    let listener = TcpListener::bind(&config.admin_bind_addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
