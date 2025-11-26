//! Agent Monitoring System for RPC Integration (Phase 3:5.3)
//!
//! This module integrates the JSON stream parser with the RPC system to provide
//! real-time agent monitoring capabilities for swarm orchestration.
//!
//! # Features
//!
//! - Real-time agent status streaming via RPC
//! - Agent discovery and lifecycle tracking
//! - Status aggregation across multiple agents
//! - Integration with event bus for GUI updates
//! - Error handling and recovery
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  Agent Process  │
//! │  (stdout/JSON)  │
//! └────────┬────────┘
//!          │ NDJSON stream
//!          ▼
//! ┌─────────────────────┐
//! │ AgentStreamParser   │
//! │ (phase3:5.2)        │
//! └────────┬────────────┘
//!          │ Parsed events
//!          ▼
//! ┌─────────────────────┐
//! │ AgentMonitor        │
//! │ - Discovery         │
//! │ - Aggregation       │
//! │ - State tracking    │
//! └────────┬────────────┘
//!          │
//!     ┌────┴────┐
//!     │         │
//!     ▼         ▼
//! ┌──────┐  ┌──────────┐
//! │ RPC  │  │EventBus  │
//! │Stream│  │(phase3:3)│
//! └──────┘  └──────────┘
//! ```

use crate::events::{AgentEvent, AgentEventType, DescartesEvent, EventBus};
use chrono::Utc;
use descartes_core::{
    agent_state::{
        AgentError, AgentProgress, AgentRuntimeState, AgentStateCollection, AgentStatistics,
        AgentStatus, AgentStreamMessage, LifecycleEvent, OutputStream,
    },
    agent_stream_parser::{AgentStreamParser, ParserConfig, StreamHandler, StreamResult},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// CONSTANTS
// ============================================================================

/// How often to check for stale agents (agents that haven't updated in a while)
const STALE_CHECK_INTERVAL_SECS: u64 = 30;

/// Consider an agent stale if it hasn't updated in this many seconds
const AGENT_STALE_THRESHOLD_SECS: i64 = 120;

/// Maximum number of agents to track
const MAX_TRACKED_AGENTS: usize = 1000;

// ============================================================================
// AGENT MONITORING CONFIGURATION
// ============================================================================

/// Configuration for the agent monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMonitorConfig {
    /// Enable automatic agent discovery
    pub auto_discover: bool,

    /// Maximum number of agents to track
    pub max_agents: usize,

    /// Stale agent threshold in seconds
    pub stale_threshold_secs: i64,

    /// Stale check interval in seconds
    pub stale_check_interval_secs: u64,

    /// Enable event bus integration
    pub enable_event_bus: bool,

    /// Parser configuration
    pub parser_config: ParserConfig,
}

impl Default for AgentMonitorConfig {
    fn default() -> Self {
        Self {
            auto_discover: true,
            max_agents: MAX_TRACKED_AGENTS,
            stale_threshold_secs: AGENT_STALE_THRESHOLD_SECS,
            stale_check_interval_secs: STALE_CHECK_INTERVAL_SECS,
            enable_event_bus: true,
            parser_config: ParserConfig::default(),
        }
    }
}

// ============================================================================
// AGENT MONITOR
// ============================================================================

/// Main agent monitoring system
///
/// This struct integrates the stream parser with RPC and event bus systems
/// to provide comprehensive agent monitoring capabilities.
pub struct AgentMonitor {
    /// Configuration
    config: AgentMonitorConfig,

    /// Stream parser for processing agent updates
    parser: Arc<RwLock<AgentStreamParser>>,

    /// Event bus for publishing updates
    event_bus: Arc<EventBus>,

    /// Active agents indexed by ID
    agents: Arc<RwLock<HashMap<Uuid, AgentRuntimeState>>>,

    /// Monitoring statistics
    stats: Arc<RwLock<MonitorStats>>,
}

