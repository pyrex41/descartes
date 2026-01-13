//! jsonrpsee-based RPC Server over Unix Socket
//!
//! This module implements a JSON-RPC 2.0 server using the jsonrpsee library,
//! configured to listen on a Unix socket for IPC communication.
//!
//! The server exposes methods for:
//! - spawn: Create and start new agents
//! - list_tasks: List all tasks in the system
//! - approve: Approve pending tasks or actions
//! - get_state: Query the current state

use crate::errors::{DaemonError, DaemonResult};
use crate::events::{AgentEvent, AgentEventType, DescartesEvent, EventBus};
use crate::types::{RpcError, RpcRequest, RpcResponse};
use descartes_core::swank::{find_available_port, SwankClient, SwankLauncher, SwankMessage, DEFAULT_SWANK_PORT};
use descartes_core::tools::SWANK_REGISTRY;
use descartes_core::traits::{AgentConfig, TaskStatus};
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Child;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// RPC API trait defining all available methods
#[rpc(server)]
pub trait DescartesRpc {
    /// Spawn a new agent with the given configuration
    ///
    /// # Arguments
    /// * `name` - The name of the agent
    /// * `agent_type` - The type of agent to spawn
    /// * `config` - Additional configuration parameters
    ///
    /// # Returns
    /// The ID of the spawned agent
    #[method(name = "spawn")]
    async fn spawn(
        &self,
        name: String,
        agent_type: String,
        config: Value,
    ) -> Result<String, ErrorObjectOwned>;

    /// List all tasks in the system
    ///
    /// # Arguments
    /// * `filter` - Optional filter criteria
    ///
    /// # Returns
    /// List of tasks matching the filter
    #[method(name = "list_tasks")]
    async fn list_tasks(&self, filter: Option<Value>) -> Result<Vec<TaskInfo>, ErrorObjectOwned>;

    /// Approve a pending task or action
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to approve
    /// * `approved` - Whether to approve or reject
    ///
    /// # Returns
    /// Confirmation of the approval
    #[method(name = "approve")]
    async fn approve(
        &self,
        task_id: String,
        approved: bool,
    ) -> Result<ApprovalResult, ErrorObjectOwned>;

    /// Get the current state of the system or a specific entity
    ///
    /// # Arguments
    /// * `entity_id` - Optional ID of a specific entity to query
    ///
    /// # Returns
    /// The current state
    #[method(name = "get_state")]
    async fn get_state(&self, entity_id: Option<String>) -> Result<Value, ErrorObjectOwned>;

    /// Pause a running agent
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to pause
    /// * `force` - If true, use SIGSTOP (forced); otherwise use cooperative pause
    ///
    /// # Returns
    /// Pause confirmation with timestamp and mode
    #[method(name = "agent.pause")]
    async fn pause_agent(
        &self,
        agent_id: String,
        force: bool,
    ) -> Result<PauseResult, ErrorObjectOwned>;

    /// Resume a paused agent
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to resume
    ///
    /// # Returns
    /// Resume confirmation with timestamp
    #[method(name = "agent.resume")]
    async fn resume_agent(&self, agent_id: String) -> Result<ResumeResult, ErrorObjectOwned>;

    /// Request attach credentials for a paused agent
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to attach to
    /// * `client_type` - The type of client requesting attachment (e.g., "claude-code")
    ///
    /// # Returns
    /// Attach credentials including token and connect URL
    #[method(name = "agent.attach.request")]
    async fn attach_request(
        &self,
        agent_id: String,
        client_type: String,
    ) -> Result<AttachCredentialsResult, ErrorObjectOwned>;

    /// Validate an attach token
    ///
    /// # Arguments
    /// * `token` - The attach token to validate
    ///
    /// # Returns
    /// Validation result including whether token is valid and associated agent
    #[method(name = "agent.attach.validate")]
    async fn attach_validate(&self, token: String) -> Result<AttachValidateResult, ErrorObjectOwned>;

    /// Revoke an attach token
    ///
    /// # Arguments
    /// * `token` - The attach token to revoke
    ///
    /// # Returns
    /// Whether the token was successfully revoked
    #[method(name = "agent.attach.revoke")]
    async fn attach_revoke(&self, token: String) -> Result<AttachRevokeResult, ErrorObjectOwned>;
}

/// Task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Approval result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResult {
    pub task_id: String,
    pub approved: bool,
    pub timestamp: i64,
}

/// Pause result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseResult {
    pub agent_id: String,
    pub paused_at: i64,
    pub pause_mode: String,
}

/// Resume result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeResult {
    pub agent_id: String,
    pub resumed_at: i64,
}

/// Attach credentials result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachCredentialsResult {
    pub agent_id: String,
    pub token: String,
    pub connect_url: String,
    pub expires_at: i64,
}

/// Attach validate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachValidateResult {
    pub valid: bool,
    pub agent_id: Option<String>,
    pub expires_at: Option<i64>,
}

/// Attach revoke result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachRevokeResult {
    pub revoked: bool,
}

/// Swank restart result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwankRestartResult {
    pub agent_id: String,
    pub restart_index: usize,
    pub success: bool,
    pub message: Option<String>,
}

/// Check if an agent should use Lisp/Swank.
fn is_lisp_agent(config: &AgentConfig) -> bool {
    // Check model_backend
    let backend_lower = config.model_backend.to_lowercase();
    if backend_lower.contains("lisp") || backend_lower.contains("sbcl") {
        return true;
    }

    // Check environment variable
    if let Some(tool_level) = config.environment.get("DESCARTES_TOOL_LEVEL") {
        if tool_level == "lisp_developer" || tool_level == "lisp-developer" {
            return true;
        }
    }

    // Check name convention
    if config.name.to_lowercase().contains("lisp") {
        return true;
    }

    false
}

/// Implementation of the RPC server
pub struct RpcServerImpl {
    /// Agent runner for spawning and managing agents
    agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
    /// Concrete LocalProcessRunner reference for I/O channel access
    /// Only set when using LocalProcessRunner as the backend
    local_runner: Option<Arc<descartes_core::LocalProcessRunner>>,
    /// State store for persisting tasks and events
    state_store: Arc<dyn descartes_core::traits::StateStore>,
    /// Mapping of agent IDs (String -> Uuid) for convenience
    agent_ids: Arc<dashmap::DashMap<String, uuid::Uuid>>,
    /// Attach session manager for managing TUI attachment sessions
    attach_manager: Arc<crate::attach_session::AttachSessionManager>,
    /// Event bus for system events
    event_bus: Arc<crate::events::EventBus>,
    /// Active attach server handles (agent_id -> JoinHandle)
    attach_servers: Arc<dashmap::DashMap<uuid::Uuid, tokio::task::JoinHandle<()>>>,
    /// SBCL processes for Lisp agents (agent_id -> Child)
    sbcl_processes: Arc<dashmap::DashMap<uuid::Uuid, Child>>,
    /// Swank event forwarding tasks (agent_id -> JoinHandle)
    swank_event_tasks: Arc<dashmap::DashMap<uuid::Uuid, tokio::task::JoinHandle<()>>>,
}

