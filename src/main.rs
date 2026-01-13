mod client;
mod protocol;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};
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

        /// Maximum number of output lines to display (0 = unlimited).
        /// When truncating, keeps first N/2 and last N/2 lines.
        #[arg(short = 'l', long, default_value = "500")]
        max_lines: usize,

        /// Show all output without truncation
        #[arg(long, default_value = "false")]
        no_truncate: bool,
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
        Commands::Run {
            dir,
            command,
            port,
            max_lines,
            no_truncate,
        } => {
            let limit = if no_truncate { 0 } else { max_lines };
            client::run_build(dir, command, port, limit).await?;
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
