/// ZMQ server implementation for remote agent spawning and lifecycle management.
///
/// This module provides the server-side implementation for distributed agent orchestration,
/// handling incoming requests from ZMQ clients to spawn, control, and monitor agents.
///
/// # Architecture
///
/// ```text
/// ZmqAgentServer
///   ├── Request Handling
///   │   ├── SpawnRequest -> spawn_agent()
///   │   ├── ControlCommand -> control_agent()
///   │   ├── ListAgentsRequest -> list_agents()
///   │   └── HealthCheckRequest -> health_check()
///   ├── Agent Management
///   │   ├── Agent Registry (DashMap)
///   │   ├── LocalProcessRunner integration
///   │   └── Lifecycle tracking
///   ├── Status Updates
///   │   ├── Periodic status broadcasts
///   │   ├── Event-driven updates
///   │   └── PUB socket for subscribers
///   └── Server Control
///       ├── Start/stop server
///       ├── Graceful shutdown
///       └── Server statistics
/// ```
use crate::agent_runner::{LocalProcessRunner, ProcessRunnerConfig};
use crate::errors::{AgentError, AgentResult};
use crate::traits::{AgentConfig, AgentHandle, AgentInfo, AgentRunner, AgentSignal, AgentStatus};
use crate::zmq_agent_runner::{
    CommandResponse, ControlCommand, ControlCommandType, HealthCheckRequest, HealthCheckResponse,
    ListAgentsRequest, ListAgentsResponse, LogStreamMessage, LogStreamType, SpawnRequest,
    SpawnResponse, ZmqMessage, ZMQ_PROTOCOL_VERSION,
};
use crate::zmq_communication::{SocketType, ZmqConnection};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Default status update interval (10 seconds)
const DEFAULT_STATUS_UPDATE_INTERVAL_SECS: u64 = 10;

/// Maximum number of agents per server
const DEFAULT_MAX_AGENTS: usize = 100;

/// Agent entry in the server registry
#[derive(Debug)]
struct ManagedAgent {
    /// Agent information
    info: AgentInfo,
    /// Agent configuration used to spawn
    _config: AgentConfig,
    /// Spawn timestamp
    _spawned_at: SystemTime,
    /// Request ID that spawned this agent
    _spawn_request_id: String,
    /// Output buffer for stdout
    _stdout_buffer: Arc<Mutex<Vec<u8>>>,
    /// Output buffer for stderr
    _stderr_buffer: Arc<Mutex<Vec<u8>>>,
    /// Last status update sent
    last_status_update: Arc<RwLock<Option<SystemTime>>>,
}

/// ZMQ server configuration
#[derive(Debug, Clone)]
pub struct ZmqServerConfig {
    /// Server endpoint to bind to (REP socket for commands)
    pub endpoint: String,
    /// PUB socket endpoint for log streaming (None = disabled)
    pub pub_endpoint: Option<String>,
    /// Server identifier (for multi-server setups)
    pub server_id: String,
    /// Maximum concurrent agents
    pub max_agents: usize,
    /// Status update interval in seconds
    pub status_update_interval_secs: u64,
    /// Enable automatic status updates
    pub enable_status_updates: bool,
    /// Process runner configuration
    pub runner_config: ProcessRunnerConfig,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}

impl Default for ZmqServerConfig {
    fn default() -> Self {
        Self {
            endpoint: "tcp://0.0.0.0:5555".to_string(),
            pub_endpoint: Some("tcp://0.0.0.0:5556".to_string()),
            server_id: format!("server-{}", Uuid::new_v4()),
            max_agents: DEFAULT_MAX_AGENTS,
            status_update_interval_secs: DEFAULT_STATUS_UPDATE_INTERVAL_SECS,
            enable_status_updates: true,
            runner_config: ProcessRunnerConfig::default(),
            request_timeout_secs: 30,
        }
    }
}

/// Server statistics
#[derive(Debug, Clone, Default)]
pub struct ServerStats {
    /// Total spawn requests received
    pub spawn_requests: u64,
    /// Successful spawns
    pub successful_spawns: u64,
    /// Failed spawns
    pub failed_spawns: u64,
    /// Total control commands received
    pub control_commands: u64,
    /// Total list requests received
    pub list_requests: u64,
    /// Total health checks received
    pub health_checks: u64,
    /// Server start time
    pub started_at: Option<SystemTime>,
    /// Total errors encountered
    pub errors: u64,
}

