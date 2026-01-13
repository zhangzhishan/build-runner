use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request from client to server
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Execute a build command
    Build {
        /// Working directory
        dir: PathBuf,
        /// Command to execute
        command: String,
    },
    /// Check server status
    Status,
    /// Stop the server
    Stop,
}

/// Response from server to client
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Build output line (stdout or stderr)
    Output {
        line: String,
        is_stderr: bool,
    },
    /// Build completed
    BuildComplete {
        exit_code: i32,
    },
    /// Server status
    Status {
        initialized: bool,
        init_script: Option<String>,
    },
    /// Server is stopping
    Stopping,
    /// Error occurred
    Error {
        message: String,
    },
}
