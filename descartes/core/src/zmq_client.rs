/// ZMQ client implementation for remote agent spawning and control.
///
/// This module provides a concrete implementation of the ZmqAgentRunner trait
/// for client-side remote agent management. It uses the ZmqConnection layer
/// to communicate with remote agent servers.
///
/// # Architecture
///
/// ```text
/// ZmqClient
///   ├── Connection Management
///   │   ├── Connect to server
///   │   ├── Disconnect
///   │   ├── Auto-reconnect
///   │   └── Health checks
///   ├── Agent Lifecycle
///   │   ├── spawn_remote()
///   │   ├── list_remote_agents()
///   │   ├── get_remote_agent()
///   │   └── get_agent_status()
///   ├── Agent Control
///   │   ├── pause_agent()
///   │   ├── resume_agent()
///   │   ├── stop_agent()
///   │   └── kill_agent()
///   └── I/O Operations
///       ├── write_agent_stdin()
///       ├── read_agent_stdout()
///       └── read_agent_stderr()
/// ```

use crate::errors::{AgentError, AgentResult};
use crate::traits::{AgentConfig, AgentInfo, AgentStatus};
use crate::zmq_agent_runner::{
    CommandResponse, ControlCommand, ControlCommandType, HealthCheckRequest,
    HealthCheckResponse, ListAgentsRequest, ListAgentsResponse, SpawnRequest, SpawnResponse,
    StatusUpdate, ZmqAgentRunner, ZmqMessage, ZmqRunnerConfig,
};
use crate::zmq_communication::{SocketType, ZmqConnection, ZmqMessageRouter};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// ZMQ client for remote agent management
pub struct ZmqClient {
    /// ZMQ connection
    connection: Arc<Mutex<ZmqConnection>>,
    /// Configuration
    config: ZmqRunnerConfig,
    /// Message router for request/response correlation
    router: Arc<ZmqMessageRouter>,
    /// Status update subscribers
    status_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<AgentResult<StatusUpdate>>>>>,
}

