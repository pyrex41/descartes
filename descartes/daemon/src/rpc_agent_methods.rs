//! RPC Agent Monitoring Methods (Phase 3:5.3)
//!
//! This module extends the RPC server with comprehensive agent monitoring capabilities,
//! integrating the AgentMonitor system with jsonrpsee RPC methods.
//!
//! # Features
//!
//! - `list_agents`: List all active agents
//! - `get_agent_status`: Get detailed status for a specific agent
//! - `get_agent_statistics`: Get aggregated statistics
//! - `subscribe_agent_updates`: Subscribe to agent update stream
//! - `get_monitoring_health`: Get monitoring system health

use crate::agent_monitor::{AgentMonitor, HealthSummary, MonitorStats};
use crate::errors::{DaemonError, DaemonResult};
use descartes_core::{
    agent_state::{AgentRuntimeState, AgentStateCollection, AgentStatus},
    AgentStreamMessage,
};
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

// ============================================================================
// RPC API TRAIT
// ============================================================================

/// RPC API trait for agent monitoring methods
#[rpc(server)]
pub trait AgentMonitoringRpc {
    /// List all active agents
    ///
    /// # Arguments
    /// * `filter` - Optional filter (by status)
    ///
    /// # Returns
    /// List of agent runtime states
    #[method(name = "list_agents")]
    async fn list_agents(
        &self,
        filter: Option<AgentStatusFilter>,
    ) -> Result<Vec<AgentRuntimeState>, ErrorObjectOwned>;

    /// Get detailed status for a specific agent
    ///
    /// # Arguments
    /// * `agent_id` - The UUID of the agent
    ///
    /// # Returns
    /// Agent runtime state, or error if not found
    #[method(name = "get_agent_status")]
    async fn get_agent_status(
        &self,
        agent_id: String,
    ) -> Result<AgentRuntimeState, ErrorObjectOwned>;

    /// Get aggregated statistics for all agents
    ///
    /// # Returns
    /// Collection of agent states with statistics
    #[method(name = "get_agent_statistics")]
    async fn get_agent_statistics(&self) -> Result<AgentStateCollection, ErrorObjectOwned>;

    /// Get monitoring system health summary
    ///
    /// # Returns
    /// Health summary with metrics
    #[method(name = "get_monitoring_health")]
    async fn get_monitoring_health(&self) -> Result<HealthSummary, ErrorObjectOwned>;

    /// Get monitoring system stats
    ///
    /// # Returns
    /// Monitoring statistics
    #[method(name = "get_monitor_stats")]
    async fn get_monitor_stats(&self) -> Result<MonitorStats, ErrorObjectOwned>;

    /// Send an agent stream message to the monitoring system
    ///
    /// This allows external systems to push agent updates into the monitoring system
    ///
    /// # Arguments
    /// * `message` - The agent stream message
    ///
    /// # Returns
    /// Success/failure
    #[method(name = "push_agent_update")]
    async fn push_agent_update(
        &self,
        message: AgentStreamMessage,
    ) -> Result<bool, ErrorObjectOwned>;

    /// Register a new agent with the monitoring system
    ///
    /// # Arguments
    /// * `agent` - The agent runtime state
    ///
    /// # Returns
    /// Success/failure
    #[method(name = "register_agent")]
    async fn register_agent(&self, agent: AgentRuntimeState) -> Result<bool, ErrorObjectOwned>;

    /// Remove an agent from the monitoring system
    ///
    /// # Arguments
    /// * `agent_id` - The UUID of the agent
    ///
    /// # Returns
    /// True if removed, false if not found
    #[method(name = "remove_agent")]
    async fn remove_agent(&self, agent_id: String) -> Result<bool, ErrorObjectOwned>;
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Filter for listing agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusFilter {
    /// Filter by status
    pub status: Option<AgentStatus>,

    /// Filter by model backend
    pub model_backend: Option<String>,

    /// Filter by active state
    pub active_only: Option<bool>,
}

// ============================================================================
// RPC SERVER IMPLEMENTATION
// ============================================================================

/// Implementation of agent monitoring RPC methods
pub struct AgentMonitoringRpcImpl {
    /// Agent monitoring system
    monitor: Arc<AgentMonitor>,
}

impl AgentMonitoringRpcImpl {
    /// Create a new RPC implementation
    pub fn new(monitor: Arc<AgentMonitor>) -> Self {
        Self { monitor }
    }
}

#[async_trait]
impl AgentMonitoringRpcServer for AgentMonitoringRpcImpl {
    async fn list_agents(
        &self,
        filter: Option<AgentStatusFilter>,
    ) -> Result<Vec<AgentRuntimeState>, ErrorObjectOwned> {
        debug!("RPC: list_agents with filter: {:?}", filter);

        let mut agents = self.monitor.list_agents().await;

        // Apply filters if provided
        if let Some(f) = filter {
            if let Some(status) = f.status {
                agents.retain(|a| a.status == status);
            }

            if let Some(backend) = f.model_backend {
                agents.retain(|a| a.model_backend == backend);
            }

            if let Some(true) = f.active_only {
                agents.retain(|a| a.is_active());
            }
        }

        info!("RPC: list_agents returned {} agents", agents.len());
        Ok(agents)
    }