impl RpcServerImpl {
    /// Create a new RPC server implementation
    pub fn new(
        agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
        state_store: Arc<dyn descartes_core::traits::StateStore>,
    ) -> Self {
        let attach_config = crate::attach_session::AttachSessionConfig::default();
        let token_store = Arc::new(descartes_core::AttachTokenStore::new());
        let event_bus = Arc::new(crate::events::EventBus::new());
        let attach_manager = Arc::new(crate::attach_session::AttachSessionManager::new(
            Arc::clone(&token_store),
            Arc::clone(&event_bus),
            attach_config,
        ));
        Self {
            agent_runner,
            local_runner: None,
            state_store,
            agent_ids: Arc::new(dashmap::DashMap::new()),
            attach_manager,
            event_bus,
            attach_servers: Arc::new(dashmap::DashMap::new()),
            sbcl_processes: Arc::new(dashmap::DashMap::new()),
            swank_event_tasks: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create a new RPC server implementation with a LocalProcessRunner
    ///
    /// This allows the server to access I/O channels for TUI attachment.
    pub fn with_local_runner(
        local_runner: Arc<descartes_core::LocalProcessRunner>,
        state_store: Arc<dyn descartes_core::traits::StateStore>,
    ) -> Self {
        let attach_config = crate::attach_session::AttachSessionConfig::default();
        let token_store = Arc::new(descartes_core::AttachTokenStore::new());
        let event_bus = Arc::new(crate::events::EventBus::new());
        let attach_manager = Arc::new(crate::attach_session::AttachSessionManager::new(
            Arc::clone(&token_store),
            Arc::clone(&event_bus),
            attach_config,
        ));
        Self {
            agent_runner: Arc::clone(&local_runner) as Arc<dyn descartes_core::traits::AgentRunner>,
            local_runner: Some(local_runner),
            state_store,
            agent_ids: Arc::new(dashmap::DashMap::new()),
            attach_manager,
            event_bus,
            attach_servers: Arc::new(dashmap::DashMap::new()),
            sbcl_processes: Arc::new(dashmap::DashMap::new()),
            swank_event_tasks: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create a new RPC server implementation with custom attach manager
    pub fn with_attach_manager(
        agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
        state_store: Arc<dyn descartes_core::traits::StateStore>,
        attach_manager: Arc<crate::attach_session::AttachSessionManager>,
        event_bus: Arc<crate::events::EventBus>,
    ) -> Self {
        Self {
            agent_runner,
            local_runner: None,
            state_store,
            agent_ids: Arc::new(dashmap::DashMap::new()),
            attach_manager,
            event_bus,
            attach_servers: Arc::new(dashmap::DashMap::new()),
            sbcl_processes: Arc::new(dashmap::DashMap::new()),
            swank_event_tasks: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub(crate) async fn spawn_agent_internal(
        &self,
        name: String,
        agent_type: String,
        config: Value,
    ) -> Result<String, ErrorObjectOwned> {
        info!("Spawning agent: {} (type: {})", name, agent_type);

        let environment: HashMap<String, String> = config
            .get("environment")
            .and_then(|e| serde_json::from_value(e.clone()).ok())
            .unwrap_or_default();

        let task = config
            .get("task")
            .and_then(|t| t.as_str())
            .unwrap_or("No task specified")
            .to_string();

        let context = config
            .get("context")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let system_prompt = config
            .get("system_prompt")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        // Extract optional fields
        let model = config
            .get("model")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let tool_level = config
            .get("tool_level")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let agents = config
            .get("agents")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        let agent_config = AgentConfig {
            name: name.clone(),
            model_backend: agent_type,
            task,
            context,
            system_prompt,
            environment,
            model,
            tool_level,
            agents,
        };

        // Check if this is a Lisp agent before spawning (agent_config is moved by spawn)
        let needs_swank = is_lisp_agent(&agent_config);

        let agent_handle = self.agent_runner.spawn(agent_config).await.map_err(|e| {
            error!("Failed to spawn agent: {}", e);
            ErrorObjectOwned::owned(-32603, format!("Failed to spawn agent: {}", e), None::<()>)
        })?;

        let agent_id = agent_handle.id();
        let agent_id_str = agent_id.to_string();
        self.agent_ids.insert(agent_id_str.clone(), agent_id);

        // Initialize Swank for Lisp agents - fail spawn if Swank init fails
        if needs_swank {
            match self.initialize_swank_session(agent_id).await {
                Ok(()) => {
                    info!("Swank session initialized for agent {}", agent_id_str);
                }
                Err(e) => {
                    // Swank is required for Lisp agents - kill the agent and fail the spawn
                    error!("Failed to initialize Swank for Lisp agent {}: {}", agent_id_str, e);

                    // Clean up the partially spawned agent
                    if let Err(kill_err) = self.agent_runner.kill(&agent_id).await {
                        warn!("Failed to kill agent after Swank init failure: {}", kill_err);
                    }
                    self.agent_ids.remove(&agent_id_str);

                    return Err(ErrorObjectOwned::owned(
                        -32017,
                        format!("Failed to initialize Lisp runtime (SBCL/Swank): {}", e),
                        None::<()>,
                    ));
                }
            }
        }

        info!("Agent spawned successfully with ID: {}", agent_id_str);
        Ok(agent_id_str)
    }

    /// Initialize a Swank session for a Lisp agent.
    async fn initialize_swank_session(&self, agent_id: Uuid) -> Result<(), String> {
        // Find available port
        let port = find_available_port(DEFAULT_SWANK_PORT)
            .await
            .map_err(|e| format!("Failed to find available port: {}", e))?;

        info!("Starting SBCL with Swank on port {} for agent {}", port, agent_id);

        // Start SBCL
        let sbcl_child = SwankLauncher::start_sbcl(port)
            .await
            .map_err(|e| format!("Failed to start SBCL: {}", e))?;

        // Create event channel for this session
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(64);

        // Connect SwankClient
        let swank_client = SwankClient::connect(agent_id, port, event_tx)
            .await
            .map_err(|e| format!("Failed to connect to Swank: {}", e))?;

        // Register in global registry
        SWANK_REGISTRY.insert(agent_id, swank_client);

        // Store SBCL process for cleanup
        self.sbcl_processes.insert(agent_id, sbcl_child);

        // Spawn event forwarding task
        let event_bus = Arc::clone(&self.event_bus);
        let agent_id_clone = agent_id;
        let event_task = tokio::spawn(async move {
            while let Some(msg) = event_rx.recv().await {
                Self::forward_swank_event(&event_bus, agent_id_clone, msg).await;
            }
        });
        self.swank_event_tasks.insert(agent_id, event_task);

        Ok(())
    }

    /// Forward a Swank event to the EventBus.
    async fn forward_swank_event(
        event_bus: &EventBus,
        agent_id: Uuid,
        msg: SwankMessage,
    ) {
        let event = match msg {
            SwankMessage::Debug { condition, restarts, frames, .. } => {
                DescartesEvent::AgentEvent(AgentEvent {
                    id: Uuid::new_v4().to_string(),
                    agent_id: agent_id.to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: AgentEventType::DebuggerPaused,
                    data: serde_json::json!({
                        "condition": condition,
                        "restarts": restarts.iter().map(|r| serde_json::json!({
                            "index": r.index,
                            "name": r.name,
                            "description": r.description,
                        })).collect::<Vec<_>>(),
                        "frames": frames.iter().map(|f| serde_json::json!({
                            "index": f.index,
                            "description": f.description,
                        })).collect::<Vec<_>>(),
                    }),
                })
            }
            SwankMessage::WriteString(text) => {
                DescartesEvent::AgentEvent(AgentEvent {
                    id: Uuid::new_v4().to_string(),
                    agent_id: agent_id.to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: AgentEventType::SwankOutput,
                    data: serde_json::json!({ "text": text }),
                })
            }
            SwankMessage::Return { id, value } => {
                DescartesEvent::AgentEvent(AgentEvent {
                    id: Uuid::new_v4().to_string(),
                    agent_id: agent_id.to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: AgentEventType::Log,
                    data: serde_json::json!({
                        "type": "swank_return",
                        "request_id": id,
                        "value": value,
                    }),
                })
            }
            SwankMessage::Abort { id, reason } => {
                DescartesEvent::AgentEvent(AgentEvent {
                    id: Uuid::new_v4().to_string(),
                    agent_id: agent_id.to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: AgentEventType::Log,
                    data: serde_json::json!({
                        "type": "swank_abort",
                        "request_id": id,
                        "reason": reason,
                    }),
                })
            }
        };

        event_bus.publish(event).await;
    }

    /// Cleanup Swank session for an agent.
    pub async fn cleanup_swank_session(&self, agent_id: &Uuid) {
        // Stop event forwarding task
        if let Some((_, task)) = self.swank_event_tasks.remove(agent_id) {
            task.abort();
            debug!("Stopped Swank event task for agent {}", agent_id);
        }

        // Remove from global registry
        SWANK_REGISTRY.remove(agent_id);

        // Kill SBCL process
        if let Some((_, mut child)) = self.sbcl_processes.remove(agent_id) {
            if let Err(e) = child.kill().await {
                warn!("Failed to kill SBCL process for agent {}: {}", agent_id, e);
            } else {
                info!("Killed SBCL process for agent {}", agent_id);
            }
        }
    }

    pub(crate) async fn list_tasks_internal(
        &self,
        filter: Option<Value>,
    ) -> Result<Vec<TaskInfo>, ErrorObjectOwned> {
        info!("Listing tasks with filter: {:?}", filter);

        let tasks = self.state_store.get_tasks().await.map_err(|e| {
            error!("Failed to get tasks: {}", e);
            ErrorObjectOwned::owned(-32603, format!("Failed to get tasks: {}", e), None::<()>)
        })?;

        let mut filtered_tasks = tasks;

        if let Some(filter_obj) = filter {
            if let Some(status) = filter_obj.get("status").and_then(|s| s.as_str()) {
                filtered_tasks.retain(|task| {
                    format!("{:?}", task.status).to_lowercase() == status.to_lowercase()
                });
            }

            if let Some(assigned_to) = filter_obj.get("assigned_to").and_then(|s| s.as_str()) {
                filtered_tasks.retain(|task| task.assigned_to.as_deref() == Some(assigned_to));
            }
        }

        let task_infos: Vec<TaskInfo> = filtered_tasks
            .into_iter()
            .map(|task| TaskInfo {
                id: task.id.to_string(),
                name: task.title,
                status: format!("{:?}", task.status),
                created_at: task.created_at,
                updated_at: task.updated_at,
            })
            .collect();

        info!("Found {} tasks", task_infos.len());
        Ok(task_infos)
    }

    pub(crate) async fn approve_task_internal(
        &self,
        task_id: String,
        approved: bool,
    ) -> Result<ApprovalResult, ErrorObjectOwned> {
        info!("Approving task: {} (approved: {})", task_id, approved);

        let task_uuid = Uuid::parse_str(&task_id).map_err(|e| {
            error!("Invalid task ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid task ID format: {}", e), None::<()>)
        })?;

        let mut task = self
            .state_store
            .get_task(&task_uuid)
            .await
            .map_err(|e| {
                error!("Failed to get task: {}", e);
                ErrorObjectOwned::owned(-32603, format!("Failed to get task: {}", e), None::<()>)
            })?
            .ok_or_else(|| {
                error!("Task not found: {}", task_id);
                ErrorObjectOwned::owned(-32602, format!("Task not found: {}", task_id), None::<()>)
            })?;

        task.status = if approved {
            TaskStatus::InProgress
        } else {
            TaskStatus::Blocked
        };
        task.updated_at = chrono::Utc::now().timestamp();

        let mut metadata = task.metadata.unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("approved".to_string(), serde_json::json!(approved));
            obj.insert(
                "approval_timestamp".to_string(),
                serde_json::json!(task.updated_at),
            );
        }
        task.metadata = Some(metadata);

        self.state_store.save_task(&task).await.map_err(|e| {
            error!("Failed to save task: {}", e);
            ErrorObjectOwned::owned(-32603, format!("Failed to save task: {}", e), None::<()>)
        })?;

        Ok(ApprovalResult {
            task_id,
            approved,
            timestamp: task.updated_at,
        })
    }

    pub(crate) async fn get_state_internal(
        &self,
        entity_id: Option<String>,
    ) -> Result<Value, ErrorObjectOwned> {
        info!("Getting state for entity: {:?}", entity_id);

        if let Some(entity_id_str) = entity_id {
            if let Ok(agent_uuid) = Uuid::parse_str(&entity_id_str) {
                let agent_info = self
                    .agent_runner
                    .get_agent(&agent_uuid)
                    .await
                    .map_err(|e| {
                        error!("Failed to get agent info: {}", e);
                        ErrorObjectOwned::owned(
                            -32603,
                            format!("Failed to get agent info: {}", e),
                            None::<()>,
                        )
                    })?;

                if let Some(info) = agent_info {
                    let state = serde_json::json!({
                        "entity_type": "agent",
                        "entity_id": entity_id_str,
                        "name": info.name,
                        "status": format!("{:?}", info.status),
                        "model_backend": info.model_backend,
                        "started_at": info.started_at.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        "task": info.task,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    return Ok(state);
                } else {
                    return Err(ErrorObjectOwned::owned(
                        -32602,
                        format!("Agent not found: {}", entity_id_str),
                        None::<()>,
                    ));
                }
            }

            return Err(ErrorObjectOwned::owned(
                -32602,
                format!("Invalid entity ID format: {}", entity_id_str),
                None::<()>,
            ));
        }

        let agents = self.agent_runner.list_agents().await.map_err(|e| {
            error!("Failed to list agents: {}", e);
            ErrorObjectOwned::owned(-32603, format!("Failed to list agents: {}", e), None::<()>)
        })?;

        let tasks = self.state_store.get_tasks().await.map_err(|e| {
            error!("Failed to get tasks: {}", e);
            ErrorObjectOwned::owned(-32603, format!("Failed to get tasks: {}", e), None::<()>)
        })?;

        Ok(serde_json::json!({
            "entity_type": "system",
            "agents": {
                "total": agents.len(),
                "running": agents.iter().filter(|a| {
                    matches!(a.status, descartes_core::traits::AgentStatus::Running)
                }).count(),
            },
            "tasks": {
                "total": tasks.len(),
                "todo": tasks.iter().filter(|t| t.status == TaskStatus::Todo).count(),
                "in_progress": tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count(),
                "done": tasks.iter().filter(|t| t.status == TaskStatus::Done).count(),
                "blocked": tasks.iter().filter(|t| t.status == TaskStatus::Blocked).count(),
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    pub(crate) async fn pause_agent_internal(
        &self,
        agent_id: String,
        force: bool,
    ) -> Result<PauseResult, ErrorObjectOwned> {
        info!("Pausing agent: {} (force: {})", agent_id, force);

        let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            error!("Invalid agent ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid agent ID format: {}", e), None::<()>)
        })?;

        // Check if agent exists and is running
        let agent_info = self
            .agent_runner
            .get_agent(&agent_uuid)
            .await
            .map_err(|e| {
                error!("Failed to get agent: {}", e);
                ErrorObjectOwned::owned(-32603, format!("Failed to get agent: {}", e), None::<()>)
            })?
            .ok_or_else(|| {
                error!("Agent not found: {}", agent_id);
                ErrorObjectOwned::owned(-32002, format!("Agent not found: {}", agent_id), None::<()>)
            })?;

        // Check if agent is running
        if !matches!(agent_info.status, descartes_core::traits::AgentStatus::Running) {
            return Err(ErrorObjectOwned::owned(
                -32013,
                format!("Agent is not running (status: {:?})", agent_info.status),
                None::<()>,
            ));
        }

        // Pause the agent
        self.agent_runner.pause(&agent_uuid, force).await.map_err(|e| {
            error!("Failed to pause agent: {}", e);
            ErrorObjectOwned::owned(-32013, format!("Failed to pause agent: {}", e), None::<()>)
        })?;

        let pause_mode = if force { "forced" } else { "cooperative" };
        let paused_at = chrono::Utc::now().timestamp();

        // Start attach server for the paused agent if we have LocalProcessRunner
        if let Some(ref local_runner) = self.local_runner {
            if let Some(handle) = local_runner.get_agent_handle(&agent_uuid) {
                let handle_guard = handle.read();
                let agent_info = handle_guard.agent_info();

                // Get I/O channels
                let stdin_tx = handle_guard.get_stdin_sender();
                let stdout_tx = handle_guard.stdout_broadcast_sender();
                let stderr_tx = handle_guard.stderr_broadcast_sender();

                // Create socket path
                let socket_path = std::path::PathBuf::from(format!(
                    "/tmp/descartes-attach-{}.sock",
                    agent_uuid
                ));

                let attach_manager = Arc::clone(&self.attach_manager);
                let agent_name = agent_info.name.clone();
                let agent_task = agent_info.task.clone();

                drop(handle_guard); // Release the read lock before spawning

                // Spawn attach server in background
                let server_handle = tokio::spawn(async move {
                    if let Err(e) = crate::claude_code_tui::start_attach_server(
                        &socket_path,
                        crate::claude_code_tui::ClaudeCodeTuiConfig::default(),
                        attach_manager,
                        agent_uuid,
                        agent_name,
                        agent_task,
                        stdin_tx,
                        stdout_tx,
                        stderr_tx,
                    )
                    .await
                    {
                        error!("Attach server error for agent {}: {}", agent_uuid, e);
                    }
                });

                // Store the handle so we can stop it when agent resumes
                self.attach_servers.insert(agent_uuid, server_handle);
                info!("Started attach server for paused agent {}", agent_uuid);
            } else {
                warn!("Could not get agent handle for attach server (agent {})", agent_uuid);
            }
        } else {
            debug!("LocalProcessRunner not available, skipping attach server");
        }

        info!("Agent {} paused successfully (mode: {})", agent_id, pause_mode);

        Ok(PauseResult {
            agent_id,
            paused_at,
            pause_mode: pause_mode.to_string(),
        })
    }

    pub(crate) async fn resume_agent_internal(
        &self,
        agent_id: String,
    ) -> Result<ResumeResult, ErrorObjectOwned> {
        info!("Resuming agent: {}", agent_id);

        let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            error!("Invalid agent ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid agent ID format: {}", e), None::<()>)
        })?;

        // Check if agent exists and is paused
        let agent_info = self
            .agent_runner
            .get_agent(&agent_uuid)
            .await
            .map_err(|e| {
                error!("Failed to get agent: {}", e);
                ErrorObjectOwned::owned(-32603, format!("Failed to get agent: {}", e), None::<()>)
            })?
            .ok_or_else(|| {
                error!("Agent not found: {}", agent_id);
                ErrorObjectOwned::owned(-32002, format!("Agent not found: {}", agent_id), None::<()>)
            })?;

        // Check if agent is paused
        if !matches!(agent_info.status, descartes_core::traits::AgentStatus::Paused) {
            return Err(ErrorObjectOwned::owned(
                -32014,
                format!("Agent is not paused (status: {:?})", agent_info.status),
                None::<()>,
            ));
        }

        // Stop the attach server if running
        if let Some((_, server_handle)) = self.attach_servers.remove(&agent_uuid) {
            server_handle.abort();
            info!("Stopped attach server for agent {}", agent_uuid);

            // Clean up socket file
            let socket_path = format!("/tmp/descartes-attach-{}.sock", agent_uuid);
            let _ = std::fs::remove_file(&socket_path);

            // Terminate any active attach sessions for this agent
            let terminated_count = self.attach_manager.terminate_sessions_for_agent(&agent_uuid).await;
            if terminated_count > 0 {
                info!("Terminated {} attach session(s) for agent {}", terminated_count, agent_uuid);
            }
        }

        // Resume the agent
        self.agent_runner.resume(&agent_uuid).await.map_err(|e| {
            error!("Failed to resume agent: {}", e);
            ErrorObjectOwned::owned(-32014, format!("Failed to resume agent: {}", e), None::<()>)
        })?;

        let resumed_at = chrono::Utc::now().timestamp();

        info!("Agent {} resumed successfully", agent_id);

        Ok(ResumeResult {
            agent_id,
            resumed_at,
        })
    }

    pub(crate) async fn attach_request_internal(
        &self,
        agent_id: String,
        client_type: String,
    ) -> Result<AttachCredentialsResult, ErrorObjectOwned> {
        info!("Attach request for agent: {} from client: {}", agent_id, client_type);

        let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            error!("Invalid agent ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid agent ID format: {}", e), None::<()>)
        })?;

        // Check if agent exists and is paused
        let agent_info = self
            .agent_runner
            .get_agent(&agent_uuid)
            .await
            .map_err(|e| {
                error!("Failed to get agent: {}", e);
                ErrorObjectOwned::owned(-32603, format!("Failed to get agent: {}", e), None::<()>)
            })?
            .ok_or_else(|| {
                error!("Agent not found: {}", agent_id);
                ErrorObjectOwned::owned(-32002, format!("Agent not found: {}", agent_id), None::<()>)
            })?;

        // Agent should be paused to attach
        if !matches!(agent_info.status, descartes_core::traits::AgentStatus::Paused) {
            return Err(ErrorObjectOwned::owned(
                -32015,
                format!("Cannot attach to agent that is not paused (status: {:?})", agent_info.status),
                None::<()>,
            ));
        }

        // Check if attach server is running for this agent
        let socket_path = format!("/tmp/descartes-attach-{}.sock", agent_uuid);
        if !self.attach_servers.contains_key(&agent_uuid) {
            // Server not running - this could happen if daemon was restarted after pause
            // or if using a runner that doesn't support TUI attachment
            warn!("Attach server not running for agent {}, socket may not be available", agent_uuid);
        }

        // Parse client type
        let parsed_client_type = match client_type.as_str() {
            "claude-code" => crate::attach_session::ClientType::ClaudeCode,
            "opencode" => crate::attach_session::ClientType::OpenCode,
            other => crate::attach_session::ClientType::Custom(other.to_string()),
        };

        // Request attach credentials
        let credentials = self
            .attach_manager
            .request_attach(agent_uuid, parsed_client_type)
            .await
            .map_err(|e| {
                error!("Failed to create attach credentials: {}", e);
                ErrorObjectOwned::owned(-32015, format!("Failed to create attach credentials: {}", e), None::<()>)
            })?;

        info!("Attach credentials created for agent {}: token={}", agent_id, credentials.token);

        // Use the actual Unix socket path instead of the ZMQ IPC path from the attach manager
        Ok(AttachCredentialsResult {
            agent_id,
            token: credentials.token,
            connect_url: socket_path,
            expires_at: credentials.expires_at,
        })
    }

    pub(crate) async fn attach_validate_internal(
        &self,
        token: String,
    ) -> Result<AttachValidateResult, ErrorObjectOwned> {
        info!("Validating attach token");

        match self.attach_manager.validate_token(&token).await {
            Some(agent_id) => {
                info!("Token valid for agent {}", agent_id);
                // Note: expires_at is not easily retrievable from the current API
                // since validate_token only returns the agent_id
                Ok(AttachValidateResult {
                    valid: true,
                    agent_id: Some(agent_id.to_string()),
                    expires_at: None, // Token expiration not exposed by validate_token
                })
            }
            None => {
                info!("Token invalid or expired");
                Ok(AttachValidateResult {
                    valid: false,
                    agent_id: None,
                    expires_at: None,
                })
            }
        }
    }

    pub(crate) async fn attach_revoke_internal(
        &self,
        token: String,
    ) -> Result<AttachRevokeResult, ErrorObjectOwned> {
        info!("Revoking attach token");

        // Use the token store to revoke the token
        let token_store = self.attach_manager.token_store();
        let revoked = token_store.revoke(&token).await;

        if revoked {
            info!("Token revoked successfully");
        } else {
            info!("Token not found or already revoked");
        }

        Ok(AttachRevokeResult { revoked })
    }

    pub(crate) async fn swank_restart_internal(
        &self,
        agent_id: String,
        restart_index: usize,
    ) -> Result<SwankRestartResult, ErrorObjectOwned> {
        info!("Invoking Swank restart {} for agent {}", restart_index, agent_id);

        let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            error!("Invalid agent ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid agent ID format: {}", e), None::<()>)
        })?;

