/// ZeroMQ-based remote agent spawning and control for distributed agent orchestration.
///
/// This module provides the infrastructure for spawning and controlling agents across
/// remote machines using ZeroMQ as the transport layer. It defines:
/// - Message schemas for ZMQ communication (spawn, control, status)
/// - The ZmqAgentRunner trait for remote agent management
/// - Serialization/deserialization using MessagePack for efficiency
/// - Request/response patterns for reliable communication
///
/// # Architecture
///
/// The ZMQ agent runner uses a REQ/REP or DEALER/ROUTER pattern:
/// - Client sends spawn/control requests to remote server
/// - Server spawns agents locally and sends back responses
/// - Status updates can be pushed to clients via PUB/SUB
///
/// # Message Flow
///
/// ```text
/// Client                          Server
///   |                               |
///   |-- SpawnRequest -------------->|
///   |                               |-- Spawn local agent
///   |<-- SpawnResponse -------------|
///   |                               |
///   |-- ControlCommand ------------>|
///   |                               |-- Execute command
///   |<-- CommandResponse -----------|
///   |                               |
///   |<-- StatusUpdate --------------|  (async push)
/// ```
use crate::errors::{AgentError, AgentResult};
use crate::traits::{AgentConfig, AgentInfo, AgentStatus};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Message version for protocol compatibility checking
pub const ZMQ_PROTOCOL_VERSION: &str = "1.0.0";

/// Maximum message size (10 MB)
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Default timeout for ZMQ operations (30 seconds)
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// Message Schemas
// ============================================================================

/// Request to spawn a new agent on a remote server.
///
/// # Example
///
/// ```rust
/// use descartes_core::{SpawnRequest, AgentConfig};
/// use std::collections::HashMap;
///
/// let request = SpawnRequest {
///     request_id: uuid::Uuid::new_v4().to_string(),
///     config: AgentConfig {
///         name: "remote-agent".to_string(),
///         model_backend: "claude".to_string(),
///         task: "Write code".to_string(),
///         context: None,
///         system_prompt: None,
///         environment: HashMap::new(),
///         ..Default::default()
///     },
///     timeout_secs: Some(300),
///     metadata: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Unique identifier for this request
    pub request_id: String,

    /// Configuration for the agent to spawn
    pub config: AgentConfig,

    /// Optional timeout for the spawn operation (seconds)
    #[serde(default)]
    pub timeout_secs: Option<u64>,

    /// Optional metadata for tracking/logging
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Response to a spawn request.
///
/// # Example
///
/// ```rust
/// use descartes_core::{SpawnResponse, AgentInfo, AgentStatus};
/// use std::time::SystemTime;
/// use uuid::Uuid;
///
/// let response = SpawnResponse {
///     request_id: "req-123".to_string(),
///     success: true,
///     agent_info: Some(AgentInfo {
///         id: Uuid::new_v4(),
///         name: "remote-agent".to_string(),
///         status: AgentStatus::Running,
///         model_backend: "claude".to_string(),
///         started_at: SystemTime::now(),
///         task: "Write code".to_string(),
///         paused_at: None,
///         pause_mode: None,
///         attach_info: None,
///     }),
///     error: None,
///     server_id: Some("server-01".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResponse {
    /// The request ID this is responding to
    pub request_id: String,

    /// Whether the spawn was successful
    pub success: bool,

    /// Information about the spawned agent (if successful)
    #[serde(default)]
    pub agent_info: Option<AgentInfo>,

    /// Error message (if unsuccessful)
    #[serde(default)]
    pub error: Option<String>,

    /// Identifier of the server that spawned the agent
    #[serde(default)]
    pub server_id: Option<String>,
}

/// Types of control commands that can be sent to agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlCommandType {
    /// Pause the agent
    Pause,

    /// Resume a paused agent
    Resume,

    /// Stop the agent gracefully
    Stop,

    /// Kill the agent immediately
    Kill,

    /// Send input to the agent's stdin
    WriteStdin,

    /// Read from the agent's stdout
    ReadStdout,

    /// Read from the agent's stderr
    ReadStderr,

    /// Get current agent status
    GetStatus,

    /// Send a custom signal to the agent
    Signal,

    /// Send a custom action to the agent
    CustomAction,

    /// Query agent output with filtering
    QueryOutput,

    /// Stream agent logs
    StreamLogs,
}