/// ZMQ Agent Server - Handles incoming requests and manages agent lifecycle
#[derive(Clone)]
pub struct ZmqAgentServer {
    /// Server configuration
    config: ZmqServerConfig,
    /// ZMQ connection for request/response (REP socket)
    connection: Arc<Mutex<ZmqConnection>>,
    /// ZMQ connection for log streaming (PUB socket, optional)
    pub_connection: Option<Arc<Mutex<ZmqConnection>>>,
    /// Local agent runner for spawning processes
    runner: Arc<LocalProcessRunner>,
    /// Registry of managed agents
    agents: Arc<DashMap<Uuid, Arc<ManagedAgent>>>,
    /// Server statistics
    stats: Arc<RwLock<ServerStats>>,
    /// Shutdown signal
    shutdown_tx: Arc<Mutex<Option<tokio::sync::broadcast::Sender<()>>>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

impl ZmqAgentServer {
    /// Create a new ZMQ agent server
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use descartes_core::zmq_server::{ZmqAgentServer, ZmqServerConfig};
    ///
    /// let config = ZmqServerConfig {
    ///     endpoint: "tcp://0.0.0.0:5555".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// let server = ZmqAgentServer::new(config);
    /// ```
    pub fn new(config: ZmqServerConfig) -> Self {
        let endpoint_clone = config.endpoint.clone();
        let zmq_config = crate::zmq_agent_runner::ZmqRunnerConfig {
            endpoint: endpoint_clone,
            connection_timeout_secs: config.request_timeout_secs,
            request_timeout_secs: config.request_timeout_secs,
            ..Default::default()
        };

        let connection = ZmqConnection::new(SocketType::Rep, &config.endpoint, zmq_config.clone());

        // Create PUB socket for log streaming if endpoint configured
        let pub_connection = config.pub_endpoint.as_ref().map(|pub_endpoint| {
            let pub_config = crate::zmq_agent_runner::ZmqRunnerConfig {
                endpoint: pub_endpoint.clone(),
                ..zmq_config
            };
            Arc::new(Mutex::new(ZmqConnection::new(
                SocketType::Pub,
                pub_endpoint,
                pub_config,
            )))
        });

        let runner = LocalProcessRunner::with_config(config.runner_config.clone());

        Self {
            config,
            connection: Arc::new(Mutex::new(connection)),
            pub_connection,
            runner: Arc::new(runner),
            agents: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(ServerStats::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the server and begin processing requests
    ///
    /// This method runs the server event loop, listening for incoming requests
    /// and dispatching them to the appropriate handlers.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::zmq_server::{ZmqAgentServer, ZmqServerConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = ZmqAgentServer::new(ZmqServerConfig::default());
    /// server.start().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&self) -> AgentResult<()> {
        // Check if already running
        if *self.is_running.read() {
            return Err(AgentError::ExecutionError(
                "Server is already running".to_string(),
            ));
        }

        // Connect the REP socket for commands
        self.connection.lock().await.connect().await?;

        // Connect the PUB socket for log streaming if configured
        if let Some(pub_conn) = &self.pub_connection {
            pub_conn.lock().await.connect().await?;
            tracing::info!(
                "PUB socket bound for log streaming: {}",
                self.config.pub_endpoint.as_deref().unwrap_or("unknown")
            );
        }

        // Mark as running
        *self.is_running.write() = true;
        self.stats.write().started_at = Some(SystemTime::now());

        // Create shutdown channel
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        *self.shutdown_tx.lock().await = Some(shutdown_tx.clone());

        tracing::info!(
            "ZMQ Agent Server started: endpoint={}, server_id={}",
            self.config.endpoint,
            self.config.server_id
        );

        // Spawn status update task if enabled
        if self.config.enable_status_updates {
            self.spawn_status_update_task(shutdown_tx.subscribe());
        }

        // Spawn agent monitoring task
        self.spawn_agent_monitoring_task(shutdown_tx.subscribe());

        // Main event loop
        self.run_event_loop(shutdown_tx.subscribe()).await?;

        Ok(())
    }

    /// Stop the server gracefully
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use descartes_core::zmq_server::{ZmqAgentServer, ZmqServerConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let server = ZmqAgentServer::new(ZmqServerConfig::default());
    /// server.stop().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stop(&self) -> AgentResult<()> {
        if !*self.is_running.read() {
            return Ok(());
        }

        tracing::info!("Stopping ZMQ Agent Server...");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
            let _ = tx.send(());
        }

        // Mark as not running
        *self.is_running.write() = false;

        // Stop all agents
        self.stop_all_agents().await?;

        // Disconnect REP socket
        self.connection.lock().await.disconnect().await?;

        // Disconnect PUB socket if configured
        if let Some(pub_conn) = &self.pub_connection {
            pub_conn.lock().await.disconnect().await?;
        }

        tracing::info!("ZMQ Agent Server stopped");

        Ok(())
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }

    /// Get server statistics
    pub fn stats(&self) -> ServerStats {
        self.stats.read().clone()
    }

    /// Get the number of active agents
    pub fn active_agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Get server uptime in seconds
    pub fn uptime_secs(&self) -> Option<u64> {
        self.stats.read().started_at.map(|start| {
            SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs()
        })
    }

    /// Get the PUB endpoint for log streaming (if configured)
    pub fn pub_endpoint(&self) -> Option<&str> {
        self.config.pub_endpoint.as_deref()
    }

    /// Check if log streaming is enabled
    pub fn is_log_streaming_enabled(&self) -> bool {
        self.pub_connection.is_some()
    }

    /// Publish a log message to subscribers via PUB socket.
    ///
    /// The message is published with the agent_id as the topic for filtering.
    pub async fn publish_log(&self, message: &crate::zmq_agent_runner::LogStreamMessage) -> AgentResult<()> {
        if let Some(pub_conn) = &self.pub_connection {
            let topic = message.agent_id.to_string();
            let data = crate::zmq_agent_runner::serialize_zmq_message(
                &ZmqMessage::LogStream(message.clone()),
            )?;
            pub_conn.lock().await.send_with_topic(&topic, &data).await?;
            tracing::trace!(
                "Published log for agent {}: {} bytes",
                message.agent_id,
                message.data.len()
            );
        }
        Ok(())
    }

    /// Run the main event loop
    async fn run_event_loop(
        &self,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> AgentResult<()> {
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    tracing::info!("Received shutdown signal");
                    break;
                }
                // Receive and process request
                _ = async {
                    let connection = self.connection.lock().await;
                    let result = connection.receive_message(Some(Duration::from_secs(1))).await;
                    drop(connection); // Explicitly drop the lock

                    match result {
                        Ok(message) => {
                            // Process the request and send response
                            if let Err(e) = self.handle_request(message).await {
                                tracing::error!("Error handling request: {}", e);
                                self.stats.write().errors += 1;
                            }
                        }
                        Err(AgentError::ExecutionError(msg)) if msg.contains("Timeout") => {
                            // Timeout is expected when no messages are available
                        }
                        Err(e) => {
                            tracing::error!("Error receiving message: {}", e);
                            self.stats.write().errors += 1;
                        }
                    }
                } => {}
            }
        }

