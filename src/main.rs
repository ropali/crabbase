mod api;
mod auth;
mod core;
mod files;
mod hooks;
mod realtime;

use api::routes::get_app_routes;
use sqlx::migrate;
use tokio::net::TcpListener;

use crate::api::state::AppState;
use crate::core::db::connection::pool;

#[tokio::main]
async fn main() {
    let db_pool = pool().await.unwrap();

    migrate!("./migrations").run(&db_pool).await.unwrap();

    let app_state = AppState { db: db_pool };

    let api = get_app_routes(app_state);

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    eprintln!("Started server at http://0.0.0.0:8000");
    let _ = axum::serve(listener, api).await;
}