/// Control command to send to an agent.
///
/// # Example
///
/// ```rust
/// use descartes_core::{ControlCommand, ControlCommandType};
/// use uuid::Uuid;
///
/// // Pause an agent
/// let pause_cmd = ControlCommand {
///     request_id: uuid::Uuid::new_v4().to_string(),
///     agent_id: Uuid::new_v4(),
///     command_type: ControlCommandType::Pause,
///     payload: None,
/// };
///
/// // Write to stdin
/// let stdin_cmd = ControlCommand {
///     request_id: uuid::Uuid::new_v4().to_string(),
///     agent_id: Uuid::new_v4(),
///     command_type: ControlCommandType::WriteStdin,
///     payload: Some(serde_json::json!({
///         "data": "Hello, agent!\n"
///     })),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCommand {
    /// Unique identifier for this request
    pub request_id: String,

    /// ID of the agent to control
    pub agent_id: Uuid,

    /// Type of control command
    pub command_type: ControlCommandType,

    /// Optional payload for the command (e.g., stdin data, signal type)
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

/// Response to a control command.
///
/// # Example
///
/// ```rust
/// use descartes_core::{CommandResponse, AgentStatus};
/// use uuid::Uuid;
///
/// let response = CommandResponse {
///     request_id: "req-123".to_string(),
///     agent_id: Uuid::new_v4(),
///     success: true,
///     status: Some(AgentStatus::Paused),
///     data: None,
///     error: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    /// The request ID this is responding to
    pub request_id: String,

    /// ID of the agent
    pub agent_id: Uuid,

    /// Whether the command was successful
    pub success: bool,

    /// Current status of the agent
    #[serde(default)]
    pub status: Option<AgentStatus>,

    /// Optional data (e.g., stdout/stderr content)
    #[serde(default)]
    pub data: Option<serde_json::Value>,

    /// Error message (if unsuccessful)
    #[serde(default)]
    pub error: Option<String>,
}

/// Asynchronous status update pushed from server to client.
///
/// # Example
///
/// ```rust
/// use descartes_core::{StatusUpdate, StatusUpdateType, AgentStatus};
/// use uuid::Uuid;
///
/// let update = StatusUpdate {
///     agent_id: Uuid::new_v4(),
///     update_type: StatusUpdateType::StatusChanged,
///     status: Some(AgentStatus::Completed),
///     message: Some("Agent completed successfully".to_string()),
///     data: None,
///     timestamp: std::time::SystemTime::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    /// ID of the agent this update is about
    pub agent_id: Uuid,

    /// Type of status update
    pub update_type: StatusUpdateType,

    /// Current status of the agent
    #[serde(default)]
    pub status: Option<AgentStatus>,

    /// Optional message
    #[serde(default)]
    pub message: Option<String>,

    /// Optional data payload
    #[serde(default)]
    pub data: Option<serde_json::Value>,

    /// Timestamp of the update
    pub timestamp: SystemTime,
}

/// Types of status updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusUpdateType {
    /// Agent status changed
    StatusChanged,

    /// Agent output available
    OutputAvailable,

    /// Agent error occurred
    Error,

    /// Agent completed
    Completed,

    /// Agent terminated
    Terminated,

    /// Heartbeat/keepalive
    Heartbeat,
}

/// Log stream message for real-time log streaming via PUB/SUB.
///
/// This message type is used to stream agent stdout/stderr output
/// in real-time to subscribed clients.
///
/// # Example
///
/// ```rust
/// use descartes_core::{LogStreamMessage, LogStreamType};
/// use uuid::Uuid;
/// use std::time::SystemTime;
///
/// let msg = LogStreamMessage {
///     agent_id: Uuid::new_v4(),
///     stream_type: LogStreamType::Stdout,
///     data: b"Hello from agent\n".to_vec(),
///     timestamp: SystemTime::now(),
///     sequence: 1,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStreamMessage {
    /// ID of the agent producing the log
    pub agent_id: Uuid,

    /// Type of output stream (stdout or stderr)
    pub stream_type: LogStreamType,

    /// The log data (raw bytes, typically UTF-8 text)
    pub data: Vec<u8>,

    /// Timestamp when the log was produced
    pub timestamp: SystemTime,

    /// Sequence number for ordering (monotonically increasing per agent/stream)
    pub sequence: u64,
}

/// Type of log stream (stdout or stderr).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogStreamType {
    /// Standard output stream
    Stdout,

    /// Standard error stream
    Stderr,
}

