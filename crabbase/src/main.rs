use std::path::{Path, PathBuf};

use clap::Parser;
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

#[tokio::main]
async fn main() {
    let _log_guard = init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config } => {
            let cfg = match Config::load(Some(&config)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            let pool = match bootstrap::bootstrap(&cfg).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = run_server(&cfg, pool).await {
                error!(error = %err, "Application failed to start.");
                std::process::exit(1);
            }
        }
        Commands::Admin { config } => {
            let cfg = match Config::load(Some(&config)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            let pool = match bootstrap::bootstrap(&cfg).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = run_admin(&cfg, pool).await {
                error!(error = %err, "Admin dashboard failed to start.");
                std::process::exit(1);
            }
        }
    }
}
