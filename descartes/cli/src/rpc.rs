//! RPC client for communicating with the Descartes daemon.
//!
//! This module provides helper functions for connecting to the daemon
//! and making RPC calls from CLI commands.

use anyhow::Result;
use descartes_core::DescaratesConfig;
use descartes_daemon::{UnixSocketRpcClient, UnixSocketRpcClientBuilder};
use std::path::PathBuf;

/// Get the daemon socket path from config
pub fn get_daemon_socket(config: &DescaratesConfig) -> PathBuf {
    PathBuf::from(format!("{}/run/daemon.sock", config.storage.base_path))
}

/// Check if daemon is running by checking if socket exists
#[allow(dead_code)]
pub fn is_daemon_running(config: &DescaratesConfig) -> bool {
    get_daemon_socket(config).exists()
}

/// Connect to daemon or bail with helpful error
///
/// This is the primary entry point for CLI commands that need to
/// communicate with the daemon.
pub async fn connect_or_bail(config: &DescaratesConfig) -> Result<UnixSocketRpcClient> {
    let socket_path = get_daemon_socket(config);

    if !socket_path.exists() {
        anyhow::bail!(
            "Daemon not running (socket not found at {:?}).\n\
             Start the daemon with 'descartes daemon' first.",
            socket_path
        );
    }

    let client = UnixSocketRpcClientBuilder::new()
        .socket_path(socket_path.clone())
        .timeout(30)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create RPC client: {}", e))?;

    // Test connection
    client.test_connection().await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to connect to daemon at {:?}: {}\n\
             Make sure the daemon is running with 'descartes daemon'.",
            socket_path,
            e
        )
    })?;

    Ok(client)
}

/// Make a raw RPC call and return the JSON result
///
/// This is a helper for making RPC calls that don't have dedicated
/// methods on UnixSocketRpcClient.
pub async fn call_method(
    client: &UnixSocketRpcClient,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    // We need to use the internal call method, but it's private.
    // Instead, we'll work around this by using tokio UnixStream directly.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let socket_path = client.socket_path();

    let mut stream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

    // Build JSON-RPC 2.0 request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });

    let request_str = serde_json::to_string(&request)?;

    // Send request with newline delimiter
    stream.write_all(request_str.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    // Read response with timeout
    let mut response_bytes = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        stream.read_to_end(&mut response_bytes),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Request timed out"))?
    .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))?;

    if response_bytes.is_empty() {
        anyhow::bail!("Empty response from daemon");
    }

    let response_str = String::from_utf8(response_bytes)?;
    let response: serde_json::Value = serde_json::from_str(&response_str)?;

    // Check for errors
    if let Some(error) = response.get("error") {
        let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        anyhow::bail!("RPC error ({}): {}", code, message);
    }

    // Extract result
    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing result field in response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_daemon_socket() {
        let config = DescaratesConfig {
            storage: descartes_core::StorageConfig {
                base_path: "/tmp/test".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let socket = get_daemon_socket(&config);
        assert_eq!(socket, PathBuf::from("/tmp/test/run/daemon.sock"));
    }
}