/// Monitoring statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitorStats {
    /// Total agents discovered
    pub total_discovered: u64,

    /// Total agents removed (terminated/stale)
    pub total_removed: u64,

    /// Total messages processed
    pub total_messages: u64,

    /// Total errors encountered
    pub total_errors: u64,

    /// Timestamp of last update
    pub last_update: Option<chrono::DateTime<Utc>>,
}

impl AgentMonitor {
    /// Create a new agent monitor
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self::with_config(AgentMonitorConfig::default(), event_bus)
    }

    /// Create a new agent monitor with custom configuration
    pub fn with_config(config: AgentMonitorConfig, event_bus: Arc<EventBus>) -> Self {
        let parser = AgentStreamParser::with_config(config.parser_config.clone());

        Self {
            config,
            parser: Arc::new(RwLock::new(parser)),
            event_bus,
            agents: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MonitorStats::default())),
        }
    }

    /// Start the monitoring system
    ///
    /// This spawns background tasks for:
    /// - Stale agent cleanup
    /// - Periodic statistics updates
    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let agents = Arc::clone(&self.agents);
        let stats = Arc::clone(&self.stats);
        let event_bus = Arc::clone(&self.event_bus);
        let stale_threshold = self.config.stale_threshold_secs;
        let check_interval = self.config.stale_check_interval_secs;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(check_interval));

            loop {
                ticker.tick().await;

                // Check for stale agents
                let now = Utc::now();
                let mut agents_lock = agents.write().await;
                let mut removed_count = 0;

                agents_lock.retain(|agent_id, agent| {
                    let age_secs = (now - agent.updated_at).num_seconds();
                    let is_stale = age_secs > stale_threshold;

                    if is_stale && !agent.status.is_terminal() {
                        warn!(
                            "Removing stale agent {} (last updated {} seconds ago)",
                            agent_id, age_secs
                        );
                        removed_count += 1;

                        // Publish termination event
                        let event = AgentEvent::failed(
                            agent_id.to_string(),
                            format!("Agent became stale ({}s without updates)", age_secs),
                        );
                        let _ = event_bus.publish(event);
                        false
                    } else {
                        true
                    }
                });

                if removed_count > 0 {
                    let mut stats_lock = stats.write().await;
                    stats_lock.total_removed += removed_count;
                    info!("Removed {} stale agents", removed_count);
                }
            }
        })
    }

    /// Register an agent handler that forwards events to the event bus
    pub async fn register_event_handler(&self) {
        let event_bus = Arc::clone(&self.event_bus);
        let stats = Arc::clone(&self.stats);

        let handler = EventBusHandler::new(event_bus, stats);
        self.parser.write().await.register_handler(handler);
    }

    /// Process a JSON line from an agent
    pub async fn process_message(&self, line: &str) -> StreamResult<()> {
        let mut parser = self.parser.write().await;
        parser.process_lines(vec![line])?;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_messages += 1;
        stats.last_update = Some(Utc::now());

        // Sync agents from parser
        drop(stats);
        self.sync_agents(&parser).await;

        Ok(())
    }

    /// Process an agent stream message directly
    pub async fn process_stream_message(&self, message: AgentStreamMessage) -> StreamResult<()> {
        let json = serde_json::to_string(&message)
            .map_err(|e| descartes_core::agent_stream_parser::StreamParseError::JsonError(e))?;
        self.process_message(&json).await
    }

    /// Sync agents from the parser to our local tracking
    async fn sync_agents(&self, parser: &AgentStreamParser) {
        let mut agents = self.agents.write().await;
        let parser_agents = parser.agents();

        for (agent_id, agent_state) in parser_agents {
            // Check if this is a new agent
            if !agents.contains_key(agent_id) {
                if self.config.auto_discover {
                    info!("Discovered new agent: {} ({})", agent_state.name, agent_id);

                    let mut stats = self.stats.write().await;
                    stats.total_discovered += 1;
                    drop(stats);

                    // Publish discovery event
                    let event = AgentEvent::spawned(
                        agent_id.to_string(),
                        serde_json::json!({
                            "name": agent_state.name,
                            "task": agent_state.task,
                            "model_backend": agent_state.model_backend,
                        }),
                    );
                    self.event_bus.publish(event).await;
                }
            }

            // Update our tracking
            agents.insert(*agent_id, agent_state.clone());
        }
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// List all active agents
    pub async fn list_agents(&self) -> Vec<AgentRuntimeState> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Get detailed status for a specific agent
    pub async fn get_agent_status(&self, agent_id: &Uuid) -> Option<AgentRuntimeState> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// Get aggregated statistics for all agents
    pub async fn get_statistics(&self) -> AgentStateCollection {
        let agents = self.list_agents().await;
        AgentStateCollection::new(agents)
    }

    /// Get monitoring statistics
    pub async fn get_monitor_stats(&self) -> MonitorStats {
        self.stats.read().await.clone()
    }

    /// Get agents by status
    pub async fn get_agents_by_status(&self, status: AgentStatus) -> Vec<AgentRuntimeState> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.status == status)
            .cloned()
            .collect()
    }

    /// Get health summary
    pub async fn get_health_summary(&self) -> HealthSummary {
        let agents = self.agents.read().await;
        let stats = self.stats.read().await;

        let total_agents = agents.len();
        let active_agents = agents.values().filter(|a| a.is_active()).count();
        let failed_agents = agents
            .values()
            .filter(|a| a.status == AgentStatus::Failed)
            .count();
        let completed_agents = agents
            .values()
            .filter(|a| a.status == AgentStatus::Completed)
            .count();

        // Calculate average execution time for completed agents
        let execution_times: Vec<i64> = agents
            .values()
            .filter_map(|a| a.execution_time().map(|d| d.num_seconds()))
            .collect();

        let avg_execution_time = if !execution_times.is_empty() {
            Some(execution_times.iter().sum::<i64>() as f64 / execution_times.len() as f64)
        } else {
            None
        };

        HealthSummary {
            total_agents,
            active_agents,
            failed_agents,
            completed_agents,
            total_discovered: stats.total_discovered,
            total_removed: stats.total_removed,
            total_messages: stats.total_messages,
            total_errors: stats.total_errors,
            avg_execution_time_secs: avg_execution_time,
            last_update: stats.last_update,
        }
    }

    /// Register a new agent manually
    pub async fn register_agent(&self, agent: AgentRuntimeState) {
        let agent_id = agent.agent_id;
        let mut agents = self.agents.write().await;

        if agents.len() >= self.config.max_agents {
            warn!(
                "Maximum agent limit ({}) reached, cannot register new agent",
                self.config.max_agents
            );
            return;
        }

        info!("Manually registering agent: {} ({})", agent.name, agent_id);
        agents.insert(agent_id, agent);

        let mut stats = self.stats.write().await;
        stats.total_discovered += 1;
    }

    /// Remove an agent from tracking
    pub async fn remove_agent(&self, agent_id: &Uuid) -> bool {
        let mut agents = self.agents.write().await;
        let removed = agents.remove(agent_id).is_some();

        if removed {
            info!("Removed agent from tracking: {}", agent_id);
            let mut stats = self.stats.write().await;
            stats.total_removed += 1;
        }

        removed
    }
}

