use crate::protocol::{Request, Response};
use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Output line with metadata for truncation
struct OutputLine {
    content: String,
    is_stderr: bool,
}

/// Smart output buffer that keeps first N/2 and last N/2 lines
struct TruncatingBuffer {
    max_lines: usize,
    head: Vec<OutputLine>,
    tail: VecDeque<OutputLine>,
    total_count: usize,
    head_limit: usize,
    tail_limit: usize,
}

impl TruncatingBuffer {
    fn new(max_lines: usize) -> Self {
        let head_limit = max_lines / 2;
        let tail_limit = max_lines - head_limit;
        Self {
            max_lines,
            head: Vec::with_capacity(head_limit),
            tail: VecDeque::with_capacity(tail_limit + 1),
            total_count: 0,
            head_limit,
            tail_limit,
        }
    }

    fn push(&mut self, line: OutputLine) {
        self.total_count += 1;

        if self.max_lines == 0 {
            // No truncation - print immediately
            Self::print_line(&line);
            return;
        }

        if self.head.len() < self.head_limit {
            // Still filling head buffer - print and store
            Self::print_line(&line);
            self.head.push(line);
        } else {
            // Head is full, add to tail ring buffer
            if self.tail.len() >= self.tail_limit {
                self.tail.pop_front();
            }
            self.tail.push_back(line);
        }
    }

    fn finish(self) {
        if self.max_lines == 0 {
            return;
        }

        let skipped = self.total_count.saturating_sub(self.head.len() + self.tail.len());

        if skipped > 0 {
            eprintln!();
            eprintln!("... [{} lines truncated] ...", skipped);
            eprintln!();

            // Print the tail (wasn't printed in real-time)
            for line in self.tail {
                Self::print_line(&line);
            }
        } else if self.total_count > self.head.len() {
            // No truncation but we have tail lines that weren't printed
            for line in self.tail {
                Self::print_line(&line);
            }
        }
    }

    fn print_line(line: &OutputLine) {
        if line.is_stderr {
            eprintln!("{}", line.content);
        } else {
            println!("{}", line.content);
        }
    }
}

pub async fn run_build(dir: PathBuf, command: String, port: u16, max_lines: usize) -> Result<()> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .context(format!(
            "Failed to connect to build server on port {}. Is the server running?",
            port
        ))?;

    let request = Request::Build { dir, command };
    send_request(&mut stream, &request).await?;

    let (reader, _) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    let mut exit_code = 0;
    let mut buffer = TruncatingBuffer::new(max_lines);

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let response: Response = serde_json::from_str(&line)?;

        match response {
            Response::Output {
                line: content,
                is_stderr,
            } => {
                buffer.push(OutputLine { content, is_stderr });
            }
            Response::BuildComplete { exit_code: code } => {
                exit_code = code;
                break;
            }
            Response::Error { message } => {
                eprintln!("Error: {}", message);
                std::process::exit(1);
            }
            _ => {}
        }
    }

    buffer.finish();

    if exit_code != 0 {
        eprintln!("\nBuild failed with exit code: {}", exit_code);
    }

    std::process::exit(exit_code);
}

pub async fn check_status(port: u16) -> Result<()> {
    let mut stream = match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
        Ok(s) => s,
        Err(_) => {
            println!("Build server is NOT running on port {}", port);
            return Ok(());
        }
    };

    send_request(&mut stream, &Request::Status).await?;

    let (reader, _) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: Response = serde_json::from_str(&line)?;

    match response {
        Response::Status {
            initialized,
            init_script,
        } => {
            println!("Build server is running on port {}", port);
            println!("  Initialized: {}", initialized);
            if let Some(script) = init_script {
                println!("  Init script: {}", script);
            }
        }
        _ => {
            println!("Unexpected response from server");
        }
    }

    Ok(())
}

pub async fn stop_server(port: u16) -> Result<()> {
    let mut stream = match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
        Ok(s) => s,
        Err(_) => {
            println!("Build server is not running on port {}", port);
            return Ok(());
        }
    };

    send_request(&mut stream, &Request::Stop).await?;

    let (reader, _) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: Response = serde_json::from_str(&line)?;

    match response {
        Response::Stopping => {
            println!("Build server is stopping...");
        }
        _ => {
            println!("Unexpected response from server");
        }
    }

    Ok(())
}

async fn send_request(stream: &mut TcpStream, request: &Request) -> Result<()> {
    let json = serde_json::to_string(request)?;
    stream.write_all(json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;
    Ok(())
}
