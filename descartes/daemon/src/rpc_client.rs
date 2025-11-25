//! Unix Socket RPC Client for Descartes Core
//!
//! This module provides a JSON-RPC 2.0 client that connects to the Descartes daemon
//! via Unix sockets using the jsonrpsee library.

use crate::errors::{DaemonError, DaemonResult};
use crate::rpc_server::{ApprovalResult, TaskInfo};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::ClientError;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::UnixStream;
use tracing::{debug, error, info};

/// Configuration for the RPC client
#[derive(Debug, Clone)]
pub struct RpcClientConfig {
    /// Path to the Unix socket
    pub socket_path: PathBuf,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        RpcClientConfig {
            socket_path: PathBuf::from("/tmp/descartes-rpc.sock"),
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

impl RpcClientConfig {
    /// Create a new configuration with the given socket path
    pub fn new(socket_path: PathBuf) -> Self {
        RpcClientConfig {
            socket_path,
            ..Default::default()
        }
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set max retries
    pub fn with_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Unix Socket RPC Client for Descartes
///
/// This client connects to the Descartes daemon via Unix socket and provides
/// a high-level API for interacting with the RPC server.
///
/// # Example
///
/// ```no_run
/// use descartes_daemon::UnixSocketRpcClient;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = UnixSocketRpcClient::new(PathBuf::from("/tmp/descartes-rpc.sock"))?;
///
///     // Spawn an agent
///     let agent_id = client.spawn("my-agent", "worker", serde_json::json!({})).await?;
///     println!("Spawned agent: {}", agent_id);
///
///     // List tasks
///     let tasks = client.list_tasks(None).await?;
///     println!("Found {} tasks", tasks.len());
///
///     Ok(())
/// }
/// ```
pub struct UnixSocketRpcClient {
    socket_path: PathBuf,
    timeout: Duration,
    max_retries: u32,
}

impl UnixSocketRpcClient {
    /// Create a new Unix socket RPC client
    pub fn new(socket_path: PathBuf) -> DaemonResult<Self> {
        Ok(UnixSocketRpcClient {
            socket_path,
            timeout: Duration::from_secs(30),
            max_retries: 3,
        })
    }

    /// Create a new client with configuration
    pub fn with_config(config: RpcClientConfig) -> DaemonResult<Self> {
        Ok(UnixSocketRpcClient {
            socket_path: config.socket_path,
            timeout: Duration::from_secs(config.timeout_secs),
            max_retries: config.max_retries,
        })
    }

    /// Create a default client (connects to /tmp/descartes-rpc.sock)
    pub fn default_client() -> DaemonResult<Self> {
        Self::with_config(RpcClientConfig::default())
    }

    /// Test connection to the server
    pub async fn test_connection(&self) -> DaemonResult<()> {
        info!("Testing connection to {:?}", self.socket_path);

        // Try to connect to the Unix socket
        UnixStream::connect(&self.socket_path).await.map_err(|e| {
            DaemonError::ConnectionError(format!(
                "Failed to connect to Unix socket {:?}: {}",
                self.socket_path, e
            ))
        })?;

        info!("Connection test successful");
        Ok(())
    }

    /// Low-level RPC call via Unix socket
    ///
    /// Since jsonrpsee doesn't natively support Unix sockets in the client,
    /// we implement a custom transport layer using tokio's UnixStream.
    async fn call(&self, method: &str, params: Value) -> DaemonResult<Value> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        debug!("RPC call: {} with params: {:?}", method, params);

        // Connect to Unix socket
        let mut stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            DaemonError::ConnectionError(format!(
                "Failed to connect to {:?}: {}",
                self.socket_path, e
            ))
        })?;

        // Build JSON-RPC 2.0 request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let request_str = serde_json::to_string(&request).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to serialize request: {}", e))
        })?;

        debug!("Sending request: {}", request_str);