        // Get Swank client from registry
        let swank_client = SWANK_REGISTRY.get(&agent_uuid).ok_or_else(|| {
            error!("No Swank session for agent {}", agent_id);
            ErrorObjectOwned::owned(
                -32016,
                format!("No Swank session for agent {}", agent_id),
                None::<()>,
            )
        })?;

        // Invoke the restart
        match swank_client.invoke_restart(restart_index).await {
            Ok(_) => {
                info!("Swank restart {} invoked successfully for agent {}", restart_index, agent_id);
                Ok(SwankRestartResult {
                    agent_id,
                    restart_index,
                    success: true,
                    message: None,
                })
            }
            Err(e) => {
                error!("Failed to invoke Swank restart: {}", e);
                Ok(SwankRestartResult {
                    agent_id,
                    restart_index,
                    success: false,
                    message: Some(e.to_string()),
                })
            }
        }
    }
}

// Import ErrorObjectOwned for the trait implementation
use jsonrpsee::types::ErrorObjectOwned;

#[async_trait]
impl DescartesRpcServer for RpcServerImpl {
    async fn spawn(
        &self,
        name: String,
        agent_type: String,
        config: Value,
    ) -> Result<String, ErrorObjectOwned> {
        self.spawn_agent_internal(name, agent_type, config).await
    }

