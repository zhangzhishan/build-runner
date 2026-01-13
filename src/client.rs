use crate::protocol::{Request, Response};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub async fn run_build(dir: PathBuf, command: String, port: u16) -> Result<()> {
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

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let response: Response = serde_json::from_str(&line)?;

        match response {
            Response::Output { line, is_stderr } => {
                if is_stderr {
                    eprintln!("{}", line);
                } else {
                    println!("{}", line);
                }
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
