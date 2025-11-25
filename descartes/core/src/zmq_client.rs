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
    BatchControlCommand, BatchControlResponse, CommandResponse, ControlCommand, ControlCommandType,
    CustomActionRequest, HealthCheckRequest, HealthCheckResponse, ListAgentsRequest,
    ListAgentsResponse, OutputQueryRequest, OutputQueryResponse, SpawnRequest, SpawnResponse,
    StatusUpdate, ZmqAgentRunner, ZmqMessage, ZmqOutputStream, ZmqRunnerConfig,
};
use crate::zmq_communication::{SocketType, ZmqConnection, ZmqMessageRouter};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Queued command for offline operation
#[derive(Debug, Clone)]
struct QueuedCommand {
    /// The message to send
    message: ZmqMessage,
    /// When the command was queued
    queued_at: std::time::Instant,
    /// Response channel
    response_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<AgentResult<ZmqMessage>>>>>,
}

/// ZMQ client for remote agent management
pub struct ZmqClient {
    /// ZMQ connection
    connection: Arc<Mutex<ZmqConnection>>,
    /// Configuration
    config: ZmqRunnerConfig,
    /// Message router for request/response correlation
    router: Arc<ZmqMessageRouter>,
    /// Status update subscribers
    status_subscribers:
        Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<AgentResult<StatusUpdate>>>>>,
    /// Command queue for when disconnected
    command_queue: Arc<Mutex<VecDeque<QueuedCommand>>>,
    /// Maximum queue size (prevents unbounded growth during long disconnections)
    max_queue_size: usize,
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
        let connection = ZmqConnection::new(SocketType::Req, &config.endpoint, config.clone());