/// Request to list agents on a remote server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsRequest {
    /// Unique identifier for this request
    pub request_id: String,

    /// Optional filter by status
    #[serde(default)]
    pub filter_status: Option<AgentStatus>,

    /// Optional limit on number of agents to return
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Response to a list agents request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsResponse {
    /// The request ID this is responding to
    pub request_id: String,

    /// Whether the request was successful
    pub success: bool,

    /// List of agents (if successful)
    #[serde(default)]
    pub agents: Vec<AgentInfo>,

    /// Error message (if unsuccessful)
    #[serde(default)]
    pub error: Option<String>,
}

/// Health check request to verify server is responsive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRequest {
    /// Unique identifier for this request
    pub request_id: String,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// The request ID this is responding to
    pub request_id: String,

    /// Whether the server is healthy
    pub healthy: bool,

    /// Protocol version
    pub protocol_version: String,

    /// Server uptime in seconds
    #[serde(default)]
    pub uptime_secs: Option<u64>,

    /// Number of active agents
    #[serde(default)]
    pub active_agents: Option<usize>,

    /// Server metadata
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Custom action request for sending arbitrary commands to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomActionRequest {
    /// Unique identifier for this request
    pub request_id: String,

    /// ID of the agent to send action to
    pub agent_id: Uuid,

    /// Action name/type
    pub action: String,

    /// Action parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,

    /// Optional timeout for the action
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Batch control command for multiple agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchControlCommand {
    /// Unique identifier for this batch request
    pub request_id: String,

    /// Agent IDs to control
    pub agent_ids: Vec<Uuid>,

    /// Command to execute on all agents
    pub command_type: ControlCommandType,

    /// Optional payload for the command
    #[serde(default)]
    pub payload: Option<serde_json::Value>,

    /// Whether to fail fast or continue on errors
    #[serde(default)]
    pub fail_fast: bool,
}

/// Batch control response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchControlResponse {
    /// The request ID this is responding to
    pub request_id: String,

    /// Overall success (true if all succeeded)
    pub success: bool,

    /// Individual results per agent
    pub results: Vec<BatchAgentResult>,

    /// Number of successful operations
    pub successful: usize,

    /// Number of failed operations
    pub failed: usize,
}

/// Result for a single agent in a batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAgentResult {
    /// Agent ID
    pub agent_id: Uuid,

    /// Whether this agent's operation succeeded
    pub success: bool,

    /// Current status (if available)
    #[serde(default)]
    pub status: Option<AgentStatus>,

    /// Error message (if failed)
    #[serde(default)]
    pub error: Option<String>,
}

/// Output query request for retrieving agent output with filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputQueryRequest {
    /// Unique identifier for this request
    pub request_id: String,

    /// Agent ID to query
    pub agent_id: Uuid,

    /// Output stream to query (stdout, stderr, or both)
    pub stream: ZmqOutputStream,

    /// Optional filter pattern (regex)
    #[serde(default)]
    pub filter: Option<String>,

    /// Maximum number of lines to return
    #[serde(default)]
    pub limit: Option<usize>,

    /// Offset for pagination
    #[serde(default)]
    pub offset: Option<usize>,
}

/// ZMQ output stream type (for querying agent output)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZmqOutputStream {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
    /// Both stdout and stderr
    Both,
}

/// Output query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputQueryResponse {
    /// The request ID this is responding to
    pub request_id: String,

    /// Agent ID
    pub agent_id: Uuid,

    /// Whether the query was successful
    pub success: bool,

    /// Output lines
    #[serde(default)]
    pub lines: Vec<String>,

    /// Total number of lines available
    #[serde(default)]
    pub total_lines: Option<usize>,

    /// Whether there are more lines available
    #[serde(default)]
    pub has_more: bool,

    /// Error message (if unsuccessful)
    #[serde(default)]
    pub error: Option<String>,
}

/// Envelope for all ZMQ messages to support multiplexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ZmqMessage {
    /// Spawn request
    SpawnRequest(SpawnRequest),

    /// Spawn response
    SpawnResponse(SpawnResponse),

    /// Control command
    ControlCommand(ControlCommand),

    /// Command response
    CommandResponse(CommandResponse),

    /// Status update
    StatusUpdate(StatusUpdate),

    /// List agents request
    ListAgentsRequest(ListAgentsRequest),

    /// List agents response
    ListAgentsResponse(ListAgentsResponse),

    /// Health check request
    HealthCheckRequest(HealthCheckRequest),

    /// Health check response
    HealthCheckResponse(HealthCheckResponse),

    /// Custom action request
    CustomActionRequest(CustomActionRequest),

    /// Batch control command
    BatchControlCommand(BatchControlCommand),

    /// Batch control response
    BatchControlResponse(BatchControlResponse),

    /// Output query request
    OutputQueryRequest(OutputQueryRequest),

    /// Output query response
    OutputQueryResponse(OutputQueryResponse),

    /// Log stream message (for PUB/SUB streaming)
    LogStream(LogStreamMessage),
}