    async fn get_agent_status(
        &self,
        agent_id: String,
    ) -> Result<AgentRuntimeState, ErrorObjectOwned> {
        debug!("RPC: get_agent_status for {}", agent_id);

        let uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            error!("Invalid agent ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid agent ID: {}", e), None::<()>)
        })?;

        let agent = self.monitor.get_agent_status(&uuid).await.ok_or_else(|| {
            error!("Agent not found: {}", agent_id);
            ErrorObjectOwned::owned(-32602, format!("Agent not found: {}", agent_id), None::<()>)
        })?;

        info!("RPC: get_agent_status found agent {}", agent.name);
        Ok(agent)
    }

    async fn get_agent_statistics(&self) -> Result<AgentStateCollection, ErrorObjectOwned> {
        debug!("RPC: get_agent_statistics");

        let statistics = self.monitor.get_statistics().await;

        info!(
            "RPC: get_agent_statistics returned stats for {} agents",
            statistics.total
        );
        Ok(statistics)
    }

    async fn get_monitoring_health(&self) -> Result<HealthSummary, ErrorObjectOwned> {
        debug!("RPC: get_monitoring_health");

        let health = self.monitor.get_health_summary().await;

        info!(
            "RPC: get_monitoring_health - {} total agents, {} active",
            health.total_agents, health.active_agents
        );
        Ok(health)
    }

    async fn get_monitor_stats(&self) -> Result<MonitorStats, ErrorObjectOwned> {
        debug!("RPC: get_monitor_stats");

        let stats = self.monitor.get_monitor_stats().await;

        info!("RPC: get_monitor_stats - {} total messages", stats.total_messages);
        Ok(stats)
    }

    async fn push_agent_update(
        &self,
        message: AgentStreamMessage,
    ) -> Result<bool, ErrorObjectOwned> {
        debug!("RPC: push_agent_update");

        self.monitor
            .process_stream_message(message)
            .await
            .map_err(|e| {
                error!("Failed to process agent update: {}", e);
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to process update: {}", e),
                    None::<()>,
                )
            })?;

        Ok(true)
    }

    async fn register_agent(&self, agent: AgentRuntimeState) -> Result<bool, ErrorObjectOwned> {
        debug!("RPC: register_agent {}", agent.name);

        self.monitor.register_agent(agent).await;

        info!("RPC: agent registered successfully");
        Ok(true)
    }

    async fn remove_agent(&self, agent_id: String) -> Result<bool, ErrorObjectOwned> {
        debug!("RPC: remove_agent {}", agent_id);

        let uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            error!("Invalid agent ID format: {}", e);
            ErrorObjectOwned::owned(-32602, format!("Invalid agent ID: {}", e), None::<()>)
        })?;

        let removed = self.monitor.remove_agent(&uuid).await;

        info!("RPC: remove_agent - removed: {}", removed);
        Ok(removed)
    }
}

impl Clone for AgentMonitoringRpcImpl {
    fn clone(&self) -> Self {
        Self {
            monitor: Arc::clone(&self.monitor),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;

    fn create_test_impl() -> AgentMonitoringRpcImpl {
        let event_bus = Arc::new(EventBus::new());
        let monitor = Arc::new(AgentMonitor::new(event_bus));
        AgentMonitoringRpcImpl::new(monitor)
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let rpc = create_test_impl();
        let agents = rpc.list_agents(None).await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_register_and_get_agent() {
        let rpc = create_test_impl();

        let agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            "test task".to_string(),
            "claude".to_string(),
        );

        let agent_id = agent.agent_id.to_string();

        // Register
        let result = rpc.register_agent(agent.clone()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Get status
        let retrieved = rpc.get_agent_status(agent_id.clone()).await;
        assert!(retrieved.is_ok());
        let retrieved_agent = retrieved.unwrap();
        assert_eq!(retrieved_agent.name, "test-agent");
    }

    #[tokio::test]
    async fn test_list_agents_with_filter() {
        let rpc = create_test_impl();

        // Register some agents
        let mut agent1 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent1".to_string(),
            "task1".to_string(),
            "claude".to_string(),
        );
        agent1.transition_to(AgentStatus::Running, None).ok();

        let agent2 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent2".to_string(),
            "task2".to_string(),
            "openai".to_string(),
        );

        rpc.register_agent(agent1).await.unwrap();
        rpc.register_agent(agent2).await.unwrap();

        // Filter by status
        let filter = AgentStatusFilter {
            status: Some(AgentStatus::Running),
            model_backend: None,
            active_only: None,
        };

        let agents = rpc.list_agents(Some(filter)).await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "agent1");

        // Filter by backend
        let filter = AgentStatusFilter {
            status: None,
            model_backend: Some("openai".to_string()),
            active_only: None,
        };

        let agents = rpc.list_agents(Some(filter)).await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "agent2");
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let rpc = create_test_impl();

        // Register some agents
        let agent1 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent1".to_string(),
            "task1".to_string(),
            "claude".to_string(),
        );

        rpc.register_agent(agent1).await.unwrap();

        let stats = rpc.get_agent_statistics().await.unwrap();
        assert_eq!(stats.total, 1);
    }

    #[tokio::test]
    async fn test_get_monitoring_health() {
        let rpc = create_test_impl();

        let health = rpc.get_monitoring_health().await.unwrap();
        assert_eq!(health.total_agents, 0);
    }

    #[tokio::test]
    async fn test_push_agent_update() {
        let rpc = create_test_impl();

        let agent_id = Uuid::new_v4();
        let message = AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Running,
            timestamp: chrono::Utc::now(),
        };

        let result = rpc.push_agent_update(message).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify agent was auto-created
        let agent = rpc.get_agent_status(agent_id.to_string()).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_remove_agent() {
        let rpc = create_test_impl();

        let agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            "test task".to_string(),
            "claude".to_string(),
        );

        let agent_id = agent.agent_id.to_string();
        rpc.register_agent(agent).await.unwrap();

        // Remove
        let removed = rpc.remove_agent(agent_id.clone()).await.unwrap();
        assert!(removed);

        // Verify removed
        let result = rpc.get_agent_status(agent_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_agent_id() {
        let rpc = create_test_impl();

        let result = rpc.get_agent_status("invalid-uuid".to_string()).await;
        assert!(result.is_err());
    }
}
