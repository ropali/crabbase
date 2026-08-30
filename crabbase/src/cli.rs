use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "crabbase")]
#[command(about = "Crabbase CLI Tool.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the backend API server
    Serve {
        /// Config file location
        #[arg(long, value_name = "FILE", default_value = "crabbase.toml")]
        config: PathBuf,
    },
    /// Start the admin dashboard
    Admin {
        /// Config file location
        #[arg(long, value_name = "FILE", default_value = "crabbase.toml")]
        config: PathBuf,
    },
}