// ============================================================================
// Traits
// ============================================================================

/// ZmqAgentRunner trait for spawning and controlling agents on remote servers via ZeroMQ.
///
/// This trait extends the base AgentRunner functionality to support remote agent execution
/// across distributed systems. Implementations handle ZMQ socket management, message
/// serialization, and reliable communication patterns.
///
/// # Example
///
/// ```rust,no_run
/// use descartes_core::{ZmqAgentRunner, AgentConfig};
/// use std::collections::HashMap;
///
/// async fn example(runner: &impl ZmqAgentRunner) -> Result<(), Box<dyn std::error::Error>> {
///     // Connect to remote server
///     runner.connect("tcp://192.168.1.100:5555").await?;
///
///     // Spawn remote agent
///     let config = AgentConfig {
///         name: "remote-agent".to_string(),
///         model_backend: "claude".to_string(),
///         task: "Analyze logs".to_string(),
///         context: None,
///         system_prompt: None,
///         environment: HashMap::new(),
///         ..Default::default()
///     };
///
///     let agent_info = runner.spawn_remote(config, None).await?;
///     println!("Spawned agent: {:?}", agent_info);
///
///     // Get status
///     let status = runner.get_agent_status(&agent_info.id).await?;
///     println!("Agent status: {:?}", status);
///
///     // Stop agent
///     runner.stop_agent(&agent_info.id).await?;
///
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait ZmqAgentRunner: Send + Sync {
    /// Connect to a remote ZMQ server.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - ZMQ endpoint (e.g., "tcp://host:port", "ipc:///tmp/agents.sock")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::ZmqAgentRunner;
    /// # async fn example(runner: &impl ZmqAgentRunner) -> Result<(), Box<dyn std::error::Error>> {
    /// runner.connect("tcp://localhost:5555").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn connect(&self, endpoint: &str) -> AgentResult<()>;

    /// Disconnect from the remote ZMQ server.
    async fn disconnect(&self) -> AgentResult<()>;

    /// Check if connected to a remote server.
    fn is_connected(&self) -> bool;

    /// Spawn an agent on the remote server.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the agent
    /// * `timeout` - Optional timeout for the spawn operation
    ///
    /// # Returns
    ///
    /// Information about the spawned agent
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::{ZmqAgentRunner, AgentConfig};
    /// # use std::collections::HashMap;
    /// # async fn example(runner: &impl ZmqAgentRunner) -> Result<(), Box<dyn std::error::Error>> {
    /// let config = AgentConfig {
    ///     name: "test-agent".to_string(),
    ///     model_backend: "claude".to_string(),
    ///     task: "Test task".to_string(),
    ///     context: None,
    ///     system_prompt: None,
    ///     environment: HashMap::new(),
    ///     ..Default::default()
    /// };
    ///
    /// let agent = runner.spawn_remote(config, Some(30)).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn spawn_remote(
        &self,
        config: AgentConfig,
        timeout_secs: Option<u64>,
    ) -> AgentResult<AgentInfo>;

    /// List all agents on the remote server.
    ///
    /// # Arguments
    ///
    /// * `filter_status` - Optional status filter
    /// * `limit` - Optional limit on number of results
    async fn list_remote_agents(
        &self,
        filter_status: Option<AgentStatus>,
        limit: Option<usize>,
    ) -> AgentResult<Vec<AgentInfo>>;

    /// Get information about a specific remote agent.
    async fn get_remote_agent(&self, agent_id: &Uuid) -> AgentResult<Option<AgentInfo>>;

    /// Get the current status of a remote agent.
    async fn get_agent_status(&self, agent_id: &Uuid) -> AgentResult<AgentStatus>;

    /// Pause a remote agent.
    async fn pause_agent(&self, agent_id: &Uuid) -> AgentResult<()>;

    /// Resume a paused remote agent.
    async fn resume_agent(&self, agent_id: &Uuid) -> AgentResult<()>;

    /// Stop a remote agent gracefully.
    async fn stop_agent(&self, agent_id: &Uuid) -> AgentResult<()>;

    /// Kill a remote agent immediately.
    async fn kill_agent(&self, agent_id: &Uuid) -> AgentResult<()>;

    /// Write data to a remote agent's stdin.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent
    /// * `data` - Data to write to stdin
    async fn write_agent_stdin(&self, agent_id: &Uuid, data: &[u8]) -> AgentResult<()>;

    /// Read data from a remote agent's stdout.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent
    ///
    /// # Returns
    ///
    /// Available output data, or None if no data available
    async fn read_agent_stdout(&self, agent_id: &Uuid) -> AgentResult<Option<Vec<u8>>>;

    /// Read data from a remote agent's stderr.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent
    ///
    /// # Returns
    ///
    /// Available error data, or None if no data available
    async fn read_agent_stderr(&self, agent_id: &Uuid) -> AgentResult<Option<Vec<u8>>>;

    /// Send a health check to the remote server.
    ///
    /// # Returns
    ///
    /// Health check response with server information
    async fn health_check(&self) -> AgentResult<HealthCheckResponse>;

    /// Subscribe to status updates from the remote server.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Optional agent ID to filter updates (None for all agents)
    ///
    /// # Returns
    ///
    /// A stream of status updates
    async fn subscribe_status_updates(
        &self,
        agent_id: Option<Uuid>,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<StatusUpdate>> + Unpin + Send>>;
}

