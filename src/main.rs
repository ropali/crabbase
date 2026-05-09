mod api;
mod auth;
mod core;
mod db;
mod files;
mod hooks;
mod realtime;

use api::routes::get_app_routes;
use tokio::net::TcpListener;

use crate::api::state::AppState;
use crate::api::store::CollectionStore;

#[tokio::main]
async fn main() {
    let app_state = AppState {
        store: CollectionStore::new(),
    };

    let api = get_app_routes(app_state);

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    eprintln!("Started server at http://0.0.0.0:8000");
    let _ = axum::serve(listener, api).await;
}