        Ok(())
    }

    /// Handle an incoming request message
    async fn handle_request(&self, message: ZmqMessage) -> AgentResult<()> {
        let response = match message {
            ZmqMessage::SpawnRequest(req) => {
                tracing::info!("Received spawn request: {}", req.request_id);
                self.stats.write().spawn_requests += 1;
                ZmqMessage::SpawnResponse(self.handle_spawn_request(req).await)
            }
            ZmqMessage::ControlCommand(cmd) => {
                tracing::debug!(
                    "Received control command: {:?} for agent {}",
                    cmd.command_type,
                    cmd.agent_id
                );
                self.stats.write().control_commands += 1;
                ZmqMessage::CommandResponse(self.handle_control_command(cmd).await)
            }
            ZmqMessage::ListAgentsRequest(req) => {
                tracing::debug!("Received list agents request: {}", req.request_id);
                self.stats.write().list_requests += 1;
                ZmqMessage::ListAgentsResponse(self.handle_list_agents_request(req).await)
            }
            ZmqMessage::HealthCheckRequest(req) => {
                tracing::debug!("Received health check request: {}", req.request_id);
                self.stats.write().health_checks += 1;
                ZmqMessage::HealthCheckResponse(self.handle_health_check_request(req).await)
            }
            _ => {
                return Err(AgentError::ExecutionError(
                    "Unexpected message type".to_string(),
                ));
            }
        };

        // Send response
        self.connection.lock().await.send_message(&response).await?;

        Ok(())
    }