// ============================================================================
// Serialization Utilities
// ============================================================================

/// Serialize a ZmqMessage to MessagePack bytes.
///
/// # Example
///
/// ```rust
/// use descartes_core::{serialize_zmq_message, ZmqMessage, HealthCheckRequest};
///
/// let msg = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
///     request_id: "test-123".to_string(),
/// });
///
/// let bytes = serialize_zmq_message(&msg)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn serialize_zmq_message(msg: &ZmqMessage) -> AgentResult<Vec<u8>> {
    rmp_serde::to_vec(msg)
        .map_err(|e| AgentError::ExecutionError(format!("Failed to serialize ZMQ message: {}", e)))
}

/// Deserialize a ZmqMessage from MessagePack bytes.
///
/// # Example
///
/// ```rust
/// use descartes_core::{deserialize_zmq_message, serialize_zmq_message, ZmqMessage, HealthCheckRequest};
///
/// let msg = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
///     request_id: "test-123".to_string(),
/// });
///
/// let bytes = serialize_zmq_message(&msg)?;
/// let deserialized = deserialize_zmq_message(&bytes)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn deserialize_zmq_message(bytes: &[u8]) -> AgentResult<ZmqMessage> {
    rmp_serde::from_slice(bytes).map_err(|e| {
        AgentError::ExecutionError(format!("Failed to deserialize ZMQ message: {}", e))
    })
}

