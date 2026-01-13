use crate::protocol::{Request, Response};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;

pub async fn run(init_script: Option<PathBuf>, port: u16) -> Result<()> {
    let initialized = Arc::new(AtomicBool::new(false));
    let init_script_path = init_script.clone();

    // Run init script if provided
    if let Some(ref script) = init_script {
        println!("Running init script: {}", script.display());
        run_init_script(script).await?;
        println!("Init script completed successfully.");
    }

    initialized.store(true, Ordering::SeqCst);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .context(format!("Failed to bind to port {}", port))?;

    println!("Build server listening on port {}...", port);
    println!("Ready to accept build requests.");

    let running = Arc::new(AtomicBool::new(true));

    while running.load(Ordering::SeqCst) {
        let (socket, addr) = listener.accept().await?;
        println!("Connection from: {}", addr);

        let running_clone = running.clone();
        let initialized_clone = initialized.clone();
        let init_script_clone = init_script_path.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                socket,
                running_clone,
                initialized_clone,
                init_script_clone,
            )
            .await
            {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }

    println!("Server shutting down...");
    Ok(())
}

async fn run_init_script(script: &PathBuf) -> Result<()> {
    let script_path = script.to_string_lossy();

    let status = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &script_path])
        .status()
        .await
        .context("Failed to run init script")?;

    if !status.success() {
        anyhow::bail!(
            "Init script failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

async fn handle_connection(
    mut socket: TcpStream,
    running: Arc<AtomicBool>,
    initialized: Arc<AtomicBool>,
    init_script: Option<PathBuf>,
) -> Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line).await?;
    let request: Request = serde_json::from_str(&line)?;

    match request {
        Request::Build { dir, command } => {
            println!("Build request: dir={}, cmd={}", dir.display(), command);
            handle_build(&mut writer, dir, command).await?;
        }
        Request::Status => {
            let response = Response::Status {
                initialized: initialized.load(Ordering::SeqCst),
                init_script: init_script.map(|p| p.to_string_lossy().to_string()),
            };
            send_response(&mut writer, &response).await?;
        }
        Request::Stop => {
            println!("Stop request received.");
            send_response(&mut writer, &Response::Stopping).await?;
            running.store(false, Ordering::SeqCst);
        }
    }

    Ok(())
}

async fn handle_build(
    writer: &mut tokio::net::tcp::WriteHalf<'_>,
    dir: PathBuf,
    command: String,
) -> Result<()> {
    // Parse command into program and args
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        send_response(
            writer,
            &Response::Error {
                message: "Empty command".to_string(),
            },
        )
        .await?;
        return Ok(());
    }

    // Spawn the build process
    let mut child = match Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!("cd '{}'; {}", dir.display(), command)])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            send_response(
                writer,
                &Response::Error {
                    message: format!("Failed to spawn process '{}': {}", program, e),
                },
            )
            .await?;
            return Ok(());
        }
    };

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Stream output to client
    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        send_response(writer, &Response::Output { line, is_stderr: false }).await?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("Error reading stdout: {}", e);
                        break;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        send_response(writer, &Response::Output { line, is_stderr: true }).await?;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Error reading stderr: {}", e);
                    }
                }
            }
        }
    }

    // Wait for process to complete
    let status = child.wait().await?;
    let exit_code = status.code().unwrap_or(-1);

    send_response(writer, &Response::BuildComplete { exit_code }).await?;
    println!("Build completed with exit code: {}", exit_code);

    Ok(())
}

async fn send_response(
    writer: &mut tokio::net::tcp::WriteHalf<'_>,
    response: &Response,
) -> Result<()> {
    let json = serde_json::to_string(response)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
