use crabbase_api::{routes::get_app_routes, state::AppState};
use crabbase_core::config::Config;
use sqlx::{Pool, Postgres};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, info_span};

pub async fn run_server(
    config: &Config,
    db_pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app_state = AppState { db: db_pool };

    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let listener = TcpListener::bind(&config.server_bind_addr).await?;
    axum::serve(listener, api).await?;

    Ok(())
}

pub async fn run_admin(
    config: &Config,
    _db_pool: Pool<Postgres>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Admin dashboard on {}", config.admin_bind_addr);

    // TODO: implement admin dashboard
    // Simulating admin command running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Admin dashboard...");

    Ok(())
}
