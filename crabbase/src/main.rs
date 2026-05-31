use clap::Parser;
use crabbase_api::routes::get_app_routes;
use sqlx::migrate;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{error, info, info_span};

use crabbase_api::state::AppState;
use crabbase_core::logging::init_logging;
use crabbase_db::connection::pool;

use crate::cli::{Cli, Commands};
pub mod cli;

#[tokio::main]
async fn main() {
    let _log_guard = init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, host } => {
            if let Err(err) = run_server(host, port).await {
                error!(error = %err, "Application failed to start.");
                std::process::exit(1)
            }
        }
        Commands::Admin { port, host } => {
            if let Err(err) = run_admin(host, port).await {
                error!(error = %err, "Admin dashboard failed to start.");
                std::process::exit(1)
            }
        }
    };
}

async fn run_server(
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Crabbase Application...");
    let db_pool = pool().await?;

    migrate!("../migrations").run(&db_pool).await?;

    info!("Running migrations...");

    let app_state = AppState { db: db_pool };

    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("Started server at http://{}:{}", host, port);
    axum::serve(listener, api).await?;

    Ok(())
}

async fn run_admin(
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Admin dashboard on {}:{}", host, port);

    // TODO: implement admin dashboard
    // Simulating admin command running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Admin dashboard...");

    Ok(())
}
