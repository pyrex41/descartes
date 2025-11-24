//! Agent Status Models for Phase 3 Interface & Swarms
//!
//! This module defines comprehensive data models and schemas to represent active agents,
//! their statuses, and 'Thinking' state, ensuring alignment with the expected JSON stream format.
//!
//! # Overview
//!
//! The agent status system provides:
//! - Comprehensive agent state tracking including "Thinking" state
//! - JSON stream format compatibility for real-time monitoring
//! - Progress tracking and metadata
//! - Status transition validation
//! - Timeline and history tracking
//!
//! # Status Transition Model
//!
//! ```text
//!                     ┌──────────────┐
//!                     │              │
//!                     │     Idle     │
//!                     │              │
//!                     └──────┬───────┘
//!                            │
//!                         spawn()
//!                            │
//!                            ▼
//!                     ┌──────────────┐
//!                     │              │
//!                     │ Initializing │
//!                     │              │
//!                     └──────┬───────┘
//!                            │
//!                       initialize()
//!                            │
//!           ┌────────────────┼────────────────┐
//!           │                │                │
//!           ▼                ▼                ▼
//!    ┌──────────┐     ┌──────────┐    ┌──────────┐
//!    │          │     │          │    │          │
//!    │ Running  │────▶│ Thinking │    │  Paused  │
//!    │          │     │          │    │          │
//!    └────┬─────┘     └────┬─────┘    └────┬─────┘
//!         │                │               │
//!         │                │               │
//!         └────────────────┼───────────────┘
//!                          │
//!              ┌───────────┼───────────┐
//!              │           │           │
//!              ▼           ▼           ▼
//!       ┌──────────┐ ┌──────────┐ ┌──────────┐
//!       │          │ │          │ │          │
//!       │Completed │ │  Failed  │ │Terminated│
//!       │          │ │          │ │          │
//!       └──────────┘ └──────────┘ └──────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

// ============================================================================
// AGENT STATUS ENUM
// ============================================================================

/// Comprehensive agent status enumeration with 'Thinking' state support
///
/// This enum represents all possible states an agent can be in during its lifecycle.
/// Each state has specific transition rules and implications for agent behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Agent has been created but not yet started
    Idle,

    /// Agent is initializing (loading context, setting up environment)
    Initializing,

    /// Agent is actively executing tasks
    Running,

    /// Agent is actively thinking/reasoning (visible to monitoring UI)
    /// This state is crucial for the Swarm Monitor visualization
    Thinking,

    /// Agent has been paused and can be resumed
    Paused,

    /// Agent has completed its task successfully
    Completed,

    /// Agent encountered an error and stopped
    Failed,

    /// Agent was externally terminated (killed)
    Terminated,
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Initializing => write!(f, "initializing"),
            AgentStatus::Running => write!(f, "running"),
            AgentStatus::Thinking => write!(f, "thinking"),
            AgentStatus::Paused => write!(f, "paused"),
            AgentStatus::Completed => write!(f, "completed"),
            AgentStatus::Failed => write!(f, "failed"),
            AgentStatus::Terminated => write!(f, "terminated"),
        }
    }
}

impl AgentStatus {
    /// Check if this status allows transitions to another status
    pub fn can_transition_to(&self, target: AgentStatus) -> bool {
        match (self, target) {
            // From Idle
            (AgentStatus::Idle, AgentStatus::Initializing) => true,
            (AgentStatus::Idle, AgentStatus::Terminated) => true,

            // From Initializing
            (AgentStatus::Initializing, AgentStatus::Running) => true,
            (AgentStatus::Initializing, AgentStatus::Thinking) => true,
            (AgentStatus::Initializing, AgentStatus::Failed) => true,
            (AgentStatus::Initializing, AgentStatus::Terminated) => true,

            // From Running
            (AgentStatus::Running, AgentStatus::Thinking) => true,
            (AgentStatus::Running, AgentStatus::Paused) => true,
            (AgentStatus::Running, AgentStatus::Completed) => true,
            (AgentStatus::Running, AgentStatus::Failed) => true,
            (AgentStatus::Running, AgentStatus::Terminated) => true,

            // From Thinking
            (AgentStatus::Thinking, AgentStatus::Running) => true,
            (AgentStatus::Thinking, AgentStatus::Paused) => true,
            (AgentStatus::Thinking, AgentStatus::Completed) => true,
            (AgentStatus::Thinking, AgentStatus::Failed) => true,
            (AgentStatus::Thinking, AgentStatus::Terminated) => true,

            // From Paused
            (AgentStatus::Paused, AgentStatus::Running) => true,
            (AgentStatus::Paused, AgentStatus::Thinking) => true,
            (AgentStatus::Paused, AgentStatus::Failed) => true,
            (AgentStatus::Paused, AgentStatus::Terminated) => true,

            // Terminal states - no transitions out
            (AgentStatus::Completed, _) => false,
            (AgentStatus::Failed, _) => false,
            (AgentStatus::Terminated, _) => false,

            // Self transitions are allowed
            (a, b) if a == &b => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Check if this is a terminal state (no further transitions)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Terminated
        )
    }

