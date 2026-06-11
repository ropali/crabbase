use clap::Parser;
use tracing::error;

use crabbase_core::{config::Config, logging::init_logging};

pub mod bootstrap;
pub mod errors;
pub mod servers;

use crate::{
    cli::{Cli, Commands},
    servers::{run_admin, run_server},
};
pub mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _log_guard = init_logging();

    let mut config = Config::load().expect("failed to load configuration.");

    let pool = bootstrap::bootstrap(&config).await?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, host } => {
            config.server_bind_addr = format!("{}:{}", host, port);

            if let Err(err) = run_server(&config, pool).await {
                error!(error = %err, "Application failed to start.");
                std::process::exit(1)
            }
        }
        Commands::Admin { port, host } => {
            config.admin_bind_addr = format!("{}:{}", host, port);
            if let Err(err) = run_admin(&config, pool).await {
                error!(error = %err, "Admin dashboard failed to start.");
                std::process::exit(1)
            }
        }
    };

    Ok(())
}