        // Send request (with newline delimiter for framing)
        stream
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| DaemonError::ConnectionError(format!("Failed to send request: {}", e)))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|e| DaemonError::ConnectionError(format!("Failed to send request: {}", e)))?;
        stream
            .flush()
            .await
            .map_err(|e| DaemonError::ConnectionError(format!("Failed to flush: {}", e)))?;

        // Read response
        let mut response_bytes = Vec::new();
        let bytes_read =
            tokio::time::timeout(self.timeout, stream.read_to_end(&mut response_bytes))
                .await
                .map_err(|_| DaemonError::Timeout)?
                .map_err(|e| {
                    DaemonError::ConnectionError(format!("Failed to read response: {}", e))
                })?;

        if bytes_read == 0 {
            return Err(DaemonError::ConnectionError("Empty response".to_string()));
        }

        let response_str = String::from_utf8(response_bytes).map_err(|e| {
            DaemonError::SerializationError(format!("Invalid UTF-8 in response: {}", e))
        })?;

        debug!("Received response: {}", response_str);

        // Parse JSON-RPC response
        let response: serde_json::Value = serde_json::from_str(&response_str).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse response: {}", e))
        })?;

        // Check for errors
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            error!("RPC error: {} (code: {})", message, code);
            return Err(DaemonError::RpcError(code, message.to_string()));
        }

        // Extract result
        response
            .get("result")
            .cloned()
            .ok_or_else(|| DaemonError::SerializationError("Missing result field".to_string()))
    }

    /// Spawn a new agent
    ///
    /// # Arguments
    /// * `name` - The name of the agent
    /// * `agent_type` - The type of agent to spawn (e.g., "worker", "supervisor")
    /// * `config` - Additional configuration as JSON
    ///
    /// # Returns
    /// The ID of the spawned agent
    pub async fn spawn(&self, name: &str, agent_type: &str, config: Value) -> DaemonResult<String> {
        let params = serde_json::json!([name, agent_type, config]);
        let result = self.call("spawn", params).await?;

        result
            .as_str()
            .ok_or_else(|| DaemonError::SerializationError("Expected string result".to_string()))
            .map(|s| s.to_string())
    }

    /// List all tasks with optional filtering
    ///
    /// # Arguments
    /// * `filter` - Optional filter criteria as JSON
    ///   - `status`: Filter by status ("todo", "in_progress", "done", "blocked")
    ///   - `assigned_to`: Filter by assigned agent ID
    ///
    /// # Returns
    /// Vector of task information
    pub async fn list_tasks(&self, filter: Option<Value>) -> DaemonResult<Vec<TaskInfo>> {
        let params = serde_json::json!([filter]);
        let result = self.call("list_tasks", params).await?;

        serde_json::from_value(result)
            .map_err(|e| DaemonError::SerializationError(format!("Failed to parse tasks: {}", e)))
    }

    /// Approve or reject a task
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to approve
    /// * `approved` - True to approve, false to reject
    ///
    /// # Returns
    /// Approval result with timestamp
    pub async fn approve(&self, task_id: &str, approved: bool) -> DaemonResult<ApprovalResult> {
        let params = serde_json::json!([task_id, approved]);
        let result = self.call("approve", params).await?;

        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse approval result: {}", e))
        })
    }

    /// Get the current state of the system or a specific entity
    ///
    /// # Arguments
    /// * `entity_id` - Optional entity ID (agent ID) to query specific state
    ///
    /// # Returns
    /// The current state as JSON
    pub async fn get_state(&self, entity_id: Option<&str>) -> DaemonResult<Value> {
        let params = serde_json::json!([entity_id]);
        self.call("get_state", params).await
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }
}

/// Builder for Unix Socket RPC Client
pub struct UnixSocketRpcClientBuilder {
    config: RpcClientConfig,
}

impl UnixSocketRpcClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        UnixSocketRpcClientBuilder {
            config: RpcClientConfig::default(),
        }
    }

    /// Set the socket path
    pub fn socket_path(mut self, path: PathBuf) -> Self {
        self.config.socket_path = path;
        self
    }

    /// Set the timeout
    pub fn timeout(mut self, timeout_secs: u64) -> Self {
        self.config.timeout_secs = timeout_secs;
        self
    }

    /// Set max retries
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    /// Build the client
    pub fn build(self) -> DaemonResult<UnixSocketRpcClient> {
        UnixSocketRpcClient::with_config(self.config)
    }
}

impl Default for UnixSocketRpcClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = RpcClientConfig::default();
        assert_eq!(config.socket_path, PathBuf::from("/tmp/descartes-rpc.sock"));
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_client_config_builder() {
        let config = RpcClientConfig::new(PathBuf::from("/var/run/descartes.sock"))
            .with_timeout(60)
            .with_retries(5);

        assert_eq!(config.socket_path, PathBuf::from("/var/run/descartes.sock"));
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_builder() {
        let client = UnixSocketRpcClientBuilder::new()
            .socket_path(PathBuf::from("/tmp/test.sock"))
            .timeout(45)
            .max_retries(2)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.socket_path(), &PathBuf::from("/tmp/test.sock"));
    }
}
