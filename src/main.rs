mod api;
mod auth;
mod core;
mod files;
mod hooks;
mod realtime;

use api::routes::get_app_routes;
use sqlx::migrate;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{error, info, info_span};

use crate::api::state::AppState;
use crate::core::db::connection::pool;
use crate::core::logging::init_logging;

#[tokio::main]
async fn main() {
    let _log_guard = init_logging();

    if let Err(err) = run().await {
        error!(error = %err, "application failed to start");
        std::process::exit(1)
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Crabbase Application...");
    let db_pool = pool().await?;

    migrate!("./migrations").run(&db_pool).await?;

    info!("Runinng migrations...");

    let app_state = AppState { db: db_pool };

    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let listener = TcpListener::bind("0.0.0.0:8000").await?;
    info!("Started server at http://0.0.0.0:8000");
    axum::serve(listener, api).await?;

    Ok(())
}
