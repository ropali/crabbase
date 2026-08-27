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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _log_guard = init_logging();

    let cli = Cli::parse();
    let mut config = Config::load(Some(&cli.config))?;

    match cli.command {
        Commands::Serve { port, host } => {
            if let (Some(host), Some(port)) = (host, port) {
                config.server_bind_addr = format!("{host}:{port}");
            }
            let pool = bootstrap::bootstrap(&config).await?;

            if let Err(err) = run_server(&config, pool).await {
                error!(error = %err, "Application failed to start.");
                std::process::exit(1)
            }
        }
        Commands::Admin { port, host } => {
            if let (Some(host), Some(port)) = (host, port) {
                config.admin_bind_addr = format!("{host}:{port}");
            }
            let pool = bootstrap::bootstrap(&config).await?;
            if let Err(err) = run_admin(&config, pool).await {
                error!(error = %err, "Admin dashboard failed to start.");
                std::process::exit(1)
            }
        }
    };

    Ok(())
}
