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
use descartes_core::traits::{AgentConfig, Task, TaskStatus};
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::{Server, ServerHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};
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
    async fn spawn(&self, name: String, agent_type: String, config: Value) -> Result<String, ErrorObjectOwned>;

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
    async fn approve(&self, task_id: String, approved: bool) -> Result<ApprovalResult, ErrorObjectOwned>;

    /// Get the current state of the system or a specific entity
    ///
    /// # Arguments
    /// * `entity_id` - Optional ID of a specific entity to query
    ///
    /// # Returns
    /// The current state
    #[method(name = "get_state")]
    async fn get_state(&self, entity_id: Option<String>) -> Result<Value, ErrorObjectOwned>;
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

/// Implementation of the RPC server
pub struct RpcServerImpl {
    /// Agent runner for spawning and managing agents
    agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
    /// State store for persisting tasks and events
    state_store: Arc<dyn descartes_core::traits::StateStore>,
    /// Mapping of agent IDs (String -> Uuid) for convenience
    agent_ids: Arc<dashmap::DashMap<String, uuid::Uuid>>,
}

impl RpcServerImpl {
    /// Create a new RPC server implementation
    pub fn new(
        agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
        state_store: Arc<dyn descartes_core::traits::StateStore>,
    ) -> Self {
        Self {
            agent_runner,
            state_store,
            agent_ids: Arc::new(dashmap::DashMap::new()),
        }
    }
}

// Import ErrorObjectOwned for the trait implementation
use jsonrpsee::types::ErrorObjectOwned;

#[async_trait]
impl DescartesRpcServer for RpcServerImpl {
    async fn spawn(&self, name: String, agent_type: String, config: Value) -> Result<String, ErrorObjectOwned> {
        info!("Spawning agent: {} (type: {})", name, agent_type);

        // Parse configuration from JSON
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

        // Create agent configuration
        let agent_config = AgentConfig {
            name: name.clone(),
            model_backend: agent_type,
            task,
            context,
            system_prompt,
            environment,
        };

        // Spawn the agent using the agent runner
        let agent_handle = self.agent_runner
            .spawn(agent_config)
            .await
            .map_err(|e| {
                error!("Failed to spawn agent: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to spawn agent: {}", e),
                    None::<()>,
                )
            })?;

        let agent_id = agent_handle.id();
        let agent_id_str = agent_id.to_string();

        // Store the mapping for future reference
        self.agent_ids.insert(agent_id_str.clone(), agent_id);

        info!("Agent spawned successfully with ID: {}", agent_id_str);
        Ok(agent_id_str)
    }

    async fn list_tasks(&self, filter: Option<Value>) -> Result<Vec<TaskInfo>, ErrorObjectOwned> {
        info!("Listing tasks with filter: {:?}", filter);

        // Get all tasks from state store
        let tasks = self.state_store
            .get_tasks()
            .await
            .map_err(|e| {
                error!("Failed to get tasks: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to get tasks: {}", e),
                    None::<()>,
                )
            })?;

        // Apply filters if provided
        let mut filtered_tasks = tasks;

        if let Some(filter_obj) = filter {
            if let Some(status) = filter_obj.get("status").and_then(|s| s.as_str()) {
                filtered_tasks.retain(|task| {
                    format!("{:?}", task.status).to_lowercase() == status.to_lowercase()
                });
            }

            if let Some(assigned_to) = filter_obj.get("assigned_to").and_then(|s| s.as_str()) {
                filtered_tasks.retain(|task| {
                    task.assigned_to.as_deref() == Some(assigned_to)
                });
            }
        }

        // Convert to TaskInfo format
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

    async fn approve(&self, task_id: String, approved: bool) -> Result<ApprovalResult, ErrorObjectOwned> {
        info!("Approving task: {} (approved: {})", task_id, approved);

        // Parse task ID as UUID
        let task_uuid = Uuid::parse_str(&task_id).map_err(|e| {
            error!("Invalid task ID format: {}", e);
            ErrorObjectOwned::owned(
                -32602,
                format!("Invalid task ID format: {}", e),
                None::<()>,
            )
        })?;

        // Get the task from state store
        let mut task = self.state_store
            .get_task(&task_uuid)
            .await
            .map_err(|e| {
                error!("Failed to get task: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to get task: {}", e),
                    None::<()>,
                )
            })?
            .ok_or_else(|| {
                error!("Task not found: {}", task_id);
                ErrorObjectOwned::owned(
                    -32602,
                    format!("Task not found: {}", task_id),
                    None::<()>,
                )
            })?;

        // Update task status based on approval
        task.status = if approved {
            TaskStatus::InProgress
        } else {
            TaskStatus::Blocked
        };
        task.updated_at = chrono::Utc::now().timestamp();

        // Add approval metadata
        let mut metadata = task.metadata.unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("approved".to_string(), serde_json::json!(approved));
            obj.insert("approval_timestamp".to_string(), serde_json::json!(task.updated_at));
        }
        task.metadata = Some(metadata);

        // Save the updated task
        self.state_store
            .save_task(&task)
            .await
            .map_err(|e| {
                error!("Failed to save task: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to save task: {}", e),
                    None::<()>,
                )
            })?;

        let result = ApprovalResult {
            task_id,
            approved,
            timestamp: task.updated_at,
        };

        info!("Task approval recorded successfully");
        Ok(result)
    }

    async fn get_state(&self, entity_id: Option<String>) -> Result<Value, ErrorObjectOwned> {
        info!("Getting state for entity: {:?}", entity_id);

        if let Some(entity_id_str) = entity_id {
            // Try to parse as agent ID
            if let Ok(agent_uuid) = Uuid::parse_str(&entity_id_str) {
                // Get agent info from agent runner
                let agent_info = self.agent_runner
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

            // If not an agent ID, return error
            return Err(ErrorObjectOwned::owned(
                -32602,
                format!("Invalid entity ID format: {}", entity_id_str),
                None::<()>,
            ));
        }

        // Return system-wide state
        let agents = self.agent_runner
            .list_agents()
            .await
            .map_err(|e| {
                error!("Failed to list agents: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to list agents: {}", e),
                    None::<()>,
                )
            })?;

        let tasks = self.state_store
            .get_tasks()
            .await
            .map_err(|e| {
                error!("Failed to get tasks: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to get tasks: {}", e),
                    None::<()>,
                )
            })?;

        let state = serde_json::json!({
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
        });

        Ok(state)
    }
}