    /// Check if this is an active state (agent is doing work)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            AgentStatus::Running | AgentStatus::Thinking | AgentStatus::Initializing
        )
    }

    /// Get a human-readable description of this status
    pub fn description(&self) -> &'static str {
        match self {
            AgentStatus::Idle => "Agent is idle and ready to start",
            AgentStatus::Initializing => "Agent is initializing",
            AgentStatus::Running => "Agent is actively executing tasks",
            AgentStatus::Thinking => "Agent is thinking and reasoning",
            AgentStatus::Paused => "Agent is paused and can be resumed",
            AgentStatus::Completed => "Agent has completed successfully",
            AgentStatus::Failed => "Agent has failed with errors",
            AgentStatus::Terminated => "Agent was terminated externally",
        }
    }
}

// ============================================================================
// AGENT RUNTIME STATE MODEL
// ============================================================================

/// Comprehensive agent runtime state model with full tracking capabilities
///
/// This model represents the complete runtime state of an agent at any point in time,
/// including its status, current thought process, progress, and metadata.
/// This is distinct from the persistence-focused AgentState in state_store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeState {
    /// Unique identifier for this agent
    pub agent_id: Uuid,

    /// Human-readable name of the agent
    pub name: String,

    /// Current status of the agent
    pub status: AgentStatus,

    /// Current thought or reasoning (populated during 'Thinking' state)
    /// This is extracted from JSON stream output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_thought: Option<String>,

    /// Progress information (percentage, steps completed, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<AgentProgress>,

    /// When the agent was created
    pub created_at: DateTime<Utc>,

    /// When the agent last updated its status
    pub updated_at: DateTime<Utc>,

    /// When the agent started execution (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// When the agent completed or failed (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    /// Process ID (if running as a process)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,

    /// Task or goal assigned to this agent
    pub task: String,

    /// Model backend being used (e.g., "anthropic", "openai")
    pub model_backend: String,

    /// Additional metadata (tags, labels, custom fields)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Error information (if status is Failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AgentError>,

    /// Timeline of status transitions
    #[serde(default)]
    pub timeline: Vec<StatusTransition>,
}

impl AgentRuntimeState {
    /// Create a new agent runtime state
    pub fn new(agent_id: Uuid, name: String, task: String, model_backend: String) -> Self {
        let now = Utc::now();
        Self {
            agent_id,
            name,
            status: AgentStatus::Idle,
            current_thought: None,
            progress: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            pid: None,
            task,
            model_backend,
            metadata: HashMap::new(),
            error: None,
            timeline: vec![StatusTransition {
                from: None,
                to: AgentStatus::Idle,
                timestamp: now,
                reason: Some("Agent created".to_string()),
            }],
        }
    }

    /// Transition to a new status
    pub fn transition_to(&mut self, new_status: AgentStatus, reason: Option<String>) -> Result<(), String> {
        if !self.status.can_transition_to(new_status) {
            return Err(format!(
                "Invalid transition from {} to {}",
                self.status, new_status
            ));
        }

        let old_status = self.status;
        self.status = new_status;
        self.updated_at = Utc::now();

        // Update timestamps based on new status
        match new_status {
            AgentStatus::Running | AgentStatus::Initializing => {
                if self.started_at.is_none() {
                    self.started_at = Some(Utc::now());
                }
            }
            AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Terminated => {
                if self.completed_at.is_none() {
                    self.completed_at = Some(Utc::now());
                }
            }
            _ => {}
        }

        // Record transition in timeline
        self.timeline.push(StatusTransition {
            from: Some(old_status),
            to: new_status,
            timestamp: Utc::now(),
            reason,
        });

        Ok(())
    }

    /// Update the current thought (for Thinking state)
    pub fn update_thought(&mut self, thought: String) {
        self.current_thought = Some(thought);
        self.updated_at = Utc::now();
    }

    /// Clear the current thought
    pub fn clear_thought(&mut self) {
        self.current_thought = None;
        self.updated_at = Utc::now();
    }

    /// Update progress information
    pub fn update_progress(&mut self, progress: AgentProgress) {
        self.progress = Some(progress);
        self.updated_at = Utc::now();
    }