    async fn list_tasks(&self, filter: Option<Value>) -> Result<Vec<TaskInfo>, ErrorObjectOwned> {
        self.list_tasks_internal(filter).await
    }

    async fn approve(
        &self,
        task_id: String,
        approved: bool,
    ) -> Result<ApprovalResult, ErrorObjectOwned> {
        self.approve_task_internal(task_id, approved).await
    }

    async fn get_state(&self, entity_id: Option<String>) -> Result<Value, ErrorObjectOwned> {
        self.get_state_internal(entity_id).await
    }

    async fn pause_agent(
        &self,
        agent_id: String,
        force: bool,
    ) -> Result<PauseResult, ErrorObjectOwned> {
        self.pause_agent_internal(agent_id, force).await
    }

    async fn resume_agent(&self, agent_id: String) -> Result<ResumeResult, ErrorObjectOwned> {
        self.resume_agent_internal(agent_id).await
    }

    async fn attach_request(
        &self,
        agent_id: String,
        client_type: String,
    ) -> Result<AttachCredentialsResult, ErrorObjectOwned> {
        self.attach_request_internal(agent_id, client_type).await
    }

    async fn attach_validate(&self, token: String) -> Result<AttachValidateResult, ErrorObjectOwned> {
        self.attach_validate_internal(token).await
    }