/// Validate message size to prevent DOS attacks.
pub fn validate_message_size(size: usize) -> AgentResult<()> {
    if size > MAX_MESSAGE_SIZE {
        return Err(AgentError::ExecutionError(format!(
            "Message size {} exceeds maximum allowed size {}",
            size, MAX_MESSAGE_SIZE
        )));
    }
    Ok(())
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for ZMQ agent runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZmqRunnerConfig {
    /// ZMQ endpoint to connect to
    pub endpoint: String,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout_secs: u64,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,

    /// Enable automatic reconnection on failure
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,

    /// Maximum number of reconnection attempts
    #[serde(default = "default_max_retries")]
    pub max_reconnect_attempts: u32,

    /// Reconnect delay in seconds
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,

    /// Enable heartbeat/keepalive
    #[serde(default = "default_true")]
    pub enable_heartbeat: bool,

    /// Heartbeat interval in seconds
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,

    /// Server identifier (for multi-server setups)
    #[serde(default)]
    pub server_id: Option<String>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_true() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

fn default_reconnect_delay() -> u64 {
    5
}

fn default_heartbeat_interval() -> u64 {
    30
}

impl Default for ZmqRunnerConfig {
    fn default() -> Self {
        Self {
            endpoint: "tcp://localhost:5555".to_string(),
            connection_timeout_secs: DEFAULT_TIMEOUT_SECS,
            request_timeout_secs: DEFAULT_TIMEOUT_SECS,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 5,
            enable_heartbeat: true,
            heartbeat_interval_secs: 30,
            server_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_request_serialization() {
        let request = SpawnRequest {
            request_id: "test-123".to_string(),
            config: AgentConfig {
                name: "test-agent".to_string(),
                model_backend: "claude".to_string(),
                task: "Test task".to_string(),
                context: None,
                system_prompt: None,
                environment: HashMap::new(),
                ..Default::default()
            },
            timeout_secs: Some(30),
            metadata: None,
        };

        let msg = ZmqMessage::SpawnRequest(request);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();

        match deserialized {
            ZmqMessage::SpawnRequest(req) => {
                assert_eq!(req.request_id, "test-123");
                assert_eq!(req.config.name, "test-agent");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_control_command_serialization() {
        let command = ControlCommand {
            request_id: "test-456".to_string(),
            agent_id: Uuid::new_v4(),
            command_type: ControlCommandType::Pause,
            payload: None,
        };

        let msg = ZmqMessage::ControlCommand(command);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();

        match deserialized {
            ZmqMessage::ControlCommand(cmd) => {
                assert_eq!(cmd.request_id, "test-456");
                assert_eq!(cmd.command_type, ControlCommandType::Pause);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_status_update_serialization() {
        let update = StatusUpdate {
            agent_id: Uuid::new_v4(),
            update_type: StatusUpdateType::StatusChanged,
            status: Some(AgentStatus::Running),
            message: Some("Agent started".to_string()),
            data: None,
            timestamp: SystemTime::now(),
        };

        let msg = ZmqMessage::StatusUpdate(update);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();

        match deserialized {
            ZmqMessage::StatusUpdate(upd) => {
                assert_eq!(upd.update_type, StatusUpdateType::StatusChanged);
                assert_eq!(upd.status, Some(AgentStatus::Running));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_size_validation() {
        assert!(validate_message_size(1024).is_ok());
        assert!(validate_message_size(MAX_MESSAGE_SIZE).is_ok());
        assert!(validate_message_size(MAX_MESSAGE_SIZE + 1).is_err());
    }

    #[test]
    fn test_zmq_runner_config_default() {
        let config = ZmqRunnerConfig::default();
        assert_eq!(config.endpoint, "tcp://localhost:5555");
        assert_eq!(config.connection_timeout_secs, DEFAULT_TIMEOUT_SECS);
        assert!(config.auto_reconnect);
        assert!(config.enable_heartbeat);
    }

    #[test]
    fn test_health_check_serialization() {
        let request = HealthCheckRequest {
            request_id: "health-1".to_string(),
        };

        let msg = ZmqMessage::HealthCheckRequest(request);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();

        match deserialized {
            ZmqMessage::HealthCheckRequest(req) => {
                assert_eq!(req.request_id, "health-1");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_log_stream_message_serialization() {
        let agent_id = Uuid::new_v4();
        let message = LogStreamMessage {
            agent_id,
            stream_type: LogStreamType::Stdout,
            data: b"Hello from agent\n".to_vec(),
            timestamp: SystemTime::now(),
            sequence: 42,
        };

        let msg = ZmqMessage::LogStream(message);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();

        match deserialized {
            ZmqMessage::LogStream(log_msg) => {
                assert_eq!(log_msg.agent_id, agent_id);
                assert_eq!(log_msg.stream_type, LogStreamType::Stdout);
                assert_eq!(log_msg.data, b"Hello from agent\n".to_vec());
                assert_eq!(log_msg.sequence, 42);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_log_stream_type_serialization() {
        // Test stdout
        let stdout_msg = LogStreamMessage {
            agent_id: Uuid::new_v4(),
            stream_type: LogStreamType::Stdout,
            data: vec![],
            timestamp: SystemTime::now(),
            sequence: 0,
        };
        let msg = ZmqMessage::LogStream(stdout_msg);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();
        match deserialized {
            ZmqMessage::LogStream(log_msg) => {
                assert_eq!(log_msg.stream_type, LogStreamType::Stdout);
            }
            _ => panic!("Wrong message type"),
        }

        // Test stderr
        let stderr_msg = LogStreamMessage {
            agent_id: Uuid::new_v4(),
            stream_type: LogStreamType::Stderr,
            data: vec![],
            timestamp: SystemTime::now(),
            sequence: 0,
        };
        let msg = ZmqMessage::LogStream(stderr_msg);
        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();
        match deserialized {
            ZmqMessage::LogStream(log_msg) => {
                assert_eq!(log_msg.stream_type, LogStreamType::Stderr);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