    /// Handle a spawn request
    async fn handle_spawn_request(&self, request: SpawnRequest) -> SpawnResponse {
        // Check if we can spawn more agents
        if self.agents.len() >= self.config.max_agents {
            return SpawnResponse {
                request_id: request.request_id,
                success: false,
                agent_info: None,
                error: Some(format!(
                    "Maximum agent limit reached: {}",
                    self.config.max_agents
                )),
                server_id: Some(self.config.server_id.clone()),
            };
        }

        // Spawn the agent
        match self.runner.spawn(request.config.clone()).await {
            Ok(handle) => {
                let agent_id = handle.id();

                // Get agent info
                match self.runner.get_agent(&agent_id).await {
                    Ok(Some(info)) => {
                        // Create managed agent entry
                        let managed = Arc::new(ManagedAgent {
                            info: info.clone(),
                            _config: request.config,
                            _spawned_at: SystemTime::now(),
                            _spawn_request_id: request.request_id.clone(),
                            _stdout_buffer: Arc::new(Mutex::new(Vec::new())),
                            _stderr_buffer: Arc::new(Mutex::new(Vec::new())),
                            last_status_update: Arc::new(RwLock::new(None)),
                        });

                        // Add to registry
                        self.agents.insert(agent_id, managed);

                        // Spawn output forwarders if log streaming is enabled
                        self.spawn_output_forwarders(agent_id, handle.as_ref());

                        self.stats.write().successful_spawns += 1;

                        tracing::info!(
                            "Agent spawned successfully: id={}, name={}",
                            agent_id,
                            info.name
                        );

                        SpawnResponse {
                            request_id: request.request_id,
                            success: true,
                            agent_info: Some(info),
                            error: None,
                            server_id: Some(self.config.server_id.clone()),
                        }
                    }
                    Ok(None) => {
                        self.stats.write().failed_spawns += 1;
                        SpawnResponse {
                            request_id: request.request_id,
                            success: false,
                            agent_info: None,
                            error: Some("Agent spawned but info not available".to_string()),
                            server_id: Some(self.config.server_id.clone()),
                        }
                    }
                    Err(e) => {
                        self.stats.write().failed_spawns += 1;
                        SpawnResponse {
                            request_id: request.request_id,
                            success: false,
                            agent_info: None,
                            error: Some(format!("Failed to get agent info: {}", e)),
                            server_id: Some(self.config.server_id.clone()),
                        }
                    }
                }
            }
            Err(e) => {
                self.stats.write().failed_spawns += 1;
                tracing::error!("Failed to spawn agent: {}", e);

                SpawnResponse {
                    request_id: request.request_id,
                    success: false,
                    agent_info: None,
                    error: Some(format!("Failed to spawn agent: {}", e)),
                    server_id: Some(self.config.server_id.clone()),
                }
            }
        }
    }

    /// Handle a control command
    async fn handle_control_command(&self, command: ControlCommand) -> CommandResponse {
        let agent_id = command.agent_id;

        // Check if agent exists
        if !self.agents.contains_key(&agent_id) {
            return CommandResponse {
                request_id: command.request_id,
                agent_id,
                success: false,
                status: None,
                data: None,
                error: Some(format!("Agent not found: {}", agent_id)),
            };
        }

        // Execute command based on type
        let result: Result<Option<serde_json::Value>, AgentError> = match command.command_type {
            ControlCommandType::Pause => {
                // Check if force pause is requested (default to cooperative)
                let force = command
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("force"))
                    .and_then(|f| f.as_bool())
                    .unwrap_or(false);

                self.runner.pause(&agent_id, force).await.map(|_| {
                    Some(serde_json::json!({
                        "paused": true,
                        "mode": if force { "forced" } else { "cooperative" }
                    }))
                })
            }
            ControlCommandType::Resume => {
                self.runner.resume(&agent_id).await.map(|_| {
                    Some(serde_json::json!({
                        "resumed": true
                    }))
                })
            }
            ControlCommandType::Stop => {
                // Graceful stop (SIGTERM)
                self.runner
                    .signal(&agent_id, AgentSignal::Terminate)
                    .await
                    .map(|_| None)
            }
            ControlCommandType::Kill => {
                // Force kill
                self.runner.kill(&agent_id).await.map(|_| None)
            }
            ControlCommandType::GetStatus => {
                // Get agent info which includes status
                match self.runner.get_agent(&agent_id).await {
                    Ok(Some(info)) => {
                        let status_json = serde_json::json!({
                            "status": format!("{:?}", info.status)
                        });
                        Ok(Some(status_json))
                    }
                    Ok(None) => Err(AgentError::NotFound(format!(
                        "Agent not found: {}",
                        agent_id
                    ))),
                    Err(e) => Err(e),
                }
            }
            ControlCommandType::WriteStdin => {
                // Get data to write from payload
                let data = command
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("data"))
                    .and_then(|d| d.as_str())
                    .map(|s| s.as_bytes().to_vec())
                    .or_else(|| {
                        command
                            .payload
                            .as_ref()
                            .and_then(|p| p.get("data"))
                            .and_then(|d| d.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                                    .collect()
                            })
                    });

