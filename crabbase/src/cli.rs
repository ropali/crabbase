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
        /// Port to bind the API server to
        #[arg(long, default_value = "8989")]
        port: u16,

        /// Host address to bind the API server to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
    /// Start the admin dashboard
    Admin {
        /// Port to bind the admin server to
        #[arg(long, default_value = "8181")]
        port: u16,

        /// Host address to bind the admin server to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
}
