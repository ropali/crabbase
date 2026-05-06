mod api;
mod auth;
mod core;
mod db;
mod files;
mod hooks;
mod realtime;

use api::routes::{AppState, build_router};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app_state = AppState {};

    let api = build_router(app_state);

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    eprintln!("Started server at http://0.0.0.0:8000");
    let _ = axum::serve(listener, api).await;
}
