mod server;
mod client;
mod protocol;

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "build-runner")]
#[command(about = "A client-server build runner that maintains initialized shell environment")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the build server (run this in your initialized terminal)
    Server {
        /// Path to init script to run on startup (optional)
        #[arg(short, long)]
        init: Option<PathBuf>,

        /// Port to listen on
        #[arg(short, long, default_value = "19527")]
        port: u16,
    },

    /// Send a build request to the server
    Run {
        /// Working directory for the build
        #[arg(short = 'd', long)]
        dir: PathBuf,

        /// Build command to execute (default: "quickbuild debug")
        #[arg(short, long, default_value = "quickbuild debug")]
        command: String,

        /// Port to connect to
        #[arg(short, long, default_value = "19527")]
        port: u16,
    },

    /// Check if the server is running
    Status {
        /// Port to check
        #[arg(short, long, default_value = "19527")]
        port: u16,
    },

    /// Stop the server
    Stop {
        /// Port to connect to
        #[arg(short, long, default_value = "19527")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { init, port } => {
            server::run(init, port).await?;
        }
        Commands::Run { dir, command, port } => {
            client::run_build(dir, command, port).await?;
        }
        Commands::Status { port } => {
            client::check_status(port).await?;
        }
        Commands::Stop { port } => {
            client::stop_server(port).await?;
        }
    }

    Ok(())
}