/// Unix socket RPC server
pub struct UnixSocketRpcServer {
    socket_path: PathBuf,
    server_impl: RpcServerImpl,
}

impl UnixSocketRpcServer {
    /// Create a new Unix socket RPC server
    ///
    /// # Arguments
    /// * `socket_path` - Path to the Unix socket file
    /// * `agent_runner` - Agent runner for spawning and managing agents
    /// * `state_store` - State store for persisting tasks and events
    pub fn new(
        socket_path: PathBuf,
        agent_runner: Arc<dyn descartes_core::traits::AgentRunner>,
        state_store: Arc<dyn descartes_core::traits::StateStore>,
    ) -> Self {
        Self {
            socket_path,
            server_impl: RpcServerImpl::new(agent_runner, state_store),
        }
    }

    /// Start the RPC server
    ///
    /// # Returns
    /// A handle to the running server
    pub async fn start(&self) -> DaemonResult<ServerHandle> {
        // Remove existing socket file if it exists
        if self.socket_path.exists() {
            info!("Removing existing socket file: {:?}", self.socket_path);
            std::fs::remove_file(&self.socket_path).map_err(|e| {
                DaemonError::ServerError(format!("Failed to remove existing socket: {}", e))
            })?;
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.socket_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    DaemonError::ServerError(format!("Failed to create socket directory: {}", e))
                })?;
            }
        }

        info!("Starting RPC server on Unix socket: {:?}", self.socket_path);

        // Build the server
        let server = Server::builder()
            .build(self.socket_path.to_str().ok_or_else(|| {
                DaemonError::ServerError("Invalid socket path".to_string())
            })?)
            .await
            .map_err(|e| {
                DaemonError::ServerError(format!("Failed to bind to Unix socket: {}", e))
            })?;

        // Start the server with our RPC implementation
        let handle = server.start(self.server_impl.clone().into_rpc());

        info!("RPC server started successfully on {:?}", self.socket_path);

        Ok(handle)
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
            state_store: Arc::clone(&self.state_store),
            agent_ids: Arc::clone(&self.agent_ids),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use descartes_core::agent_runner::LocalProcessRunner;
    use descartes_core::state_store::SqliteStateStore;
    use tempfile::tempdir;

    async fn create_test_dependencies() -> (Arc<dyn descartes_core::traits::AgentRunner>, Arc<dyn descartes_core::traits::StateStore>) {
        let agent_runner = Arc::new(LocalProcessRunner::new()) as Arc<dyn descartes_core::traits::AgentRunner>;

        let temp_db = tempdir().unwrap();
        let db_path = temp_db.path().join("test.db");
        let mut state_store = SqliteStateStore::new(db_path, false).await.unwrap();
        state_store.initialize().await.unwrap();
        let state_store = Arc::new(state_store) as Arc<dyn descartes_core::traits::StateStore>;

        (agent_runner, state_store)
    }

    #[tokio::test]
    async fn test_server_creation() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");
        let (agent_runner, state_store) = create_test_dependencies().await;
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
        let (agent_runner, state_store) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl.list_tasks(None).await;
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_list_tasks_with_data() {
        use descartes_core::traits::{TaskPriority, TaskComplexity};

        let (agent_runner, state_store) = create_test_dependencies().await;

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
        use descartes_core::traits::{TaskPriority, TaskComplexity};

        let (agent_runner, state_store) = create_test_dependencies().await;

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
        use descartes_core::traits::{TaskPriority, TaskComplexity};

        let (agent_runner, state_store) = create_test_dependencies().await;

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
        let (agent_runner, state_store) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let fake_task_id = Uuid::new_v4().to_string();
        let result = server_impl.approve(fake_task_id, true).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_approve_invalid_task_id() {
        let (agent_runner, state_store) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl.approve("invalid-uuid".to_string(), true).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_state_system() {
        let (agent_runner, state_store) = create_test_dependencies().await;
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
        let (agent_runner, state_store) = create_test_dependencies().await;
        let server_impl = RpcServerImpl::new(agent_runner, state_store);

        let result = server_impl.get_state(Some("invalid-uuid".to_string())).await;
        assert!(result.is_err());
    }
}
