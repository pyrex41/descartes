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
use crate::traits::{AgentConfig, AgentInfo, AgentRunner, AgentSignal, AgentStatus};
use crate::zmq_agent_runner::{
    CommandResponse, ControlCommand, ControlCommandType, HealthCheckRequest, HealthCheckResponse,
    ListAgentsRequest, ListAgentsResponse, SpawnRequest, SpawnResponse, ZmqMessage,
    ZMQ_PROTOCOL_VERSION,
};
use crate::zmq_communication::{SocketType, ZmqConnection};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
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
    config: AgentConfig,
    /// Spawn timestamp
    spawned_at: SystemTime,
    /// Request ID that spawned this agent
    spawn_request_id: String,
    /// Output buffer for stdout
    stdout_buffer: Arc<Mutex<Vec<u8>>>,
    /// Output buffer for stderr
    stderr_buffer: Arc<Mutex<Vec<u8>>>,
    /// Last status update sent
    last_status_update: Arc<RwLock<Option<SystemTime>>>,
}

/// ZMQ server configuration
#[derive(Debug, Clone)]
pub struct ZmqServerConfig {
    /// Server endpoint to bind to
    pub endpoint: String,
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
    /// ZMQ connection for request/response
    connection: Arc<Mutex<ZmqConnection>>,
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

        let connection = ZmqConnection::new(SocketType::Rep, &config.endpoint, zmq_config);

        let runner = LocalProcessRunner::with_config(config.runner_config.clone());

        Self {
            config,
            connection: Arc::new(Mutex::new(connection)),
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

        // Connect the socket
        self.connection.lock().await.connect().await?;

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

        // Disconnect socket
        self.connection.lock().await.disconnect().await?;

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
                            config: request.config,
                            spawned_at: SystemTime::now(),
                            spawn_request_id: request.request_id.clone(),
                            stdout_buffer: Arc::new(Mutex::new(Vec::new())),
                            stderr_buffer: Arc::new(Mutex::new(Vec::new())),
                            last_status_update: Arc::new(RwLock::new(None)),
                        });

                        // Add to registry
                        self.agents.insert(agent_id, managed);

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
                // Pause not directly supported - would need SIGTSTP on Unix
                Err(AgentError::ExecutionError(
                    "Pause command not yet implemented - requires platform-specific signal handling".to_string(),
                ))
            }
            ControlCommandType::Resume => {
                // Resume not directly supported - would need SIGCONT on Unix
                Err(AgentError::ExecutionError(
                    "Resume command not yet implemented - requires platform-specific signal handling".to_string(),
                ))
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
                // stdin/stdout/stderr operations not directly supported by AgentRunner trait
                // For now, return not implemented
                Err(AgentError::ExecutionError(
                    "WriteStdin command not yet implemented - requires extended AgentHandle interface".to_string(),
                ))
            }
            ControlCommandType::ReadStdout => {
                // stdout operations not directly supported
                Err(AgentError::ExecutionError(
                    "ReadStdout command not yet implemented - requires extended AgentHandle interface".to_string(),
                ))
            }
            ControlCommandType::ReadStderr => {
                // stderr operations not directly supported
                Err(AgentError::ExecutionError(
                    "ReadStderr command not yet implemented - requires extended AgentHandle interface".to_string(),
                ))
            }
            ControlCommandType::Signal => {
                // Custom signal handling
                Err(AgentError::ExecutionError(
                    "Signal command not yet implemented".to_string(),
                ))
            }
            ControlCommandType::CustomAction => {
                // Custom action handling
                Err(AgentError::ExecutionError(
                    "CustomAction command not yet implemented".to_string(),
                ))
            }
            ControlCommandType::QueryOutput => {
                // Output querying
                Err(AgentError::ExecutionError(
                    "QueryOutput command not yet implemented".to_string(),
                ))
            }
            ControlCommandType::StreamLogs => {
                // Log streaming
                Err(AgentError::ExecutionError(
                    "StreamLogs command not yet implemented".to_string(),
                ))
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
}