// ============================================================================
// HEALTH SUMMARY
// ============================================================================

/// Summary of agent monitoring health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    /// Total number of tracked agents
    pub total_agents: usize,

    /// Number of active agents
    pub active_agents: usize,

    /// Number of failed agents
    pub failed_agents: usize,

    /// Number of completed agents
    pub completed_agents: usize,

    /// Total agents discovered since start
    pub total_discovered: u64,

    /// Total agents removed since start
    pub total_removed: u64,

    /// Total messages processed
    pub total_messages: u64,

    /// Total errors encountered
    pub total_errors: u64,

    /// Average execution time in seconds
    pub avg_execution_time_secs: Option<f64>,

    /// Last update timestamp
    pub last_update: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// EVENT BUS HANDLER
// ============================================================================

/// Stream handler that forwards events to the event bus
struct EventBusHandler {
    event_bus: Arc<EventBus>,
    stats: Arc<RwLock<MonitorStats>>,
}

impl EventBusHandler {
    fn new(event_bus: Arc<EventBus>, stats: Arc<RwLock<MonitorStats>>) -> Self {
        Self { event_bus, stats }
    }
}

impl StreamHandler for EventBusHandler {
    fn on_status_update(
        &mut self,
        agent_id: Uuid,
        status: AgentStatus,
        timestamp: chrono::DateTime<Utc>,
    ) {
        debug!("Agent {} status update: {}", agent_id, status);

        let event = AgentEvent::status_changed(agent_id.to_string(), status.to_string());
        let event_bus = Arc::clone(&self.event_bus);

        tokio::spawn(async move {
            event_bus.publish(event).await;
        });
    }