                match data {
                    Some(bytes) => {
                        self.runner.write_stdin(&agent_id, &bytes).await.map(|_| {
                            Some(serde_json::json!({
                                "bytes_written": bytes.len()
                            }))
                        })
                    }
                    None => Err(AgentError::ExecutionError(
                        "WriteStdin requires payload with 'data' field (string or byte array)"
                            .to_string(),
                    )),
                }
            }
            ControlCommandType::ReadStdout => {
                // Read from stdout buffer
                match self.runner.read_stdout(&agent_id).await {
                    Ok(Some(data)) => {
                        let content = String::from_utf8_lossy(&data).to_string();
                        Ok(Some(serde_json::json!({
                            "data": content,
                            "bytes": data.len()
                        })))
                    }
                    Ok(None) => Ok(Some(serde_json::json!({
                        "data": null,
                        "bytes": 0,
                        "message": "No data available in stdout buffer"
                    }))),
                    Err(e) => Err(e),
                }
            }
            ControlCommandType::ReadStderr => {
                // Read from stderr buffer
                match self.runner.read_stderr(&agent_id).await {
                    Ok(Some(data)) => {
                        let content = String::from_utf8_lossy(&data).to_string();
                        Ok(Some(serde_json::json!({
                            "data": content,
                            "bytes": data.len()
                        })))
                    }
                    Ok(None) => Ok(Some(serde_json::json!({
                        "data": null,
                        "bytes": 0,
                        "message": "No data available in stderr buffer"
                    }))),
                    Err(e) => Err(e),
                }
            }
            ControlCommandType::Signal => {
                // Signal handling - parse signal type from payload
                let signal_str = command
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("signal"))
                    .and_then(|s| s.as_str());

                match signal_str {
                    Some(signal) => {
                        let agent_signal = match signal.to_lowercase().as_str() {
                            "interrupt" | "sigint" | "int" => Some(AgentSignal::Interrupt),
                            "terminate" | "sigterm" | "term" => Some(AgentSignal::Terminate),
                            "kill" | "sigkill" => Some(AgentSignal::Kill),
                            "stop" | "sigstop" | "pause" | "force_pause" => {
                                Some(AgentSignal::ForcePause)
                            }
                            "continue" | "sigcont" | "cont" | "resume" => Some(AgentSignal::Resume),
                            _ => None,
                        };

                        match agent_signal {
                            Some(sig) => self.runner.signal(&agent_id, sig).await.map(|_| {
                                Some(serde_json::json!({
                                    "signal_sent": signal
                                }))
                            }),
                            None => Err(AgentError::ExecutionError(format!(
                                "Unknown signal type: '{}'. Valid signals: interrupt, terminate, kill, stop, continue",
                                signal
                            ))),
                        }
                    }
                    None => Err(AgentError::ExecutionError(
                        "Signal command requires payload with 'signal' field".to_string(),
                    )),
                }
            }
            ControlCommandType::CustomAction => {
                // Custom action handling - extract action name and params from payload
                match &command.payload {
                    Some(payload) => {
                        let action = payload
                            .get("action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        let params = payload.get("params").cloned();

                        // Log the custom action for debugging
                        tracing::info!(
                            "Received custom action '{}' for agent {} with params: {:?}",
                            action,
                            agent_id,
                            params
                        );

                        // Handle built-in actions
                        match action {
                            // Echo action - returns the params back (useful for testing)
                            "echo" => Ok(Some(serde_json::json!({
                                "action": "echo",
                                "result": params
                            }))),

                            // Ping action - simple health check
                            "ping" => Ok(Some(serde_json::json!({
                                "action": "ping",
                                "result": "pong",
                                "agent_id": agent_id.to_string()
                            }))),

                            // Get agent environment info
                            "get_env" => {
                                if let Some(agent) = self.agents.get(&agent_id) {
                                    let last_update = agent
                                        .last_status_update
                                        .read()
                                        .map(|t| {
                                            t.elapsed()
                                                .map(|d| d.as_secs())
                                                .unwrap_or(0)
                                        })
                                        .unwrap_or(0);

                                    Ok(Some(serde_json::json!({
                                        "action": "get_env",
                                        "result": {
                                            "status": format!("{:?}", agent.info.status),
                                            "name": agent.info.name,
                                            "agent_id": agent.info.id.to_string(),
                                            "last_status_update_secs": last_update
                                        }
                                    })))
                                } else {
                                    Err(AgentError::NotFound(format!(
                                        "Agent {} not found",
                                        agent_id
                                    )))
                                }
                            }

                            // Unknown action - return error with available actions
                            _ => Err(AgentError::ExecutionError(format!(
                                "Unknown custom action '{}'. Available actions: echo, ping, get_env",
                                action
                            ))),
                        }
                    }
                    None => Err(AgentError::ExecutionError(
                        "CustomAction command requires payload with 'action' field".to_string(),
                    )),
                }
            }
            ControlCommandType::QueryOutput => {
                // Query both stdout and stderr, optionally limited
                let max_lines = command
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("max_lines"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let include_stdout = command
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("stdout"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let include_stderr = command
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("stderr"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                // Collect available stdout lines - use async blocks to handle errors
                let stdout_result: Result<Vec<String>, AgentError> = async {
                    let mut lines = Vec::new();
                    if include_stdout {
                        while lines.len() < max_lines {
                            match self.runner.read_stdout(&agent_id).await {
                                Ok(Some(data)) => {
                                    let line = String::from_utf8_lossy(&data).to_string();
                                    lines.push(line);
                                }
                                Ok(None) => break,
                                Err(e) => return Err(e),
                            }
                        }
                    }
                    Ok(lines)
                }.await;

                let stderr_result: Result<Vec<String>, AgentError> = async {
                    let mut lines = Vec::new();
                    if include_stderr {
                        while lines.len() < max_lines {
                            match self.runner.read_stderr(&agent_id).await {
                                Ok(Some(data)) => {
                                    let line = String::from_utf8_lossy(&data).to_string();
                                    lines.push(line);
                                }
                                Ok(None) => break,
                                Err(e) => return Err(e),
                            }
                        }
                    }
                    Ok(lines)
                }.await;

                match (stdout_result, stderr_result) {
                    (Ok(stdout_lines), Ok(stderr_lines)) => {
                        Ok(Some(serde_json::json!({
                            "stdout": stdout_lines,
                            "stderr": stderr_lines,
                            "stdout_lines": stdout_lines.len(),
                            "stderr_lines": stderr_lines.len()
                        })))
                    }
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            }
            ControlCommandType::StreamLogs => {
                // Log streaming - return PUB endpoint for client to connect
                match &self.config.pub_endpoint {
                    Some(pub_endpoint) => Ok(Some(serde_json::json!({
                        "pub_endpoint": pub_endpoint,
                        "topic": agent_id.to_string(),
                        "enabled": true,
                        "instructions": "Connect SUB socket to pub_endpoint, subscribe to topic filter"
                    }))),
                    None => Err(AgentError::ExecutionError(
                        "Log streaming not enabled on this server (no pub_endpoint configured)"
                            .to_string(),
                    )),
                }
            }
        };

        // Get current status
        let status = self
            .runner
            .get_agent(&agent_id)
            .await
            .ok()
            .flatten()
            .map(|info| info.status);

        match result {
            Ok(data) => CommandResponse {
                request_id: command.request_id,
                agent_id,
                success: true,
                status,
                data,
                error: None,
            },
            Err(e) => CommandResponse {
                request_id: command.request_id,
                agent_id,
                success: false,
                status,
                data: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Handle a list agents request
    async fn handle_list_agents_request(&self, request: ListAgentsRequest) -> ListAgentsResponse {
        // Use runner's list_agents method
        match self.runner.list_agents().await {
            Ok(mut agents) => {
                // Apply status filter if specified
                if let Some(filter_status) = &request.filter_status {
                    agents.retain(|agent| agent.status == *filter_status);
                }

                // Apply limit if specified
                if let Some(limit) = request.limit {
                    agents.truncate(limit);
                }

                ListAgentsResponse {
                    request_id: request.request_id,
                    success: true,
                    agents,
                    error: None,
                }
            }
            Err(e) => ListAgentsResponse {
                request_id: request.request_id,
                success: false,
                agents: Vec::new(),
                error: Some(format!("Failed to list agents: {}", e)),
            },
        }
    }

    /// Handle a health check request
    async fn handle_health_check_request(
        &self,
        request: HealthCheckRequest,
    ) -> HealthCheckResponse {
        let uptime_secs = self.uptime_secs();
        let active_agents = self.active_agent_count();

        let mut metadata = HashMap::new();
        metadata.insert("server_id".to_string(), self.config.server_id.clone());
        metadata.insert("endpoint".to_string(), self.config.endpoint.clone());
        metadata.insert("max_agents".to_string(), self.config.max_agents.to_string());

        HealthCheckResponse {
            request_id: request.request_id,
            healthy: true,
            protocol_version: ZMQ_PROTOCOL_VERSION.to_string(),
            uptime_secs,
            active_agents: Some(active_agents),
            metadata: Some(metadata),
        }
    }

    /// Stop all agents
    async fn stop_all_agents(&self) -> AgentResult<()> {
        tracing::info!("Stopping all agents...");

        // Get list of all agents
        let agents = match self.runner.list_agents().await {
            Ok(agents) => agents,
            Err(e) => {
                tracing::error!("Failed to list agents: {}", e);
                return Ok(()); // Continue with shutdown anyway
            }
        };

        let agent_ids: Vec<Uuid> = agents.iter().map(|a| a.id).collect();

        // Try graceful stop first (SIGTERM)
        for agent_id in &agent_ids {
            if let Err(e) = self.runner.signal(agent_id, AgentSignal::Terminate).await {
                tracing::warn!("Failed to terminate agent {}: {}", agent_id, e);
            }
        }

        // Wait a bit for graceful shutdown
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Force kill any remaining agents
        for agent_id in &agent_ids {
            if let Err(e) = self.runner.kill(agent_id).await {
                tracing::warn!("Failed to kill agent {}: {}", agent_id, e);
            }
        }

        // Clear the registry
        self.agents.clear();

        Ok(())
    }

    /// Spawn background task for periodic status updates
    fn spawn_status_update_task(&self, mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) {
        let agents = self.agents.clone();
        let interval_secs = self.config.status_update_interval_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Status update task shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        // Send status updates for all agents
                        for entry in agents.iter() {
                            let managed = entry.value();
                            let now = SystemTime::now();

                            // Update last status update time
                            *managed.last_status_update.write() = Some(now);

                            // In a real implementation, we would broadcast this via PUB socket
                            tracing::trace!(
                                "Status update for agent {}: {:?}",
                                managed.info.id,
                                managed.info.status
                            );
                        }
                    }
                }
            }
        });
    }

    /// Spawn background task for monitoring agent lifecycle
    fn spawn_agent_monitoring_task(&self, mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) {
        let agents = self.agents.clone();
        let runner = self.runner.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Agent monitoring task shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        // Check for completed/terminated agents and remove from registry
                        let agent_ids: Vec<Uuid> = agents.iter().map(|e| *e.key()).collect();

                        for agent_id in agent_ids {
                            if let Ok(Some(info)) = runner.get_agent(&agent_id).await {
                                match info.status {
                                    AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Terminated => {
                                        // Agent is done, remove from registry
                                        if agents.remove(&agent_id).is_some() {
                                            tracing::info!(
                                                "Agent {} completed with status {:?}, removing from registry",
                                                agent_id,
                                                info.status
                                            );
                                        }
                                    }
                                    _ => {
                                        // Agent still active
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Spawn output forwarders for an agent to stream stdout/stderr to PUB socket.
    ///
    /// This subscribes to the agent's broadcast channels and publishes received
    /// data as LogStreamMessages via the PUB socket.
    fn spawn_output_forwarders(&self, agent_id: Uuid, handle: &dyn AgentHandle) {
        // Only spawn forwarders if PUB socket is configured
        let pub_connection = match &self.pub_connection {
            Some(conn) => conn.clone(),
            None => return,
        };

        // Subscribe to stdout broadcast
        let stdout_rx = handle.subscribe_stdout();
        self.spawn_single_output_forwarder(
            agent_id,
            stdout_rx,
            LogStreamType::Stdout,
            pub_connection.clone(),
        );

        // Subscribe to stderr broadcast
        let stderr_rx = handle.subscribe_stderr();
        self.spawn_single_output_forwarder(agent_id, stderr_rx, LogStreamType::Stderr, pub_connection);

        tracing::debug!(
            "Output forwarders spawned for agent {} (stdout + stderr)",
            agent_id
        );
    }

    /// Spawn a single output forwarder task for one stream type.
    fn spawn_single_output_forwarder(
        &self,
        agent_id: Uuid,
        mut rx: broadcast::Receiver<Vec<u8>>,
        stream_type: LogStreamType,
        pub_connection: Arc<Mutex<ZmqConnection>>,
    ) {
        // Use atomic counter for sequence numbers
        let sequence = Arc::new(AtomicU64::new(0));

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        let seq = sequence.fetch_add(1, Ordering::SeqCst);
                        let message = LogStreamMessage {
                            agent_id,
                            stream_type,
                            data: data.clone(),
                            timestamp: SystemTime::now(),
                            sequence: seq,
                        };

                        // Publish to PUB socket
                        let topic = agent_id.to_string();
                        match crate::zmq_agent_runner::serialize_zmq_message(&ZmqMessage::LogStream(
                            message,
                        )) {
                            Ok(bytes) => {
                                if let Err(e) =
                                    pub_connection.lock().await.send_with_topic(&topic, &bytes).await
                                {
                                    tracing::warn!(
                                        "Failed to publish {:?} for agent {}: {}",
                                        stream_type,
                                        agent_id,
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to serialize log message for agent {}: {}",
                                    agent_id,
                                    e
                                );
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::debug!(
                            "Output channel closed for agent {} ({:?})",
                            agent_id,
                            stream_type
                        );
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            "Output forwarder lagged {} messages for agent {} ({:?})",
                            n,
                            agent_id,
                            stream_type
                        );
                        // Continue receiving
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ZmqServerConfig::default();
        assert_eq!(config.endpoint, "tcp://0.0.0.0:5555");
        assert_eq!(config.max_agents, DEFAULT_MAX_AGENTS);
        assert!(config.enable_status_updates);
    }

    #[test]
    fn test_server_creation() {
        let config = ZmqServerConfig::default();
        let server = ZmqAgentServer::new(config);
        assert!(!server.is_running());
        assert_eq!(server.active_agent_count(), 0);
    }

    #[test]
    fn test_server_stats() {
        let config = ZmqServerConfig::default();
        let server = ZmqAgentServer::new(config);
        let stats = server.stats();
        assert_eq!(stats.spawn_requests, 0);
        assert_eq!(stats.successful_spawns, 0);
    }

    #[test]
    fn test_server_config_pub_endpoint() {
        let config = ZmqServerConfig::default();
        // Default config should have PUB endpoint enabled
        assert!(config.pub_endpoint.is_some());
        assert_eq!(
            config.pub_endpoint.as_deref(),
            Some("tcp://0.0.0.0:5556")
        );
    }

    #[test]
    fn test_server_log_streaming_enabled() {
        // With default config (pub_endpoint enabled)
        let config = ZmqServerConfig::default();
        let server = ZmqAgentServer::new(config);
        assert!(server.is_log_streaming_enabled());
        assert_eq!(server.pub_endpoint(), Some("tcp://0.0.0.0:5556"));
    }

    #[test]
    fn test_server_log_streaming_disabled() {
        // With pub_endpoint disabled
        let config = ZmqServerConfig {
            pub_endpoint: None,
            ..Default::default()
        };
        let server = ZmqAgentServer::new(config);
        assert!(!server.is_log_streaming_enabled());
        assert_eq!(server.pub_endpoint(), None);
    }
}
