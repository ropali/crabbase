use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "crabbase")]
#[command(about = "Crabbase CLI Tool.")]
pub struct Cli {
    #[arg(long, value_name = "FILE", default_value = "crabbase.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the backend API server
    Serve {
        /// Port to bind the API server to
        #[arg(long)]
        port: Option<u16>,

        /// Host address to bind the API server to
        #[arg(long)]
        host: Option<String>,
    },
    /// Start the admin dashboard
    Admin {
        /// Port to bind the admin server to
        #[arg(long)]
        port: Option<u16>,

        /// Host address to bind the admin server to
        #[arg(long)]
        host: Option<String>,
    },
}