    async fn attach_revoke(&self, token: String) -> Result<AttachRevokeResult, ErrorObjectOwned> {
        self.attach_revoke_internal(token).await
    }
}

/// Unix socket RPC server
pub struct UnixSocketRpcServer {
    socket_path: PathBuf,
    server_impl: Arc<RpcServerImpl>,
}

/// Handle returned by the Unix socket RPC server.
pub struct UnixServerHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl UnixServerHandle {
    /// Stop the running server.
    pub fn stop(&mut self) -> DaemonResult<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            tx.send(())
                .map_err(|_| DaemonError::ServerError("RPC server already stopped".to_string()))?;
        }
        Ok(())
    }

    /// Compatibility helper to mirror jsonrpsee's ServerHandle API.
    pub async fn stopped(self) {
        let mut handle = self;
        if let Err(err) = handle.stop() {
            warn!("Error while stopping RPC server: {}", err);
        }
    }
}

impl Drop for UnixServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl UnixSocketRpcServer {
    /// Create a new Unix socket RPC server.
    pub fn new(
        socket_path: PathBuf,
        agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
        state_store: Arc<dyn descartes_core::traits::StateStore>,
    ) -> Self {
        Self {
            socket_path,
            server_impl: Arc::new(RpcServerImpl::new(agent_runner, state_store)),
        }
    }