    fn on_thought_update(
        &mut self,
        agent_id: Uuid,
        thought: String,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        debug!("Agent {} thought: {}", agent_id, thought);

        let event = DescartesEvent::AgentEvent(crate::events::AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type: AgentEventType::Log,
            data: serde_json::json!({
                "type": "thought",
                "content": thought,
            }),
        });

        let event_bus = Arc::clone(&self.event_bus);
        tokio::spawn(async move {
            event_bus.publish(event).await;
        });
    }

    fn on_progress_update(
        &mut self,
        agent_id: Uuid,
        progress: AgentProgress,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        debug!("Agent {} progress: {:.1}%", agent_id, progress.percentage);

        let event = DescartesEvent::AgentEvent(crate::events::AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type: AgentEventType::Metric,
            data: serde_json::json!({
                "type": "progress",
                "percentage": progress.percentage,
                "current_step": progress.current_step,
                "total_steps": progress.total_steps,
                "message": progress.message,
            }),
        });

        let event_bus = Arc::clone(&self.event_bus);
        tokio::spawn(async move {
            event_bus.publish(event).await;
        });
    }

    fn on_output(
        &mut self,
        agent_id: Uuid,
        stream: OutputStream,
        content: String,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        debug!("Agent {} {:?}: {}", agent_id, stream, content);

        let event = DescartesEvent::AgentEvent(crate::events::AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type: AgentEventType::Log,
            data: serde_json::json!({
                "stream": match stream {
                    OutputStream::Stdout => "stdout",
                    OutputStream::Stderr => "stderr",
                },
                "content": content,
            }),
        });

        let event_bus = Arc::clone(&self.event_bus);
        tokio::spawn(async move {
            event_bus.publish(event).await;
        });
    }

    fn on_error(&mut self, agent_id: Uuid, error: AgentError, _timestamp: chrono::DateTime<Utc>) {
        error!("Agent {} error: {}", agent_id, error.message);

        let event = AgentEvent::failed(agent_id.to_string(), error.message);
        let event_bus = Arc::clone(&self.event_bus);
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            event_bus.publish(event).await;
            let mut stats_lock = stats.write().await;
            stats_lock.total_errors += 1;
        });
    }

    fn on_lifecycle(
        &mut self,
        agent_id: Uuid,
        event: LifecycleEvent,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        info!("Agent {} lifecycle: {:?}", agent_id, event);

        let event_type = match event {
            LifecycleEvent::Spawned => AgentEventType::Spawned,
            LifecycleEvent::Started => AgentEventType::Started,
            LifecycleEvent::Paused => AgentEventType::Paused,
            LifecycleEvent::Resumed => AgentEventType::Resumed,
            LifecycleEvent::Completed => AgentEventType::Completed,
            LifecycleEvent::Failed => AgentEventType::Failed,
            LifecycleEvent::Terminated => AgentEventType::Killed,
        };

        let descartes_event = DescartesEvent::AgentEvent(crate::events::AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type,
            data: serde_json::json!({
                "lifecycle_event": format!("{:?}", event),
            }),
        });

        let event_bus = Arc::clone(&self.event_bus);
        tokio::spawn(async move {
            event_bus.publish(descartes_event).await;
        });
    }

    fn on_heartbeat(&mut self, agent_id: Uuid, _timestamp: chrono::DateTime<Utc>) {
        // Heartbeats are too frequent for event bus, just log at trace level
        tracing::trace!("Agent {} heartbeat", agent_id);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_monitor() -> AgentMonitor {
        let event_bus = Arc::new(EventBus::new());
        AgentMonitor::new(event_bus)
    }

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = create_test_monitor();
        let agents = monitor.list_agents().await;
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let monitor = create_test_monitor();
        let agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            "test task".to_string(),
            "claude".to_string(),
        );

        let agent_id = agent.agent_id;
        monitor.register_agent(agent).await;

        let retrieved = monitor.get_agent_status(&agent_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-agent");
    }

    #[tokio::test]
    async fn test_process_status_message() {
        let monitor = create_test_monitor();
        monitor.register_event_handler().await;

        let agent_id = Uuid::new_v4();

        // First transition to Initializing
        let message1 = AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Initializing,
            timestamp: Utc::now(),
        };
        monitor.process_stream_message(message1).await.ok();

        // Then transition to Running
        let message2 = AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Running,
            timestamp: Utc::now(),
        };

        let result = monitor.process_stream_message(message2).await;
        assert!(result.is_ok());

        // Agent should be auto-discovered
        let agent = monitor.get_agent_status(&agent_id).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().status, AgentStatus::Running);
    }

    #[tokio::test]
    async fn test_health_summary() {
        let monitor = create_test_monitor();

        // Register some test agents
        let agent1 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent1".to_string(),
            "task1".to_string(),
            "claude".to_string(),
        );

        let mut agent2 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent2".to_string(),
            "task2".to_string(),
            "claude".to_string(),
        );
        agent2.transition_to(AgentStatus::Initializing, None).ok();
        agent2.transition_to(AgentStatus::Running, None).ok();

        monitor.register_agent(agent1).await;
        monitor.register_agent(agent2).await;

        let health = monitor.get_health_summary().await;
        assert_eq!(health.total_agents, 2);
        assert_eq!(health.active_agents, 1);
        assert_eq!(health.total_discovered, 2);
    }

    #[tokio::test]
    async fn test_remove_agent() {
        let monitor = create_test_monitor();
        let agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            "test task".to_string(),
            "claude".to_string(),
        );

        let agent_id = agent.agent_id;
        monitor.register_agent(agent).await;

        let removed = monitor.remove_agent(&agent_id).await;
        assert!(removed);

        let retrieved = monitor.get_agent_status(&agent_id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_agents_by_status() {
        let monitor = create_test_monitor();

        let mut agent1 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent1".to_string(),
            "task1".to_string(),
            "claude".to_string(),
        );
        agent1.transition_to(AgentStatus::Initializing, None).ok();
        agent1.transition_to(AgentStatus::Running, None).ok();

        let mut agent2 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent2".to_string(),
            "task2".to_string(),
            "claude".to_string(),
        );
        agent2.transition_to(AgentStatus::Initializing, None).ok();
        agent2.transition_to(AgentStatus::Running, None).ok();

        let agent3 = AgentRuntimeState::new(
            Uuid::new_v4(),
            "agent3".to_string(),
            "task3".to_string(),
            "claude".to_string(),
        );

        monitor.register_agent(agent1).await;
        monitor.register_agent(agent2).await;
        monitor.register_agent(agent3).await;

        let running_agents = monitor.get_agents_by_status(AgentStatus::Running).await;
        assert_eq!(running_agents.len(), 2);

        let idle_agents = monitor.get_agents_by_status(AgentStatus::Idle).await;
        assert_eq!(idle_agents.len(), 1);
    }
}