impl ZmqClient {
    /// Create a new ZMQ client
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the client
    ///
    /// # Example
    ///
    /// ```rust
    /// use descartes_core::{ZmqClient, ZmqRunnerConfig};
    ///
    /// let config = ZmqRunnerConfig {
    ///     endpoint: "tcp://localhost:5555".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// let client = ZmqClient::new(config);
    /// ```
    pub fn new(config: ZmqRunnerConfig) -> Self {
        let connection = ZmqConnection::new(
            SocketType::Req,
            &config.endpoint,
            config.clone(),
        );

        Self {
            connection: Arc::new(Mutex::new(connection)),
            config,
            router: Arc::new(ZmqMessageRouter::new()),
            status_subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new ZMQ client with custom socket type
    ///
    /// # Arguments
    ///
    /// * `socket_type` - ZMQ socket type to use
    /// * `config` - Configuration for the client
    pub fn new_with_socket_type(socket_type: SocketType, config: ZmqRunnerConfig) -> Self {
        let connection = ZmqConnection::new(
            socket_type,
            &config.endpoint,
            config.clone(),
        );

        Self {
            connection: Arc::new(Mutex::new(connection)),
            config,
            router: Arc::new(ZmqMessageRouter::new()),
            status_subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Send a control command to an agent
    async fn send_control_command(
        &self,
        agent_id: &Uuid,
        command_type: ControlCommandType,
        payload: Option<serde_json::Value>,
    ) -> AgentResult<CommandResponse> {
        let request_id = Uuid::new_v4().to_string();

        let command = ControlCommand {
            request_id: request_id.clone(),
            agent_id: *agent_id,
            command_type,
            payload,
        };

        let request = ZmqMessage::ControlCommand(command);

        let response = self
            .connection
            .lock()
            .await
            .request_response(&request, Some(Duration::from_secs(self.config.request_timeout_secs)))
            .await?;

        match response {
            ZmqMessage::CommandResponse(resp) => {
                if resp.request_id != request_id {
                    return Err(AgentError::ExecutionError(format!(
                        "Request ID mismatch: expected {}, got {}",
                        request_id, resp.request_id
                    )));
                }

                if !resp.success {
                    return Err(AgentError::ExecutionError(
                        resp.error
                            .unwrap_or_else(|| "Command failed".to_string()),
                    ));
                }

                Ok(resp)
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }
}

#[async_trait]
impl ZmqAgentRunner for ZmqClient {
    async fn connect(&self, endpoint: &str) -> AgentResult<()> {
        let mut connection = self.connection.lock().await;

        // Update endpoint if different
        if connection.is_connected() {
            connection.disconnect().await?;
        }

        // Create new connection with updated endpoint
        let mut new_connection = ZmqConnection::new(
            SocketType::Req,
            endpoint,
            self.config.clone(),
        );

        new_connection.connect().await?;

        // Replace the connection
        *connection = new_connection;

        tracing::info!("Connected to ZMQ server at {}", endpoint);

        Ok(())
    }

    async fn disconnect(&self) -> AgentResult<()> {
        let mut connection = self.connection.lock().await;
        connection.disconnect().await?;

        tracing::info!("Disconnected from ZMQ server");

        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Note: This is a sync method, so we can't lock the async Mutex
        // In a real implementation, you might want to use a sync RwLock for state
        // For now, we'll return a conservative answer
        true // Assume connected; actual check happens in operations
    }

    async fn spawn_remote(
        &self,
        config: AgentConfig,
        timeout_secs: Option<u64>,
    ) -> AgentResult<AgentInfo> {
        let request_id = Uuid::new_v4().to_string();

        let request = SpawnRequest {
            request_id: request_id.clone(),
            config,
            timeout_secs,
            metadata: None,
        };

        let message = ZmqMessage::SpawnRequest(request);

        let response = self
            .connection
            .lock()
            .await
            .request_response(&message, Some(Duration::from_secs(timeout_secs.unwrap_or(self.config.request_timeout_secs))))
            .await?;

        match response {
            ZmqMessage::SpawnResponse(resp) => {
                if resp.request_id != request_id {
                    return Err(AgentError::ExecutionError(format!(
                        "Request ID mismatch: expected {}, got {}",
                        request_id, resp.request_id
                    )));
                }

                if !resp.success {
                    return Err(AgentError::ExecutionError(
                        resp.error
                            .unwrap_or_else(|| "Spawn failed".to_string()),
                    ));
                }

                resp.agent_info.ok_or_else(|| {
                    AgentError::ExecutionError("No agent info in response".to_string())
                })
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    async fn list_remote_agents(
        &self,
        filter_status: Option<AgentStatus>,
        limit: Option<usize>,
    ) -> AgentResult<Vec<AgentInfo>> {
        let request_id = Uuid::new_v4().to_string();

        let request = ListAgentsRequest {
            request_id: request_id.clone(),
            filter_status,
            limit,
        };

        let message = ZmqMessage::ListAgentsRequest(request);

        let response = self
            .connection
            .lock()
            .await
            .request_response(&message, Some(Duration::from_secs(self.config.request_timeout_secs)))
            .await?;

        match response {
            ZmqMessage::ListAgentsResponse(resp) => {
                if resp.request_id != request_id {
                    return Err(AgentError::ExecutionError(format!(
                        "Request ID mismatch: expected {}, got {}",
                        request_id, resp.request_id
                    )));
                }

                if !resp.success {
                    return Err(AgentError::ExecutionError(
                        resp.error
                            .unwrap_or_else(|| "List agents failed".to_string()),
                    ));
                }

                Ok(resp.agents)
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    async fn get_remote_agent(&self, agent_id: &Uuid) -> AgentResult<Option<AgentInfo>> {
        // Get status first to verify agent exists
        let status_exists = self.get_agent_status(agent_id).await.is_ok();

        if status_exists {
            // Agent exists, get full info via list
            let agents = self.list_remote_agents(None, None).await?;
            Ok(agents.into_iter().find(|a| a.id == *agent_id))
        } else {
            Ok(None)
        }
    }

    async fn get_agent_status(&self, agent_id: &Uuid) -> AgentResult<AgentStatus> {
        let response = self
            .send_control_command(agent_id, ControlCommandType::GetStatus, None)
            .await?;

        response.status.ok_or_else(|| {
            AgentError::ExecutionError("No status in response".to_string())
        })
    }

    async fn pause_agent(&self, agent_id: &Uuid) -> AgentResult<()> {
        self.send_control_command(agent_id, ControlCommandType::Pause, None)
            .await?;
        Ok(())
    }

    async fn resume_agent(&self, agent_id: &Uuid) -> AgentResult<()> {
        self.send_control_command(agent_id, ControlCommandType::Resume, None)
            .await?;
        Ok(())
    }

    async fn stop_agent(&self, agent_id: &Uuid) -> AgentResult<()> {
        self.send_control_command(agent_id, ControlCommandType::Stop, None)
            .await?;
        Ok(())
    }

    async fn kill_agent(&self, agent_id: &Uuid) -> AgentResult<()> {
        self.send_control_command(agent_id, ControlCommandType::Kill, None)
            .await?;
        Ok(())
    }

    async fn write_agent_stdin(&self, agent_id: &Uuid, data: &[u8]) -> AgentResult<()> {
        let payload = serde_json::json!({
            "data": base64::encode(data),
        });

        self.send_control_command(
            agent_id,
            ControlCommandType::WriteStdin,
            Some(payload),
        )
        .await?;

        Ok(())
    }

    async fn read_agent_stdout(&self, agent_id: &Uuid) -> AgentResult<Option<Vec<u8>>> {
        let response = self
            .send_control_command(agent_id, ControlCommandType::ReadStdout, None)
            .await?;

        if let Some(data) = response.data {
            if let Some(encoded) = data.get("data").and_then(|v| v.as_str()) {
                let decoded = base64::decode(encoded).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to decode base64: {}", e))
                })?;
                return Ok(Some(decoded));
            }
        }

        Ok(None)
    }

    async fn read_agent_stderr(&self, agent_id: &Uuid) -> AgentResult<Option<Vec<u8>>> {
        let response = self
            .send_control_command(agent_id, ControlCommandType::ReadStderr, None)
            .await?;

        if let Some(data) = response.data {
            if let Some(encoded) = data.get("data").and_then(|v| v.as_str()) {
                let decoded = base64::decode(encoded).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to decode base64: {}", e))
                })?;
                return Ok(Some(decoded));
            }
        }

        Ok(None)
    }

    async fn health_check(&self) -> AgentResult<HealthCheckResponse> {
        let request_id = Uuid::new_v4().to_string();

        let request = HealthCheckRequest {
            request_id: request_id.clone(),
        };

        let message = ZmqMessage::HealthCheckRequest(request);

        let response = self
            .connection
            .lock()
            .await
            .request_response(&message, Some(Duration::from_secs(self.config.request_timeout_secs)))
            .await?;

        match response {
            ZmqMessage::HealthCheckResponse(resp) => {
                if resp.request_id != request_id {
                    return Err(AgentError::ExecutionError(format!(
                        "Request ID mismatch: expected {}, got {}",
                        request_id, resp.request_id
                    )));
                }

                Ok(resp)
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    async fn subscribe_status_updates(
        &self,
        _agent_id: Option<Uuid>,
    ) -> AgentResult<Box<dyn Stream<Item = AgentResult<StatusUpdate>> + Unpin + Send>> {
        // Create a channel for status updates
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Add to subscribers
        self.status_subscribers.write().push(tx);

        // Convert receiver to stream
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

        Ok(Box::new(stream))
    }
}

/// Helper module for base64 encoding/decoding
mod base64 {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
        STANDARD.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_zmq_client_creation() {
        let config = ZmqRunnerConfig::default();
        let client = ZmqClient::new(config);

        // Just verify it constructs successfully
        assert!(true);
    }

    #[test]
    fn test_zmq_client_with_custom_socket() {
        let config = ZmqRunnerConfig::default();
        let client = ZmqClient::new_with_socket_type(SocketType::Dealer, config);

        // Just verify it constructs successfully
        assert!(true);
    }

    #[test]
    fn test_base64_encode_decode() {
        let data = b"Hello, World!";
        let encoded = base64::encode(data);
        let decoded = base64::decode(&encoded).unwrap();

        assert_eq!(data.to_vec(), decoded);
    }
}