    /// Set error information
    pub fn set_error(&mut self, error: AgentError) {
        self.error = Some(error);
        self.updated_at = Utc::now();
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Get total execution time (if started)
    pub fn execution_time(&self) -> Option<chrono::Duration> {
        if let Some(started) = self.started_at {
            let end = self.completed_at.unwrap_or_else(Utc::now);
            Some(end - started)
        } else {
            None
        }
    }

    /// Check if agent is currently active
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }
}

// ============================================================================
// AGENT PROGRESS
// ============================================================================

/// Progress information for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProgress {
    /// Overall progress percentage (0-100)
    pub percentage: f32,

    /// Current step number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<u32>,

    /// Total number of steps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_steps: Option<u32>,

    /// Human-readable status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Additional progress details
    #[serde(default)]
    pub details: HashMap<String, serde_json::Value>,
}

impl AgentProgress {
    /// Create a new progress tracker
    pub fn new(percentage: f32) -> Self {
        Self {
            percentage: percentage.clamp(0.0, 100.0),
            current_step: None,
            total_steps: None,
            message: None,
            details: HashMap::new(),
        }
    }

    /// Create progress with steps
    pub fn with_steps(current: u32, total: u32) -> Self {
        let percentage = if total > 0 {
            (current as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Self {
            percentage,
            current_step: Some(current),
            total_steps: Some(total),
            message: None,
            details: HashMap::new(),
        }
    }
}

// ============================================================================
// AGENT ERROR
// ============================================================================

/// Error information for failed agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    /// Error code or type
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Stack trace or detailed error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// When the error occurred
    pub timestamp: DateTime<Utc>,

    /// Whether the error is recoverable
    pub recoverable: bool,
}

impl AgentError {
    /// Create a new agent error
    pub fn new(code: String, message: String) -> Self {
        Self {
            code,
            message,
            details: None,
            timestamp: Utc::now(),
            recoverable: false,
        }
    }

    /// Create a recoverable error
    pub fn recoverable(code: String, message: String) -> Self {
        Self {
            code,
            message,
            details: None,
            timestamp: Utc::now(),
            recoverable: true,
        }
    }
}

// ============================================================================
// STATUS TRANSITION
// ============================================================================

/// Represents a status transition in the agent timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransition {
    /// Previous status (None for initial state)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<AgentStatus>,

    /// New status
    pub to: AgentStatus,

    /// When the transition occurred
    pub timestamp: DateTime<Utc>,

    /// Reason for the transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ============================================================================
// JSON STREAM FORMAT MODELS
// ============================================================================

/// JSON stream message envelope for real-time agent monitoring
///
/// This format is designed to be compatible with JSON streaming protocols
/// used by LLM providers and monitoring tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentStreamMessage {
    /// Agent status update
    StatusUpdate {
        agent_id: Uuid,
        status: AgentStatus,
        timestamp: DateTime<Utc>,
    },

    /// Agent thought/reasoning update (for Thinking state)
    ThoughtUpdate {
        agent_id: Uuid,
        thought: String,
        timestamp: DateTime<Utc>,
    },

    /// Progress update
    ProgressUpdate {
        agent_id: Uuid,
        progress: AgentProgress,
        timestamp: DateTime<Utc>,
    },

    /// Output/log message
    Output {
        agent_id: Uuid,
        stream: OutputStream,
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Error message
    Error {
        agent_id: Uuid,
        error: AgentError,
        timestamp: DateTime<Utc>,
    },

    /// Agent lifecycle event
    Lifecycle {
        agent_id: Uuid,
        event: LifecycleEvent,
        timestamp: DateTime<Utc>,
    },

    /// Heartbeat/keepalive
    Heartbeat {
        agent_id: Uuid,
        timestamp: DateTime<Utc>,
    },
}

/// Output stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Lifecycle event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    Spawned,
    Started,
    Paused,
    Resumed,
    Completed,
    Failed,
    Terminated,
}

// ============================================================================
// AGENT RUNTIME STATE COLLECTION
// ============================================================================

/// Collection of agent runtime states for bulk operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateCollection {
    /// List of agent runtime states
    pub agents: Vec<AgentRuntimeState>,

    /// Total count
    pub total: usize,

    /// Timestamp of this snapshot
    pub timestamp: DateTime<Utc>,

    /// Aggregated statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<AgentStatistics>,
}

/// Statistics about a collection of agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    /// Count by status
    pub status_counts: HashMap<AgentStatus, usize>,

    /// Total active agents
    pub total_active: usize,

    /// Total completed agents
    pub total_completed: usize,

    /// Total failed agents
    pub total_failed: usize,

    /// Average execution time (in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_execution_time: Option<f64>,
}

impl AgentStateCollection {
    /// Create a new collection
    pub fn new(agents: Vec<AgentRuntimeState>) -> Self {
        let total = agents.len();
        let statistics = Some(Self::compute_statistics(&agents));

        Self {
            agents,
            total,
            timestamp: Utc::now(),
            statistics,
        }
    }

