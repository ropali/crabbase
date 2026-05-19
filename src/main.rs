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
use tracing::{info, info_span};

use crate::api::state::AppState;
use crate::core::db::connection::pool;
use crate::core::logging::init_logging;

#[tokio::main]
async fn main() {
    let _log_guard = init_logging();
    info!("Starting Crabbase Application...");
    let db_pool = pool().await.unwrap();

    migrate!("./migrations").run(&db_pool).await.unwrap();

    info!("Runinng migrations...");

    let app_state = AppState { db: db_pool };

    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Started server at http://0.0.0.0:8000");
    let _ = axum::serve(listener, api).await;
}
