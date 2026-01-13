//! Daemon auto-start and connection utilities.
//!
//! Provides a single global daemon per user at ~/.descartes/run/daemon.sock

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Default ports for the global daemon
pub const DEFAULT_HTTP_PORT: u16 = 19280;
pub const DEFAULT_WS_PORT: u16 = 19380;

/// Get the path to the daemon socket
pub fn daemon_socket_path() -> PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".descartes")
        .join("run");
    base.join("daemon.sock")
}

/// Get the daemon HTTP endpoint
pub fn daemon_http_endpoint() -> String {
    format!("http://127.0.0.1:{}", DEFAULT_HTTP_PORT)
}

/// Get the daemon WebSocket endpoint
pub fn daemon_ws_endpoint() -> String {
    format!("ws://127.0.0.1:{}", DEFAULT_WS_PORT)
}

/// Check if daemon is running by testing the health endpoint
pub async fn is_daemon_running() -> bool {
    let endpoint = daemon_http_endpoint();
    match reqwest::get(&format!("{}/health", endpoint)).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => {
            // Fallback: try root endpoint (daemon responds with server info)
            match reqwest::get(&endpoint).await {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            }
        }
    }
}

/// Ensure daemon is running, starting it if necessary.
/// Returns Ok(true) if daemon was started, Ok(false) if already running.
pub async fn ensure_daemon_running() -> Result<bool, String> {
    if is_daemon_running().await {
        tracing::debug!("Daemon already running");
        return Ok(false);
    }

    tracing::info!("Starting daemon...");
    start_daemon().await?;
    Ok(true)
}

/// Start the daemon process in the background
async fn start_daemon() -> Result<(), String> {
    // Ensure run directory exists
    let socket_path = daemon_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create daemon run directory: {}", e))?;
    }

    // Spawn daemon process
    let mut cmd = Command::new("descartes-daemon");
    cmd.arg("--http-port")
        .arg(DEFAULT_HTTP_PORT.to_string())
        .arg("--ws-port")
        .arg(DEFAULT_WS_PORT.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    // Detach from parent process
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn daemon: {}. Is descartes-daemon in PATH?", e))?;

    // Wait for daemon to become healthy
    let mut attempts = 0;
    while attempts < 30 {
        if is_daemon_running().await {
            tracing::info!("Daemon started successfully on port {}", DEFAULT_HTTP_PORT);
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }

    Err("Daemon failed to start within 3 seconds".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_socket_path() {
        let path = daemon_socket_path();
        assert!(path.ends_with("daemon.sock"));
        assert!(path.to_string_lossy().contains(".descartes/run"));
    }

    #[test]
    fn test_daemon_endpoints() {
        assert_eq!(daemon_http_endpoint(), "http://127.0.0.1:19280");
        assert_eq!(daemon_ws_endpoint(), "ws://127.0.0.1:19380");
    }

    #[test]
    fn test_default_ports() {
        assert_eq!(DEFAULT_HTTP_PORT, 19280);
        assert_eq!(DEFAULT_WS_PORT, 19380);
    }
}
