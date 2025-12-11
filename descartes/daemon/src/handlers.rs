/// RPC method handlers
use crate::auth::AuthContext;
use crate::errors::{DaemonError, DaemonResult};
use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// RPC handlers
pub struct RpcHandlers {
    /// In-memory agent storage (for demo; would be backed by actual agent runner)
    agents: Arc<DashMap<String, AgentInfo>>,
}

impl RpcHandlers {
    /// Create new handlers
    pub fn new() -> Self {
        RpcHandlers {
            agents: Arc::new(DashMap::new()),
        }
    }

    /// Handle agent.spawn RPC method
    pub async fn handle_agent_spawn(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: AgentSpawnRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

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

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
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

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
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

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
    }

    /// Handle agent.logs RPC method
    pub async fn handle_agent_logs(
        &self,
        params: Value,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let request: AgentLogsRequest = serde_json::from_value(params)
            .map_err(|e| DaemonError::InvalidRequest(format!("Invalid params: {}", e)))?;

        // Verify agent exists
        self.agents
            .get(&request.agent_id)
            .ok_or_else(|| DaemonError::AgentNotFound(request.agent_id.clone()))?;

        // TODO: Fetch actual logs from agent runner
        let logs = vec![LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: "Agent started".to_string(),
            context: None,
        }];

        let response = AgentLogsResponse {
            agent_id: request.agent_id,
            logs: logs.clone(),
            total: logs.len(),
        };

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
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

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
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

        // TODO: Fetch actual state from state store
        let state = std::collections::HashMap::new();

        let response = StateQueryResponse {
            state: state,
            timestamp: Utc::now(),
        };

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
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
}

impl Default for RpcHandlers {
    fn default() -> Self {
        Self::new()
    }
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
