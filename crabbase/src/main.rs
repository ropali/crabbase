use clap::Parser;
use sqlx::PgPool;
use tracing::error;

use crabbase_core::logging::init_logging;

pub mod bootstrap;
pub mod config;
pub mod errors;
pub mod servers;

use crate::{
    cli::{Cli, Commands},
    config::Config,
    servers::{run_admin, run_server},
};
pub mod cli;

fn print_banner() {
    println!(
        "
        ██████╗██████╗  █████╗ ██████╗ ██████╗  █████╗ ███████╗███████╗
       ██╔════╝██╔══██╗██╔══██╗██╔══██╗██╔══██╗██╔══██╗██╔════╝██╔════╝
       ██║     ██████╔╝███████║██████╔╝██████╔╝███████║███████╗█████╗
       ██║     ██╔══██╗██╔══██║██╔══██╗██╔══██╗██╔══██║╚════██║██╔══╝
       ╚██████╗██║  ██║██║  ██║██████╔╝██████╔╝██║  ██║███████║███████╗
        ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚══════╝
    "
    );
}

fn load_config(config: &std::path::Path) -> Config {
    match Config::load(Some(config)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

async fn bootstrap_or_exit(cfg: &Config) -> PgPool {
    match bootstrap::bootstrap(cfg).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn database_parts(cfg: &Config) -> crabbase_jobs::DatabaseParts {
    crabbase_jobs::DatabaseParts {
        host: cfg.database.host.clone(),
        port: cfg.database.port,
        user: cfg.database.user.clone(),
        password: cfg.database.password.clone(),
        database: cfg.database.database.clone(),
    }
}

async fn start_runtime(cfg: &Config, pool: &PgPool) -> crabbase_jobs::JobRuntime {
    if let Err(e) = crabbase_jobs::preflight(pool).await {
        eprintln!("Backup preflight failed: {e}");
        std::process::exit(1);
    }

    match crabbase_jobs::start(pool.clone(), database_parts(cfg)).await {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("Failed to start job runtime: {e}");
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    print_banner();
    let _log_guard = init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config } => {
            let cfg = load_config(&config);
            let pool = bootstrap_or_exit(&cfg).await;
            let jobs = start_runtime(&cfg, &pool).await;

            if let Err(err) = run_server(&cfg, pool).await {
                error!(error = %err, "Application failed to start.");
                std::process::exit(1);
            }

            jobs.shutdown().await;
        }
        Commands::Admin { config } => {
            let cfg = load_config(&config);
            let pool = bootstrap_or_exit(&cfg).await;
            let jobs = start_runtime(&cfg, &pool).await;

            if let Err(err) = run_admin(&cfg, pool).await {
                error!(error = %err, "Admin dashboard failed to start.");
                std::process::exit(1);
            }

            jobs.shutdown().await;
        }
        Commands::Worker { config } => {
            let cfg = load_config(&config);
            let pool = bootstrap_or_exit(&cfg).await;
            let jobs = start_runtime(&cfg, &pool).await;

            if let Err(err) = tokio::signal::ctrl_c().await {
                error!(error = %err, "failed to listen for shutdown signal");
            }

            jobs.shutdown().await;
        }
    }
}
