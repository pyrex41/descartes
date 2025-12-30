/// RPC method handlers
use crate::auth::AuthContext;
use crate::errors::{DaemonError, DaemonResult};
use crate::fly_machines::FlyMachinesClient;
use crate::project_store::ProjectStore;
use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use descartes_core::{AgentRunner, LocalProcessRunner, SqliteStateStore};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// RPC handlers
pub struct RpcHandlers {
    /// In-memory agent storage (for demo; would be backed by actual agent runner)
    agents: Arc<DashMap<String, AgentInfo>>,
    /// Optional agent runner for live agent operations
    runner: Option<Arc<LocalProcessRunner>>,
    /// Optional state store for state queries
    state_store: Option<Arc<RwLock<SqliteStateStore>>>,
    /// Optional Fly.io client for cloud spawning
    fly_client: Option<Arc<FlyMachinesClient>>,
    /// Callback base URL for cloud workers
    callback_base_url: Option<String>,
    /// Optional project store for webapp
    project_store: Option<Arc<ProjectStore>>,
}

impl RpcHandlers {
    /// Create new handlers
    pub fn new() -> Self {
        RpcHandlers {
            agents: Arc::new(DashMap::new()),
            runner: None,
            state_store: None,
            fly_client: None,
            callback_base_url: None,
            project_store: None,
        }
    }

    /// Create new handlers with runner and state store
    pub fn with_runner(runner: Arc<LocalProcessRunner>) -> Self {
        RpcHandlers {
            agents: Arc::new(DashMap::new()),
            runner: Some(runner),
            state_store: None,
            fly_client: None,
            callback_base_url: None,
            project_store: None,
        }
    }

    /// Create new handlers with full services
    pub fn with_services(
        runner: Arc<LocalProcessRunner>,
        state_store: Arc<RwLock<SqliteStateStore>>,
    ) -> Self {
        RpcHandlers {
            agents: Arc::new(DashMap::new()),
            runner: Some(runner),
            state_store: Some(state_store),
            fly_client: None,
            callback_base_url: None,
            project_store: None,
        }
    }

    /// Set the state store
    pub fn set_state_store(&mut self, state_store: Arc<RwLock<SqliteStateStore>>) {
        self.state_store = Some(state_store);
    }

    /// Set the agent runner
    pub fn set_runner(&mut self, runner: Arc<LocalProcessRunner>) {
        self.runner = Some(runner);
    }

    /// Set the Fly.io client for cloud spawning
    pub fn set_fly_client(&mut self, fly_client: Arc<FlyMachinesClient>) {
        self.fly_client = Some(fly_client);
    }

    /// Set the callback base URL for cloud workers
    pub fn set_callback_base_url(&mut self, url: String) {
        self.callback_base_url = Some(url);
    }

    /// Set the project store for webapp
    pub fn set_project_store(&mut self, project_store: Arc<ProjectStore>) {
        self.project_store = Some(project_store);
    }