    /// Start listening for JSON-RPC requests over a Unix domain socket.
    pub async fn start(&self) -> DaemonResult<UnixServerHandle> {
        if self.socket_path.exists() {
            info!("Removing existing socket file: {:?}", self.socket_path);
            std::fs::remove_file(&self.socket_path).map_err(|e| {
                DaemonError::ServerError(format!("Failed to remove existing socket: {}", e))
            })?;
        }

        if let Some(parent) = self.socket_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    DaemonError::ServerError(format!("Failed to create socket directory: {}", e))
                })?;
            }
        }

        let listener = UnixListener::bind(&self.socket_path).map_err(|e| {
            DaemonError::ServerError(format!("Failed to bind to Unix socket: {}", e))
        })?;

        info!("RPC server listening on {:?}", self.socket_path);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server_impl = Arc::clone(&self.server_impl);
        let socket_path = self.socket_path.clone();

        tokio::spawn(async move {
            Self::run_listener(listener, server_impl, socket_path, shutdown_rx).await;
        });

        Ok(UnixServerHandle {
            shutdown_tx: Some(shutdown_tx),
        })
    }

    async fn run_listener(
        listener: UnixListener,
        server_impl: Arc<RpcServerImpl>,
        socket_path: PathBuf,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let listener = listener;
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    info!("Shutting down Unix RPC server");
                    break;
                }
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            let impl_clone = Arc::clone(&server_impl);
                            tokio::spawn(async move {
                                if let Err(err) = Self::handle_connection(stream, impl_clone).await {
                                    error!("Unix RPC connection error: {}", err);
                                }
                            });
                        }
                        Err(err) => {
                            error!("Unix RPC accept error: {}", err);
                            break;
                        }
                    }
                }
            }
        }

        if let Err(err) = tokio::fs::remove_file(&socket_path).await {
            warn!(
                "Failed to remove Unix socket {:?} during shutdown: {}",
                socket_path, err
            );
        }
    }

    async fn handle_connection(
        stream: UnixStream,
        server_impl: Arc<RpcServerImpl>,
    ) -> DaemonResult<()> {
        let mut reader = BufReader::new(stream);
        let mut payload = String::new();
        let bytes_read = reader
            .read_line(&mut payload)
            .await
            .map_err(|e| DaemonError::ServerError(format!("Failed to read RPC request: {}", e)))?;

        if bytes_read == 0 {
            return Ok(());
        }

        let response = Self::handle_payload(server_impl, payload.trim()).await;
        let mut stream = reader.into_inner();
        stream.write_all(response.as_bytes()).await.map_err(|e| {
            DaemonError::ServerError(format!("Failed to write RPC response: {}", e))
        })?;
        stream.write_all(b"\n").await.map_err(|e| {
            DaemonError::ServerError(format!("Failed to write RPC terminator: {}", e))
        })?;
        stream
            .shutdown()
            .await
            .map_err(|e| DaemonError::ServerError(format!("Failed to shutdown stream: {}", e)))?;

        Ok(())
    }

    async fn handle_payload(server_impl: Arc<RpcServerImpl>, payload: &str) -> String {
        if payload.is_empty() {
            return serde_json::to_string(&RpcResponse::error(
                -32600,
                "Invalid Request".to_string(),
                None,
            ))
            .unwrap();
        }

        let trimmed = payload.trim();
        if trimmed.starts_with('[') {
            match serde_json::from_str::<Vec<RpcRequest>>(trimmed) {
                Ok(requests) if !requests.is_empty() => {
                    let mut responses = Vec::with_capacity(requests.len());
                    for request in requests {
                        responses.push(
                            Self::process_single_request(Arc::clone(&server_impl), request).await,
                        );
                    }
                    serde_json::to_string(&responses).unwrap_or_else(|e| {
                        serde_json::to_string(&RpcResponse::error(
                            -32603,
                            format!("Serialization error: {}", e),
                            None,
                        ))
                        .unwrap()
                    })
                }
                Ok(_) => serde_json::to_string(&RpcResponse::error(
                    -32600,
                    "Invalid Request".to_string(),
                    None,
                ))
                .unwrap(),
                Err(_) => serde_json::to_string(&RpcResponse::error(
                    -32700,
                    "Parse error".to_string(),
                    None,
                ))
                .unwrap(),
            }
        } else {
            match serde_json::from_str::<RpcRequest>(trimmed) {
                Ok(request) => {
                    serde_json::to_string(&Self::process_single_request(server_impl, request).await)
                        .unwrap_or_else(|e| {
                            serde_json::to_string(&RpcResponse::error(
                                -32603,
                                format!("Serialization error: {}", e),
                                None,
                            ))
                            .unwrap()
                        })
                }
                Err(_) => serde_json::to_string(&RpcResponse::error(
                    -32700,
                    "Parse error".to_string(),
                    None,
                ))
                .unwrap(),
            }
        }
    }

    async fn process_single_request(
        server_impl: Arc<RpcServerImpl>,
        request: RpcRequest,
    ) -> RpcResponse {
        let method = request.method.clone();
        match method.as_str() {
            "spawn" | "agent.spawn" => match Self::parse_spawn_params(&request) {
                Ok((name, agent_type, config)) => match server_impl
                    .spawn_agent_internal(name, agent_type, config)
                    .await
                {
                    Ok(agent_id) => RpcResponse::success(json!(agent_id), request.id.clone()),
                    Err(err) => Self::convert_error(err, request.id.clone()),
                },
                Err(response) => response,
            },
            "list_tasks" | "task.list" => match Self::parse_list_params(&request) {
                Ok(filter) => match server_impl.list_tasks_internal(filter).await {
                    Ok(tasks) => match serde_json::to_value(tasks) {
                        Ok(value) => RpcResponse::success(value, request.id.clone()),
                        Err(e) => RpcResponse::error(
                            -32603,
                            format!("Serialization error: {}", e),
                            request.id.clone(),
                        ),
                    },
                    Err(err) => Self::convert_error(err, request.id.clone()),
                },
                Err(response) => response,
            },
            "approve" | "task.approve" => match Self::parse_approve_params(&request) {
                Ok((task_id, approved)) => {
                    match server_impl.approve_task_internal(task_id, approved).await {
                        Ok(result) => match serde_json::to_value(result) {
                            Ok(value) => RpcResponse::success(value, request.id.clone()),
                            Err(e) => RpcResponse::error(
                                -32603,
                                format!("Serialization error: {}", e),
                                request.id.clone(),
                            ),
                        },
                        Err(err) => Self::convert_error(err, request.id.clone()),
                    }
                }
                Err(response) => response,
            },
            "get_state" | "state.get" => match Self::parse_state_params(&request) {
                Ok(entity_id) => match server_impl.get_state_internal(entity_id).await {
                    Ok(value) => RpcResponse::success(value, request.id.clone()),
                    Err(err) => Self::convert_error(err, request.id.clone()),
                },
                Err(response) => response,
            },
            "agent.pause" => match Self::parse_pause_params(&request) {
                Ok((agent_id, force)) => {
                    match server_impl.pause_agent_internal(agent_id, force).await {
                        Ok(result) => match serde_json::to_value(result) {
                            Ok(value) => RpcResponse::success(value, request.id.clone()),
                            Err(e) => RpcResponse::error(
                                -32603,
                                format!("Serialization error: {}", e),
                                request.id.clone(),
                            ),
                        },
                        Err(err) => Self::convert_error(err, request.id.clone()),
                    }
                }
                Err(response) => response,
            },
            "agent.resume" => match Self::parse_resume_params(&request) {
                Ok(agent_id) => match server_impl.resume_agent_internal(agent_id).await {
                    Ok(result) => match serde_json::to_value(result) {
                        Ok(value) => RpcResponse::success(value, request.id.clone()),
                        Err(e) => RpcResponse::error(
                            -32603,
                            format!("Serialization error: {}", e),
                            request.id.clone(),
                        ),
                    },
                    Err(err) => Self::convert_error(err, request.id.clone()),
                },
                Err(response) => response,
            },
            "agent.attach.request" => match Self::parse_attach_request_params(&request) {
                Ok((agent_id, client_type)) => {
                    match server_impl.attach_request_internal(agent_id, client_type).await {
                        Ok(result) => match serde_json::to_value(result) {
                            Ok(value) => RpcResponse::success(value, request.id.clone()),
                            Err(e) => RpcResponse::error(
                                -32603,
                                format!("Serialization error: {}", e),
                                request.id.clone(),
                            ),
                        },
                        Err(err) => Self::convert_error(err, request.id.clone()),
                    }
                }
                Err(response) => response,
            },
            "agent.attach.validate" => match Self::parse_token_params(&request) {
                Ok(token) => match server_impl.attach_validate_internal(token).await {
                    Ok(result) => match serde_json::to_value(result) {
                        Ok(value) => RpcResponse::success(value, request.id.clone()),
                        Err(e) => RpcResponse::error(
                            -32603,
                            format!("Serialization error: {}", e),
                            request.id.clone(),
                        ),
                    },
                    Err(err) => Self::convert_error(err, request.id.clone()),
                },
                Err(response) => response,
            },
            "agent.attach.revoke" => match Self::parse_token_params(&request) {
                Ok(token) => match server_impl.attach_revoke_internal(token).await {
                    Ok(result) => match serde_json::to_value(result) {
                        Ok(value) => RpcResponse::success(value, request.id.clone()),
                        Err(e) => RpcResponse::error(
                            -32603,
                            format!("Serialization error: {}", e),
                            request.id.clone(),
                        ),
                    },
                    Err(err) => Self::convert_error(err, request.id.clone()),
                },
                Err(response) => response,
            },
            "swank.restart" => match Self::parse_swank_restart_params(&request) {
                Ok((agent_id, restart_index)) => {
                    match server_impl.swank_restart_internal(agent_id, restart_index).await {
                        Ok(result) => match serde_json::to_value(result) {
                            Ok(value) => RpcResponse::success(value, request.id.clone()),
                            Err(e) => RpcResponse::error(
                                -32603,
                                format!("Serialization error: {}", e),
                                request.id.clone(),
                            ),
                        },
                        Err(err) => Self::convert_error(err, request.id.clone()),
                    }
                }
                Err(response) => response,
            },
            _ => RpcResponse::error(-32601, "Method not found".to_string(), request.id.clone()),
        }
    }

    #[allow(clippy::result_large_err)]
    fn parse_spawn_params(request: &RpcRequest) -> Result<(String, String, Value), RpcResponse> {
        let params = match &request.params {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(Self::invalid_params(
                    request.id.clone(),
                    "Expected positional parameters",
                ))
            }
        };

        let name = params.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Self::invalid_params(request.id.clone(), "Missing agent name parameter")
            })?
            .to_string();
        let agent_type = params
            .get(1)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Self::invalid_params(request.id.clone(), "Missing agent type parameter")
            })?
            .to_string();
        let config = params
            .get(2)
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default()));

        Ok((name, agent_type, config))
    }

    #[allow(clippy::result_large_err)]
    fn parse_list_params(request: &RpcRequest) -> Result<Option<Value>, RpcResponse> {
        match &request.params {
            None => Ok(None),
            Some(Value::Array(arr)) => Ok(arr.first().cloned().filter(|v| !v.is_null())),
            Some(Value::Null) => Ok(None),
            _ => Err(Self::invalid_params(
                request.id.clone(),
                "Expected optional filter parameter",
            )),
        }
    }

    #[allow(clippy::result_large_err)]
    fn parse_approve_params(request: &RpcRequest) -> Result<(String, bool), RpcResponse> {
        let params = match &request.params {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(Self::invalid_params(
                    request.id.clone(),
                    "Expected positional parameters",
                ))
            }
        };

        let task_id = params.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing task_id parameter"))?
            .to_string();
        let approved = params.get(1).and_then(|v| v.as_bool()).ok_or_else(|| {
            Self::invalid_params(request.id.clone(), "Missing approved parameter")
        })?;

        Ok((task_id, approved))
    }

    #[allow(clippy::result_large_err)]
    fn parse_state_params(request: &RpcRequest) -> Result<Option<String>, RpcResponse> {
        match &request.params {
            None => Ok(None),
            Some(Value::Array(arr)) => Ok(arr.first().and_then(|value| {
                if value.is_null() {
                    None
                } else {
                    value.as_str().map(|s| s.to_string())
                }
            })),
            Some(Value::Null) => Ok(None),
            _ => Err(Self::invalid_params(
                request.id.clone(),
                "Expected optional entity_id parameter",
            )),
        }
    }

    #[allow(clippy::result_large_err)]
    fn parse_pause_params(request: &RpcRequest) -> Result<(String, bool), RpcResponse> {
        let params = match &request.params {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(Self::invalid_params(
                    request.id.clone(),
                    "Expected positional parameters [agent_id, force]",
                ))
            }
        };

        let agent_id = params.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
            .to_string();

        // force defaults to false if not provided
        let force = params
            .get(1)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok((agent_id, force))
    }

    #[allow(clippy::result_large_err)]
    fn parse_resume_params(request: &RpcRequest) -> Result<String, RpcResponse> {
        let params = match &request.params {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(Self::invalid_params(
                    request.id.clone(),
                    "Expected positional parameters [agent_id]",
                ))
            }
        };

        let agent_id = params.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
            .to_string();

        Ok(agent_id)
    }

    #[allow(clippy::result_large_err)]
    fn parse_attach_request_params(request: &RpcRequest) -> Result<(String, String), RpcResponse> {
        let params = match &request.params {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(Self::invalid_params(
                    request.id.clone(),
                    "Expected positional parameters [agent_id, client_type]",
                ))
            }
        };

        let agent_id = params.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
            .to_string();

        // client_type defaults to "claude-code" if not provided
        let client_type = params
            .get(1)
            .and_then(|v| v.as_str())
            .unwrap_or("claude-code")
            .to_string();

        Ok((agent_id, client_type))
    }

    #[allow(clippy::result_large_err)]
    fn parse_token_params(request: &RpcRequest) -> Result<String, RpcResponse> {
        let params = match &request.params {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(Self::invalid_params(
                    request.id.clone(),
                    "Expected positional parameters [token]",
                ))
            }
        };

        let token = params.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing token parameter"))?
            .to_string();

        Ok(token)
    }

    #[allow(clippy::result_large_err)]
    fn parse_swank_restart_params(request: &RpcRequest) -> Result<(String, usize), RpcResponse> {
        // Support both positional array and named object params
        match &request.params {
            Some(Value::Array(arr)) => {
                let agent_id = arr.first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
                    .to_string();
                let restart_index = arr.get(1)
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing restart_index parameter"))?
                    as usize;
                Ok((agent_id, restart_index))
            }
            Some(Value::Object(obj)) => {
                let agent_id = obj.get("agent_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
                    .to_string();
                let restart_index = obj.get("restart_index")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing restart_index parameter"))?
                    as usize;
                Ok((agent_id, restart_index))
            }
            _ => Err(Self::invalid_params(
                request.id.clone(),
                "Expected parameters [agent_id, restart_index] or {agent_id, restart_index}",
            )),
        }
    }

    fn convert_error(err: ErrorObjectOwned, id: Option<Value>) -> RpcResponse {
        let data = err
            .data()
            .and_then(|raw| serde_json::from_str(raw.get()).ok());
        RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError {
                code: err.code() as i64,
                message: err.message().to_string(),
                data,
            }),
            id,
        }
    }

    fn invalid_params(id: Option<Value>, message: impl Into<String>) -> RpcResponse {
        RpcResponse::error(-32602, message.into(), id)
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }
}