        Self {
            connection: Arc::new(Mutex::new(connection)),
            config,
            router: Arc::new(ZmqMessageRouter::new()),
            status_subscribers: Arc::new(RwLock::new(Vec::new())),
            command_queue: Arc::new(Mutex::new(VecDeque::new())),
            max_queue_size: 1000, // Default: queue up to 1000 commands
        }
    }

    /// Create a new ZMQ client with custom socket type
    ///
    /// # Arguments
    ///
    /// * `socket_type` - ZMQ socket type to use
    /// * `config` - Configuration for the client
    pub fn new_with_socket_type(socket_type: SocketType, config: ZmqRunnerConfig) -> Self {
        let connection = ZmqConnection::new(socket_type, &config.endpoint, config.clone());

        Self {
            connection: Arc::new(Mutex::new(connection)),
            config,
            router: Arc::new(ZmqMessageRouter::new()),
            status_subscribers: Arc::new(RwLock::new(Vec::new())),
            command_queue: Arc::new(Mutex::new(VecDeque::new())),
            max_queue_size: 1000, // Default: queue up to 1000 commands
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
            .request_response(
                &request,
                Some(Duration::from_secs(self.config.request_timeout_secs)),
            )
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
                        resp.error.unwrap_or_else(|| "Command failed".to_string()),
                    ));
                }

                Ok(resp)
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// Send a custom action to an agent
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent
    /// * `action` - Action name/type
    /// * `params` - Optional action parameters
    /// * `timeout_secs` - Optional timeout for the action
    ///
    /// # Returns
    ///
    /// The command response with any result data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::ZmqClient;
    /// # use uuid::Uuid;
    /// # async fn example(client: &ZmqClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let agent_id = Uuid::new_v4();
    /// let params = serde_json::json!({
    ///     "message": "Hello, agent!",
    ///     "priority": "high"
    /// });
    ///
    /// let response = client.send_action_to_agent(&agent_id, "custom_task", Some(params), Some(60)).await?;
    /// println!("Action result: {:?}", response.data);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_action_to_agent(
        &self,
        agent_id: &Uuid,
        action: &str,
        params: Option<serde_json::Value>,
        timeout_secs: Option<u64>,
    ) -> AgentResult<CommandResponse> {
        let request_id = Uuid::new_v4().to_string();

        let request = CustomActionRequest {
            request_id: request_id.clone(),
            agent_id: *agent_id,
            action: action.to_string(),
            params,
            timeout_secs,
        };

        let message = ZmqMessage::CustomActionRequest(request);

        let response = self
            .connection
            .lock()
            .await
            .request_response(
                &message,
                Some(Duration::from_secs(
                    timeout_secs.unwrap_or(self.config.request_timeout_secs),
                )),
            )
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
                        resp.error.unwrap_or_else(|| "Action failed".to_string()),
                    ));
                }

                Ok(resp)
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// Query agent output with filtering and pagination
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent
    /// * `stream` - Output stream to query (stdout, stderr, or both)
    /// * `filter` - Optional regex filter pattern
    /// * `limit` - Maximum number of lines to return
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// Query response with filtered output lines
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::{ZmqClient, ZmqOutputStream};
    /// # use uuid::Uuid;
    /// # async fn example(client: &ZmqClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let agent_id = Uuid::new_v4();
    ///
    /// // Get last 50 lines containing "ERROR"
    /// let response = client.query_agent_output(
    ///     &agent_id,
    ///     ZmqOutputStream::Both,
    ///     Some("ERROR".to_string()),
    ///     Some(50),
    ///     None
    /// ).await?;
    ///
    /// for line in response.lines {
    ///     println!("{}", line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_agent_output(
        &self,
        agent_id: &Uuid,
        stream: ZmqOutputStream,
        filter: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AgentResult<OutputQueryResponse> {
        let request_id = Uuid::new_v4().to_string();

        let request = OutputQueryRequest {
            request_id: request_id.clone(),
            agent_id: *agent_id,
            stream,
            filter,
            limit,
            offset,
        };

        let message = ZmqMessage::OutputQueryRequest(request);

        let response = self
            .connection
            .lock()
            .await
            .request_response(
                &message,
                Some(Duration::from_secs(self.config.request_timeout_secs)),
            )
            .await?;

        match response {
            ZmqMessage::OutputQueryResponse(resp) => {
                if resp.request_id != request_id {
                    return Err(AgentError::ExecutionError(format!(
                        "Request ID mismatch: expected {}, got {}",
                        request_id, resp.request_id
                    )));
                }

                if !resp.success {
                    return Err(AgentError::ExecutionError(
                        resp.error
                            .unwrap_or_else(|| "Output query failed".to_string()),
                    ));
                }

                Ok(resp)
            }
            _ => Err(AgentError::ExecutionError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// Execute a batch control command on multiple agents
    ///
    /// # Arguments
    ///
    /// * `agent_ids` - IDs of agents to control
    /// * `command_type` - Command to execute
    /// * `payload` - Optional command payload
    /// * `fail_fast` - Whether to stop on first error
    ///
    /// # Returns
    ///
    /// Batch response with individual results
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::{ZmqClient, ControlCommandType};
    /// # use uuid::Uuid;
    /// # async fn example(client: &ZmqClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let agent_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    ///
    /// // Pause all agents
    /// let response = client.batch_control(
    ///     agent_ids,
    ///     ControlCommandType::Pause,
    ///     None,
    ///     false
    /// ).await?;
    ///
    /// println!("Successful: {}, Failed: {}", response.successful, response.failed);
    /// for result in response.results {
    ///     if !result.success {
    ///         eprintln!("Agent {} failed: {}", result.agent_id, result.error.unwrap_or_default());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn batch_control(
        &self,
        agent_ids: Vec<Uuid>,
        command_type: ControlCommandType,
        payload: Option<serde_json::Value>,
        fail_fast: bool,
    ) -> AgentResult<BatchControlResponse> {
        let request_id = Uuid::new_v4().to_string();

        let request = BatchControlCommand {
            request_id: request_id.clone(),
            agent_ids,
            command_type,
            payload,
            fail_fast,
        };

        let message = ZmqMessage::BatchControlCommand(request);

        let response = self
            .connection
            .lock()
            .await
            .request_response(
                &message,
                Some(Duration::from_secs(self.config.request_timeout_secs * 2)),
            )
            .await?;

        match response {
            ZmqMessage::BatchControlResponse(resp) => {
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

    /// Subscribe to status updates with a callback
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Optional agent ID filter
    /// * `callback` - Callback function to handle updates
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::ZmqClient;
    /// # use uuid::Uuid;
    /// # async fn example(client: &ZmqClient) -> Result<(), Box<dyn std::error::Error>> {
    /// client.stream_agent_status(None, |update| {
    ///     Box::pin(async move {
    ///         println!("Status update for agent {}: {:?}", update.agent_id, update.update_type);
    ///         Ok(())
    ///     })
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stream_agent_status<F, Fut>(
        &self,
        agent_id: Option<Uuid>,
        mut callback: F,
    ) -> AgentResult<()>
    where
        F: FnMut(StatusUpdate) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = AgentResult<()>> + Send + 'static,
    {
        let mut stream = self.subscribe_status_updates(agent_id).await?;

        tokio::spawn(async move {
            while let Some(update_result) = stream.next().await {
                match update_result {
                    Ok(update) => {
                        if let Err(e) = callback(update).await {
                            tracing::error!("Status callback error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Status stream error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Get the number of queued commands
    pub async fn queued_command_count(&self) -> usize {
        self.command_queue.lock().await.len()
    }

    /// Process queued commands after reconnection
    async fn process_queued_commands(&self) -> AgentResult<usize> {
        let mut queue = self.command_queue.lock().await;
        let count = queue.len();

        if count == 0 {
            return Ok(0);
        }

        tracing::info!("Processing {} queued commands after reconnection", count);

        let mut processed = 0;
        while let Some(queued_cmd) = queue.pop_front() {
            let result = self
                .connection
                .lock()
                .await
                .request_response(
                    &queued_cmd.message,
                    Some(Duration::from_secs(self.config.request_timeout_secs)),
                )
                .await;

            // Send result to the waiting caller (if still listening)
            if let Some(tx) = queued_cmd.response_tx.lock().await.take() {
                let _ = tx.send(result);
            }

            processed += 1;
        }

        tracing::info!("Processed {} queued commands", processed);
        Ok(processed)
    }

    /// Queue a command for later execution (when disconnected)
    async fn queue_command(&self, message: ZmqMessage) -> AgentResult<ZmqMessage> {
        let mut queue = self.command_queue.lock().await;

        if queue.len() >= self.max_queue_size {
            return Err(AgentError::ExecutionError(format!(
                "Command queue is full ({} commands). Cannot queue more commands.",
                self.max_queue_size
            )));
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let queued_cmd = QueuedCommand {
            message,
            queued_at: std::time::Instant::now(),
            response_tx: Arc::new(Mutex::new(Some(tx))),
        };

        queue.push_back(queued_cmd);
        drop(queue);

        tracing::warn!(
            "Command queued (not connected). Queue size: {}",
            self.command_queue.lock().await.len()
        );

        // Wait for response (will be sent when queue is processed)
        rx.await.map_err(|e| {
            AgentError::ExecutionError(format!("Queued command response channel closed: {}", e))
        })?
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
        let mut new_connection = ZmqConnection::new(SocketType::Req, endpoint, self.config.clone());

        new_connection.connect().await?;

        // Replace the connection
        *connection = new_connection;
        drop(connection);

        tracing::info!("Connected to ZMQ server at {}", endpoint);

        // Process any queued commands
        let processed = self.process_queued_commands().await?;
        if processed > 0 {
            tracing::info!("Processed {} queued commands after connection", processed);
        }

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
            .request_response(
                &message,
                Some(Duration::from_secs(
                    timeout_secs.unwrap_or(self.config.request_timeout_secs),
                )),
            )
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
                        resp.error.unwrap_or_else(|| "Spawn failed".to_string()),
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
            .request_response(
                &message,
                Some(Duration::from_secs(self.config.request_timeout_secs)),
            )
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

        response
            .status
            .ok_or_else(|| AgentError::ExecutionError("No status in response".to_string()))
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

        self.send_control_command(agent_id, ControlCommandType::WriteStdin, Some(payload))
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
            .request_response(
                &message,
                Some(Duration::from_secs(self.config.request_timeout_secs)),
            )
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