    /// Handle agent.spawn RPC method
    pub async fn handle_agent_spawn(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: AgentSpawnRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

        // Check if cloud spawning is requested
        if request.cloud {
            return self.handle_cloud_spawn(request).await;
        }

        // Local spawn (existing logic)
        let agent_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let agent = AgentInfo {
            id: agent_id.clone(),
            name: request.name,
            status: AgentStatus::Running,
            created_at: now,
            updated_at: now,
            pid: Some(std::process::id()),
            config: request.config,
        };

        self.agents.insert(agent_id.clone(), agent.clone());

        let response = AgentSpawnResponse {
            agent_id,
            status: AgentStatus::Running,
            message: "Agent spawned successfully".to_string(),
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Handle cloud spawn via Fly.io
    async fn handle_cloud_spawn(&self, request: AgentSpawnRequest) -> DaemonResult<Value> {
        // Validate required fields for cloud spawn
        let task_id = request
            .task_id
            .as_ref()
            .ok_or_else(|| DaemonError::InvalidRequest("task_id required for cloud spawn".to_string()))?;
        let project_id = request
            .project_id
            .as_ref()
            .ok_or_else(|| DaemonError::InvalidRequest("project_id required for cloud spawn".to_string()))?;

        // Get Fly.io client
        let fly_client = self
            .fly_client
            .as_ref()
            .ok_or_else(|| DaemonError::InvalidRequest("Fly.io client not configured (FLY_API_TOKEN not set)".to_string()))?;

        // Get callback base URL
        let callback_base_url = self
            .callback_base_url
            .as_ref()
            .ok_or_else(|| DaemonError::InvalidRequest("Callback base URL not configured".to_string()))?;

        // Construct callback URL
        let callback_url = format!("{}/api/agents/callback", callback_base_url);

        // Spawn worker on Fly.io
        let machine = fly_client
            .spawn_worker(task_id, project_id, &callback_url)
            .await
            .map_err(|e| DaemonError::InvalidRequest(format!("Fly.io spawn failed: {}", e)))?;

        let now = Utc::now();

        // Create agent info with Fly machine ID
        let agent = AgentInfo {
            id: machine.id.clone(),
            name: request.name,
            status: AgentStatus::Running,
            created_at: now,
            updated_at: now,
            pid: None, // Cloud agents don't have local PIDs
            config: request.config,
        };

        self.agents.insert(machine.id.clone(), agent.clone());

        let response = AgentSpawnResponse {
            agent_id: machine.id,
            status: AgentStatus::Running,
            message: "Agent spawned on Fly.io successfully".to_string(),
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Handle agent.list RPC method
    pub async fn handle_agent_list(
        &self,
        _params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let agents: Vec<AgentInfo> = self.agents.iter().map(|r| r.clone()).collect();

        let response = AgentListResponse {
            count: agents.len(),
            agents,
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Handle agent.kill RPC method
    pub async fn handle_agent_kill(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: AgentKillRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

        let mut agent = self
            .agents
            .get_mut(&request.agent_id)
            .ok_or_else(|| DaemonError::AgentNotFound(request.agent_id.clone()))?;

        agent.status = AgentStatus::Terminated;
        agent.updated_at = Utc::now();

        let response = AgentKillResponse {
            agent_id: request.agent_id,
            status: AgentStatus::Terminated,
            message: "Agent terminated successfully".to_string(),
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Handle agent.logs RPC method
    pub async fn handle_agent_logs(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: AgentLogsRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

        // Verify agent exists (in memory)
        let _agent = self
            .agents
            .get(&request.agent_id)
            .ok_or_else(|| DaemonError::AgentNotFound(request.agent_id.clone()))?;

        // Fetch logs from runner if available
        let logs = if let Some(ref runner) = self.runner {
            // Try to get agent info which may contain recent logs
            let agent_uuid = Uuid::parse_str(&request.agent_id).map_err(|e| {
                DaemonError::InvalidRequest(format!("Invalid agent_id format: {}", e))
            })?;

            match runner.get_agent(&agent_uuid).await {
                Ok(Some(info)) => {
                    // Build log entries from agent status/info
                    let mut entries = Vec::new();
                    entries.push(LogEntry {
                        timestamp: chrono::DateTime::from_timestamp(
                            info.started_at
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64,
                            0,
                        )
                        .unwrap_or_else(Utc::now),
                        level: "info".to_string(),
                        message: format!("Agent started: {}", info.name),
                        context: Some(serde_json::json!({
                            "status": format!("{:?}", info.status),
                            "model_backend": info.model_backend,
                            "task": info.task
                        })),
                    });

                    // Add pause information if present
                    if let Some(paused_at) = info.paused_at {
                        entries.push(LogEntry {
                            timestamp: chrono::DateTime::from_timestamp(
                                paused_at
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs() as i64,
                                0,
                            )
                            .unwrap_or_else(Utc::now),
                            level: "info".to_string(),
                            message: format!(
                                "Agent paused (mode: {:?})",
                                info.pause_mode.unwrap_or(descartes_core::PauseMode::Cooperative)
                            ),
                            context: None,
                        });
                    }

                    entries
                }
                Ok(None) => {
                    // Agent not found in runner, use fallback
                    vec![LogEntry {
                        timestamp: Utc::now(),
                        level: "info".to_string(),
                        message: "Agent state unavailable".to_string(),
                        context: None,
                    }]
                }
                Err(e) => {
                    // Error fetching, use fallback with error info
                    vec![LogEntry {
                        timestamp: Utc::now(),
                        level: "warn".to_string(),
                        message: format!("Failed to fetch agent logs: {}", e),
                        context: None,
                    }]
                }
            }
        } else {
            // No runner, use placeholder logs
            vec![LogEntry {
                timestamp: Utc::now(),
                level: "info".to_string(),
                message: "Agent started (no runner attached)".to_string(),
                context: None,
            }]
        };

        // Apply limit and offset if specified
        let total = logs.len();
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit.unwrap_or(logs.len());
        let filtered_logs: Vec<LogEntry> = logs.into_iter().skip(offset).take(limit).collect();

        let response = AgentLogsResponse {
            agent_id: request.agent_id,
            logs: filtered_logs,
            total,
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Handle workflow.execute RPC method
    pub async fn handle_workflow_execute(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: WorkflowExecuteRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

        // Verify all agents exist
        for agent_id in &request.agents {
            self.agents
                .get(agent_id)
                .ok_or_else(|| DaemonError::AgentNotFound(agent_id.clone()))?;
        }

        let execution_id = Uuid::new_v4().to_string();

        let response = WorkflowExecuteResponse {
            execution_id,
            workflow_id: request.workflow_id,
            status: "running".to_string(),
            created_at: Utc::now(),
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Handle state.query RPC method
    pub async fn handle_state_query(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: StateQueryRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

        // If agent_id is specified, verify it exists
        if let Some(agent_id) = &request.agent_id {
            self.agents
                .get(agent_id)
                .ok_or_else(|| DaemonError::AgentNotFound(agent_id.clone()))?;
        }

        // Fetch state from state store if available
        let state = if let Some(ref state_store) = self.state_store {
            let store = state_store.read().await;

            if let Some(agent_id) = &request.agent_id {
                // Get state for specific agent
                match store.load_agent_state(agent_id).await {
                    Ok(Some(agent_state)) => {
                        let mut state_map = std::collections::HashMap::new();

                        // Convert agent state to key-value map
                        state_map.insert("name".to_string(), serde_json::json!(agent_state.name));
                        state_map
                            .insert("status".to_string(), serde_json::json!(agent_state.status));
                        state_map.insert(
                            "updated_at".to_string(),
                            serde_json::json!(agent_state.updated_at),
                        );
                        state_map.insert("metadata".to_string(), agent_state.metadata);

                        // If specific key requested, filter to just that key
                        if let Some(key) = &request.key {
                            if let Some(value) = state_map.get(key) {
                                let mut filtered = std::collections::HashMap::new();
                                filtered.insert(key.clone(), value.clone());
                                filtered
                            } else {
                                state_map
                            }
                        } else {
                            state_map
                        }
                    }
                    Ok(None) => {
                        // No state found for agent
                        let mut state_map = std::collections::HashMap::new();
                        state_map.insert(
                            "info".to_string(),
                            serde_json::json!("No state stored for agent"),
                        );
                        state_map
                    }
                    Err(e) => {
                        // Error fetching state
                        let mut state_map = std::collections::HashMap::new();
                        state_map.insert(
                            "error".to_string(),
                            serde_json::json!(format!("Failed to fetch state: {}", e)),
                        );
                        state_map
                    }
                }
            } else {
                // List all agents' states
                match store.list_agents().await {
                    Ok(agents) => {
                        let mut state_map = std::collections::HashMap::new();
                        for agent_state in agents {
                            state_map.insert(
                                agent_state.agent_id.clone(),
                                serde_json::json!({
                                    "name": agent_state.name,
                                    "status": agent_state.status,
                                    "updated_at": agent_state.updated_at
                                }),
                            );
                        }
                        state_map
                    }
                    Err(e) => {
                        let mut state_map = std::collections::HashMap::new();
                        state_map.insert(
                            "error".to_string(),
                            serde_json::json!(format!("Failed to list states: {}", e)),
                        );
                        state_map
                    }
                }
            }
        } else {
            // No state store, return empty
            std::collections::HashMap::new()
        };

        let response = StateQueryResponse {
            state,
            timestamp: Utc::now(),
        };

        serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))
    }

    /// Get agent by ID
    pub fn get_agent(&self, agent_id: &str) -> DaemonResult<AgentInfo> {
        self.agents
            .get(agent_id)
            .map(|r| r.clone())
            .ok_or_else(|| DaemonError::AgentNotFound(agent_id.to_string()))
    }

    /// Get all agents
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.iter().map(|r| r.clone()).collect()
    }

    /// Get the project store
    pub fn project_store(&self) -> Option<&Arc<ProjectStore>> {
        self.project_store.as_ref()
    }
}

impl Default for RpcHandlers {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REST Endpoint Handlers
// ============================================================================

/// Handle create project request
pub async fn handle_create_project(
    project_store: &ProjectStore,
    owner_id: &str,
    req: CreateProjectRequest,
) -> Result<CreateProjectResponse, RpcError> {
    let project = project_store
        .create(owner_id, req)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to create project: {}", e)))?;

    Ok(CreateProjectResponse { project })
}

/// Handle list projects request
pub async fn handle_list_projects(
    project_store: &ProjectStore,
    owner_id: &str,
) -> Result<Vec<Project>, RpcError> {
    project_store
        .list(owner_id)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to list projects: {}", e)))
}

/// Handle get project request
pub async fn handle_get_project(
    project_store: &ProjectStore,
    project_id: &str,
) -> Result<Project, RpcError> {
    project_store
        .get(project_id)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to get project: {}", e)))?
        .ok_or_else(|| RpcError::not_found("Project not found"))
}

/// Handle delete project request
pub async fn handle_delete_project(
    project_store: &ProjectStore,
    project_id: &str,
) -> Result<bool, RpcError> {
    project_store
        .delete(project_id)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to delete project: {}", e)))
}

/// Handle parse PRD request
pub async fn handle_parse_prd(
    project_store: &ProjectStore,
    project_id: &str,
) -> Result<Vec<Wave>, RpcError> {
    let project = handle_get_project(project_store, project_id).await?;

    let _prd_content = project.prd_content
        .ok_or_else(|| RpcError::bad_request("Project has no PRD content"))?;

    // For MVP, return mock waves - real implementation would use SCUD CLI
    // In production: parse PRD with SCUD and return actual waves
    Ok(vec![
        Wave {
            index: 0,
            tasks: vec!["Task 1".to_string(), "Task 2".to_string()],
            status: WaveStatus::Pending,
        },
        Wave {
            index: 1,
            tasks: vec!["Task 3".to_string()],
            status: WaveStatus::Pending,
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_agent_spawn() {
        let handlers = RpcHandlers::new();
        let auth = AuthContext::unauthenticated();

        let params = json!({
            "name": "test-agent",
            "agent_type": "basic",
            "config": {}
        });

        let result = handlers.handle_agent_spawn(params, auth).await;
        assert!(result.is_ok());

        let response: AgentSpawnResponse = serde_json::from_value(result.unwrap()).unwrap();
        assert_eq!(response.status, AgentStatus::Running);
    }

    #[tokio::test]
    async fn test_agent_list() {
        let handlers = RpcHandlers::new();
        let auth = AuthContext::unauthenticated();

        // Spawn an agent
        let spawn_params = json!({
            "name": "test-agent",
            "agent_type": "basic",
            "config": {}
        });
        let _ = handlers
            .handle_agent_spawn(spawn_params, auth.clone())
            .await;

        // List agents
        let result = handlers.handle_agent_list(json!({}), auth).await;
        assert!(result.is_ok());

        let response: AgentListResponse = serde_json::from_value(result.unwrap()).unwrap();
        assert_eq!(response.count, 1);
    }

    #[tokio::test]
    async fn test_agent_kill() {
        let handlers = RpcHandlers::new();
        let auth = AuthContext::unauthenticated();

        // Spawn an agent
        let spawn_params = json!({
            "name": "test-agent",
            "agent_type": "basic",
            "config": {}
        });
        let spawn_result = handlers
            .handle_agent_spawn(spawn_params, auth.clone())
            .await;
        let spawn_response: AgentSpawnResponse =
            serde_json::from_value(spawn_result.unwrap()).unwrap();

        // Kill the agent
        let kill_params = json!({
            "agent_id": spawn_response.agent_id,
            "force": false
        });
        let result = handlers.handle_agent_kill(kill_params, auth).await;
        assert!(result.is_ok());

        let response: AgentKillResponse = serde_json::from_value(result.unwrap()).unwrap();
        assert_eq!(response.status, AgentStatus::Terminated);
    }
}