impl Clone for RpcServerImpl {
    fn clone(&self) -> Self {
        Self {
            agent_runner: Arc::clone(&self.agent_runner),
            local_runner: self.local_runner.as_ref().map(Arc::clone),
            state_store: Arc::clone(&self.state_store),
            agent_ids: Arc::clone(&self.agent_ids),
            attach_manager: Arc::clone(&self.attach_manager),
            event_bus: Arc::clone(&self.event_bus),
            attach_servers: Arc::clone(&self.attach_servers),
            sbcl_processes: Arc::clone(&self.sbcl_processes),
            swank_event_tasks: Arc::clone(&self.swank_event_tasks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use descartes_core::agent_runner::LocalProcessRunner;
    use descartes_core::state_store::SqliteStateStore;
    use descartes_core::traits::{StateStore, Task};
    use tempfile::tempdir;

    async fn create_test_dependencies() -> (
        Arc<dyn descartes_core::traits::AgentRunner>,
        Arc<dyn descartes_core::traits::StateStore>,
        tempfile::TempDir,
    ) {
        let agent_runner =
            Arc::new(LocalProcessRunner::new()) as Arc<dyn descartes_core::traits::AgentRunner>;

        let temp_db = tempdir().unwrap();
        let db_path = temp_db.path().join("test.db");
        let mut state_store = SqliteStateStore::new(db_path, true).await.unwrap();
        state_store.initialize().await.unwrap();
        let state_store = Arc::new(state_store) as Arc<dyn descartes_core::traits::StateStore>;

        (agent_runner, state_store, temp_db)
    }

    #[tokio::test]
    async fn test_server_creation() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");
        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;
        let server = UnixSocketRpcServer::new(socket_path.clone(), agent_runner, state_store);
        assert_eq!(server.socket_path(), &socket_path);
    }

    #[tokio::test]
    async fn test_task_info_serialization() {
        let task = TaskInfo {
            id: "task-1".to_string(),
            name: "Test Task".to_string(),
            status: "pending".to_string(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: TaskInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.name, deserialized.name);
    }

    #[tokio::test]
    async fn test_approval_result_serialization() {
        let result = ApprovalResult {
            task_id: "task-1".to_string(),
            approved: true,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ApprovalResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.task_id, deserialized.task_id);
        assert_eq!(result.approved, deserialized.approved);
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl.list_tasks(None).await;
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_list_tasks_with_data() {
        use descartes_core::traits::{TaskComplexity, TaskPriority};

        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;

        // Create test tasks
        let task1 = Task {
            id: Uuid::new_v4(),
            title: "Test Task 1".to_string(),
            description: Some("Description 1".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("agent-1".to_string()),
            dependencies: vec![],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            metadata: None,
        };

        let task2 = Task {
            id: Uuid::new_v4(),
            title: "Test Task 2".to_string(),
            description: Some("Description 2".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("agent-2".to_string()),
            dependencies: vec![],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            metadata: None,
        };

        state_store.save_task(&task1).await.unwrap();
        state_store.save_task(&task2).await.unwrap();

        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        // Test listing all tasks
        let result = server_impl.list_tasks(None).await;
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 2);

        // Test filtering by status
        let filter = serde_json::json!({ "status": "todo" });
        let result = server_impl.list_tasks(Some(filter)).await;
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "Todo");

        // Test filtering by assigned_to
        let filter = serde_json::json!({ "assigned_to": "agent-2" });
        let result = server_impl.list_tasks(Some(filter)).await;
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Test Task 2");
    }

    #[tokio::test]
    async fn test_approve_task() {
        use descartes_core::traits::{TaskComplexity, TaskPriority};

        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;

        // Create a test task
        let task = Task {
            id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("agent-1".to_string()),
            dependencies: vec![],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            metadata: None,
        };

        let task_id = task.id;
        state_store.save_task(&task).await.unwrap();

        let server_impl = RpcServerImpl::new(agent_runner, state_store.clone());

        // Test approval
        let result = server_impl.approve(task_id.to_string(), true).await;
        assert!(result.is_ok());
        let approval_result = result.unwrap();
        assert_eq!(approval_result.task_id, task_id.to_string());
        assert!(approval_result.approved);

        // Verify task status was updated
        let updated_task = state_store.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(updated_task.status, TaskStatus::InProgress);
        assert!(updated_task.metadata.is_some());
    }

    #[tokio::test]
    async fn test_approve_task_rejection() {
        use descartes_core::traits::{TaskComplexity, TaskPriority};

        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;

        // Create a test task
        let task = Task {
            id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("agent-1".to_string()),
            dependencies: vec![],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            metadata: None,
        };

        let task_id = task.id;
        state_store.save_task(&task).await.unwrap();

        let server_impl = RpcServerImpl::new(agent_runner, state_store.clone());

        // Test rejection
        let result = server_impl.approve(task_id.to_string(), false).await;
        assert!(result.is_ok());
        let approval_result = result.unwrap();
        assert_eq!(approval_result.task_id, task_id.to_string());
        assert!(!approval_result.approved);

        // Verify task status was updated to blocked
        let updated_task = state_store.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(updated_task.status, TaskStatus::Blocked);
    }

    #[tokio::test]
    async fn test_approve_nonexistent_task() {
        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let fake_task_id = Uuid::new_v4().to_string();
        let result = server_impl.approve(fake_task_id, true).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_approve_invalid_task_id() {
        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl.approve("invalid-uuid".to_string(), true).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_state_system() {
        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl.get_state(None).await;
        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state["entity_type"], "system");
        assert!(state["agents"].is_object());
        assert!(state["tasks"].is_object());
    }

    #[tokio::test]
    async fn test_get_state_invalid_entity() {
        let (agent_runner, state_store, _temp_db) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl
            .get_state(Some("invalid-uuid".to_string()))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_lisp_agent_detection() {
        use descartes_core::traits::AgentConfig;

        // Test model_backend detection (sbcl)
        let lisp_config = AgentConfig {
            name: "test-lisp".to_string(),
            model_backend: "sbcl".to_string(),
            task: "test".to_string(),
            context: None,
            system_prompt: None,
            environment: std::collections::HashMap::new(),
            ..Default::default()
        };
        assert!(is_lisp_agent(&lisp_config));

        // Test model_backend detection (lisp)
        let lisp_backend_config = AgentConfig {
            name: "test".to_string(),
            model_backend: "lisp".to_string(),
            task: "test".to_string(),
            context: None,
            system_prompt: None,
            environment: std::collections::HashMap::new(),
            ..Default::default()
        };
        assert!(is_lisp_agent(&lisp_backend_config));

        // Test name detection
        let lisp_name_config = AgentConfig {
            name: "my-lisp-agent".to_string(),
            model_backend: "claude".to_string(),
            task: "test".to_string(),
            context: None,
            system_prompt: None,
            environment: std::collections::HashMap::new(),
            ..Default::default()
        };
        assert!(is_lisp_agent(&lisp_name_config));

        // Test environment variable detection
        let mut env_config = AgentConfig {
            name: "test".to_string(),
            model_backend: "claude".to_string(),
            task: "test".to_string(),
            context: None,
            system_prompt: None,
            environment: std::collections::HashMap::new(),
            ..Default::default()
        };
        env_config.environment.insert(
            "DESCARTES_TOOL_LEVEL".to_string(),
            "lisp_developer".to_string(),
        );
        assert!(is_lisp_agent(&env_config));

        // Test non-lisp agent
        let non_lisp_config = AgentConfig {
            name: "regular-agent".to_string(),
            model_backend: "claude".to_string(),
            task: "test".to_string(),
            context: None,
            system_prompt: None,
            environment: std::collections::HashMap::new(),
            ..Default::default()
        };
        assert!(!is_lisp_agent(&non_lisp_config));
    }
}