    /// Compute statistics for a collection of agents
    fn compute_statistics(agents: &[AgentRuntimeState]) -> AgentStatistics {
        let mut status_counts: HashMap<AgentStatus, usize> = HashMap::new();
        let mut total_active = 0;
        let mut total_completed = 0;
        let mut total_failed = 0;
        let mut execution_times = Vec::new();

        for agent in agents {
            *status_counts.entry(agent.status).or_insert(0) += 1;

            if agent.is_active() {
                total_active += 1;
            }

            match agent.status {
                AgentStatus::Completed => total_completed += 1,
                AgentStatus::Failed => total_failed += 1,
                _ => {}
            }

            if let Some(exec_time) = agent.execution_time() {
                execution_times.push(exec_time.num_seconds() as f64);
            }
        }

        let avg_execution_time = if !execution_times.is_empty() {
            Some(execution_times.iter().sum::<f64>() / execution_times.len() as f64)
        } else {
            None
        };

        AgentStatistics {
            status_counts,
            total_active,
            total_completed,
            total_failed,
            avg_execution_time,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_transitions() {
        // Valid transitions
        assert!(AgentStatus::Idle.can_transition_to(AgentStatus::Initializing));
        assert!(AgentStatus::Initializing.can_transition_to(AgentStatus::Running));
        assert!(AgentStatus::Running.can_transition_to(AgentStatus::Thinking));
        assert!(AgentStatus::Thinking.can_transition_to(AgentStatus::Running));
        assert!(AgentStatus::Running.can_transition_to(AgentStatus::Paused));
        assert!(AgentStatus::Paused.can_transition_to(AgentStatus::Running));
        assert!(AgentStatus::Running.can_transition_to(AgentStatus::Completed));

        // Invalid transitions
        assert!(!AgentStatus::Completed.can_transition_to(AgentStatus::Running));
        assert!(!AgentStatus::Failed.can_transition_to(AgentStatus::Running));
        assert!(!AgentStatus::Idle.can_transition_to(AgentStatus::Running));
    }

    #[test]
    fn test_agent_status_terminal() {
        assert!(AgentStatus::Completed.is_terminal());
        assert!(AgentStatus::Failed.is_terminal());
        assert!(AgentStatus::Terminated.is_terminal());
        assert!(!AgentStatus::Running.is_terminal());
        assert!(!AgentStatus::Thinking.is_terminal());
    }

    #[test]
    fn test_agent_status_active() {
        assert!(AgentStatus::Running.is_active());
        assert!(AgentStatus::Thinking.is_active());
        assert!(AgentStatus::Initializing.is_active());
        assert!(!AgentStatus::Idle.is_active());
        assert!(!AgentStatus::Paused.is_active());
        assert!(!AgentStatus::Completed.is_active());
    }

    #[test]
    fn test_agent_runtime_state_creation() {
        let agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            "test task".to_string(),
            "anthropic".to_string(),
        );

        assert_eq!(agent.status, AgentStatus::Idle);
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.timeline.len(), 1);
    }

    #[test]
    fn test_agent_runtime_state_transitions() {
        let mut agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            "test task".to_string(),
            "anthropic".to_string(),
        );

        // Valid transition
        assert!(agent
            .transition_to(AgentStatus::Initializing, Some("Starting".to_string()))
            .is_ok());
        assert_eq!(agent.status, AgentStatus::Initializing);
        assert_eq!(agent.timeline.len(), 2);

        // Invalid transition
        assert!(agent
            .transition_to(AgentStatus::Completed, None)
            .is_err());
    }

    #[test]
    fn test_agent_progress() {
        let progress = AgentProgress::with_steps(5, 10);
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.current_step, Some(5));
        assert_eq!(progress.total_steps, Some(10));
    }

    #[test]
    fn test_agent_stream_message_serialization() {
        let msg = AgentStreamMessage::StatusUpdate {
            agent_id: Uuid::new_v4(),
            status: AgentStatus::Thinking,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("status_update"));
        assert!(json.contains("thinking"));
    }

    #[test]
    fn test_agent_runtime_state_collection() {
        let agents = vec![
            AgentRuntimeState::new(
                Uuid::new_v4(),
                "agent1".to_string(),
                "task1".to_string(),
                "backend1".to_string(),
            ),
            AgentRuntimeState::new(
                Uuid::new_v4(),
                "agent2".to_string(),
                "task2".to_string(),
                "backend2".to_string(),
            ),
        ];

        let collection = AgentStateCollection::new(agents);
        assert_eq!(collection.total, 2);
        assert!(collection.statistics.is_some());
    }
}
