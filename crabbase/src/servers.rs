use axum::{
    Router,
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use crabbase_api::{get_app_routes, state::AppState};
use crabbase_core::config::Config;
use rust_embed::RustEmbed;
use sqlx::{Pool, Postgres};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, info_span};

#[derive(RustEmbed, Clone)]
#[folder = "../crates/admin-ui/dist"]
struct AdminAssets;

// Handler to serve static assets or fallback to index.html for SPA routes
async fn static_handler(uri: Uri) -> impl IntoResponse {
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

pub async fn run_server(
    config: &Config,
    db_pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app_state = AppState { db: db_pool };

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
    axum::serve(listener, api).await?;

    Ok(())
}

pub async fn run_admin(
    config: &Config,
    db_pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Admin dashboard on {}", config.admin_bind_addr);

    let app_state = AppState { db: db_pool };
    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let app = Router::new().nest("/api", api).fallback(static_handler);

    let listener = TcpListener::bind(&config.admin_bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
