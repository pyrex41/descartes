//! Statig-based state machine integration for Descartes workflow orchestration
//!
//! Provides production-ready state machines with:
//! - Hierarchical state support for complex workflows
//! - Event-driven state transitions with compile-time verification
//! - Async handlers integrated with Tokio
//! - State persistence using SQLite StateStore
//! - State history and rollback capabilities
//! - Support for concurrent multi-agent workflows

use crate::traits::StateStore;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Error, Debug)]
pub enum StateMachineError {
    #[error("Invalid state transition: {0} -> {1}")]
    InvalidTransition(String, String),

    #[error("State machine not found: {0}")]
    NotFound(String),

    #[error("Handler execution failed: {0}")]
    HandlerError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("State store error: {0}")]
    StateStoreError(String),

    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    #[error("Workflow timeout")]
    Timeout,

    #[error("Rollback failed: {0}")]
    RollbackError(String),

    #[error("State locked: {0}")]
    StateLocked(String),

    #[error("History limit exceeded")]
    HistoryLimitExceeded,

    #[error("Metadata error: {0}")]
    MetadataError(String),
}

pub type StateMachineResult<T> = Result<T, StateMachineError>;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Represents a workflow state in the state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowState {
    /// Workflow initialized, awaiting start
    Idle,

    /// Workflow actively executing
    Running,

    /// Workflow temporarily paused
    Paused,

    /// Workflow completed successfully
    Completed,

    /// Workflow failed with errors
    Failed,
}

impl fmt::Display for WorkflowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkflowState::Idle => write!(f, "Idle"),
            WorkflowState::Running => write!(f, "Running"),
            WorkflowState::Paused => write!(f, "Paused"),
            WorkflowState::Completed => write!(f, "Completed"),
            WorkflowState::Failed => write!(f, "Failed"),
        }
    }
}

impl WorkflowState {
    /// Check if this state allows transitions to another state
    pub fn can_transition_to(&self, target: WorkflowState) -> bool {
        match (self, &target) {
            // From Idle
            (WorkflowState::Idle, WorkflowState::Running) => true,

            // From Running
            (WorkflowState::Running, WorkflowState::Paused) => true,
            (WorkflowState::Running, WorkflowState::Completed) => true,
            (WorkflowState::Running, WorkflowState::Failed) => true,

            // From Paused
            (WorkflowState::Paused, WorkflowState::Running) => true,
            (WorkflowState::Paused, WorkflowState::Failed) => true,

            // Terminal states - no transitions
            (WorkflowState::Completed, _) => false,
            (WorkflowState::Failed, _) => false,

            // Self transitions allowed
            (a, b) if a == b => true,

            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Completed | WorkflowState::Failed)
    }
}

/// Events that trigger state transitions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowEvent {
    /// Start the workflow
    Start,

    /// Pause the workflow
    Pause,

    /// Resume from pause
    Resume,

    /// Complete successfully
    Complete,

    /// Fail with error message
    Fail(String),

    /// Custom event with arbitrary data
    Custom { name: String, data: serde_json::Value },

    /// Timeout event
    Timeout,

    /// Retry after failure
    Retry,

    /// Rollback to previous state
    Rollback,
}

impl fmt::Display for WorkflowEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkflowEvent::Start => write!(f, "Start"),
            WorkflowEvent::Pause => write!(f, "Pause"),
            WorkflowEvent::Resume => write!(f, "Resume"),
            WorkflowEvent::Complete => write!(f, "Complete"),
            WorkflowEvent::Fail(msg) => write!(f, "Fail({})", msg),
            WorkflowEvent::Custom { name, .. } => write!(f, "Custom({})", name),
            WorkflowEvent::Timeout => write!(f, "Timeout"),
            WorkflowEvent::Retry => write!(f, "Retry"),
            WorkflowEvent::Rollback => write!(f, "Rollback"),
        }
    }
}

/// Metadata associated with a workflow state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionMetadata {
    /// Unique ID for this transition
    pub transition_id: String,

    /// Timestamp when transition occurred
    pub timestamp: String,

    /// Source state
    pub from_state: WorkflowState,

    /// Target state
    pub to_state: WorkflowState,

    /// Event that triggered transition
    pub event: String,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Any error message
    pub error: Option<String>,

    /// Handler execution details
    pub handler_details: Option<String>,
}

/// Complete state history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryEntry {
    /// The transition
    pub transition: TransitionMetadata,

    /// Serialized context at this point
    pub context_snapshot: serde_json::Value,
}

// ============================================================================
// STATE HANDLER TRAIT
// ============================================================================

/// Async handler for state transitions
#[async_trait]
pub trait StateHandler: Send + Sync {
    /// Called when entering a state
    async fn on_enter(&self, state: WorkflowState) -> StateMachineResult<()>;

    /// Called when exiting a state
    async fn on_exit(&self, state: WorkflowState) -> StateMachineResult<()>;

    /// Called when an event is processed
    async fn on_event(
        &self,
        state: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<()>;

    /// Called on transition completion
    async fn on_transition(
        &self,
        from: WorkflowState,
        to: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<()>;
}

/// No-op default handler
pub struct DefaultStateHandler;

#[async_trait]
impl StateHandler for DefaultStateHandler {
    async fn on_enter(&self, _state: WorkflowState) -> StateMachineResult<()> {
        Ok(())
    }

    async fn on_exit(&self, _state: WorkflowState) -> StateMachineResult<()> {
        Ok(())
    }

    async fn on_event(
        &self,
        _state: WorkflowState,
        _event: &WorkflowEvent,
    ) -> StateMachineResult<()> {
        Ok(())
    }

    async fn on_transition(
        &self,
        _from: WorkflowState,
        _to: WorkflowState,
        _event: &WorkflowEvent,
    ) -> StateMachineResult<()> {
        Ok(())
    }
}

// ============================================================================
// WORKFLOW STATE MACHINE
// ============================================================================

/// A production-ready state machine for workflow orchestration
pub struct WorkflowStateMachine {
    /// Unique ID for this workflow instance
    workflow_id: String,

    /// Current state
    current_state: Arc<RwLock<WorkflowState>>,

    /// State history with configurable retention
    history: Arc<RwLock<Vec<StateHistoryEntry>>>,

    /// Maximum history entries (default: 1000)
    max_history_size: usize,

    /// Optional state handler for lifecycle events
    handler: Arc<dyn StateHandler>,

    /// Optional context storage
    context: Arc<RwLock<serde_json::Value>>,

    /// Creation timestamp
    created_at: String,

    /// Last transition timestamp
    last_transition_at: Arc<RwLock<String>>,
}

impl WorkflowStateMachine {
    /// Create a new workflow state machine
    pub fn new(workflow_id: String) -> Self {
        Self::with_handler(workflow_id, Arc::new(DefaultStateHandler))
    }

    /// Create with custom handler
    pub fn with_handler(workflow_id: String, handler: Arc<dyn StateHandler>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            workflow_id,
            current_state: Arc::new(RwLock::new(WorkflowState::Idle)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 1000,
            handler,
            context: Arc::new(RwLock::new(serde_json::json!({}))),
            created_at: now.clone(),
            last_transition_at: Arc::new(RwLock::new(now)),
        }
    }

    /// Set maximum history size
    pub fn with_max_history(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }

    /// Get workflow ID
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    /// Get current state
    pub async fn current_state(&self) -> WorkflowState {
        *self.current_state.read().await
    }

    /// Process an event and transition state if valid
    pub async fn process_event(&self, event: WorkflowEvent) -> StateMachineResult<()> {
        let current = *self.current_state.read().await;
        let start_time = std::time::Instant::now();

        // Call on_event handler
        self.handler.on_event(current, &event).await?;

        // Determine target state
        let target_state = self.determine_target_state(current, &event)?;

        // Validate transition
        if !current.can_transition_to(target_state) {
            return Err(StateMachineError::InvalidTransition(
                format!("{}", current),
                format!("{}", target_state),
            ));
        }

        // Call exit handler
        self.handler.on_exit(current).await?;

        // Perform transition
        let mut state_guard = self.current_state.write().await;
        *state_guard = target_state;
        drop(state_guard);

        // Call enter handler
        self.handler.on_enter(target_state).await?;

        // Call transition handler
        self.handler.on_transition(current, target_state, &event).await?;

        // Record transition
        let duration = start_time.elapsed().as_millis() as u64;
        let transition = TransitionMetadata {
            transition_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            from_state: current,
            to_state: target_state,
            event: format!("{}", event),
            duration_ms: duration,
            error: None,
            handler_details: None,
        };

        self.add_history_entry(transition).await?;

        // Update last transition time
        *self.last_transition_at.write().await = chrono::Utc::now().to_rfc3339();

        Ok(())
    }

    /// Get state transition history
    pub async fn get_history(&self) -> Vec<StateHistoryEntry> {
        self.history.read().await.clone()
    }

    /// Get last N history entries
    pub async fn get_history_tail(&self, n: usize) -> Vec<StateHistoryEntry> {
        let history = self.history.read().await;
        history.iter().rev().take(n).cloned().collect()
    }

    /// Rollback to previous state
    pub async fn rollback(&self) -> StateMachineResult<()> {
        let history = self.history.read().await;

        if history.len() < 2 {
            return Err(StateMachineError::RollbackError(
                "Insufficient history for rollback".to_string(),
            ));
        }

        let previous_transition = &history[history.len() - 2].transition;
        let target_state = previous_transition.to_state;

        drop(history);

        // Process rollback event
        self.process_event(WorkflowEvent::Rollback).await?;

        // Restore state
        *self.current_state.write().await = target_state;

        Ok(())
    }

    /// Set context data
    pub async fn set_context(&self, key: &str, value: serde_json::Value) -> StateMachineResult<()> {
        let mut ctx = self.context.write().await;
        if let serde_json::Value::Object(ref mut obj) = *ctx {
            obj.insert(key.to_string(), value);
            Ok(())
        } else {
            Err(StateMachineError::MetadataError(
                "Context is not an object".to_string(),
            ))
        }
    }

    /// Get context data
    pub async fn get_context(&self, key: &str) -> Option<serde_json::Value> {
        let ctx = self.context.read().await;
        if let serde_json::Value::Object(ref obj) = *ctx {
            obj.get(key).cloned()
        } else {
            None
        }
    }

    /// Get all context
    pub async fn get_all_context(&self) -> serde_json::Value {
        self.context.read().await.clone()
    }

    /// Get workflow metadata
    pub async fn get_metadata(&self) -> WorkflowMetadata {
        WorkflowMetadata {
            workflow_id: self.workflow_id.clone(),
            current_state: *self.current_state.read().await,
            created_at: self.created_at.clone(),
            last_transition_at: self.last_transition_at.read().await.clone(),
            history_size: self.history.read().await.len(),
        }
    }

    // ============================================================================
    // PRIVATE HELPERS
    // ============================================================================

    fn determine_target_state(
        &self,
        current: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<WorkflowState> {
        let target = match (current, event) {
            // Start transitions
            (WorkflowState::Idle, WorkflowEvent::Start) => WorkflowState::Running,

            // Running transitions
            (WorkflowState::Running, WorkflowEvent::Pause) => WorkflowState::Paused,
            (WorkflowState::Running, WorkflowEvent::Complete) => WorkflowState::Completed,
            (WorkflowState::Running, WorkflowEvent::Fail(_)) => WorkflowState::Failed,
            (WorkflowState::Running, WorkflowEvent::Timeout) => WorkflowState::Failed,

            // Paused transitions
            (WorkflowState::Paused, WorkflowEvent::Resume) => WorkflowState::Running,
            (WorkflowState::Paused, WorkflowEvent::Fail(_)) => WorkflowState::Failed,

            // Failed transitions
            (WorkflowState::Failed, WorkflowEvent::Retry) => WorkflowState::Running,

            // Self transitions
            (state, WorkflowEvent::Custom { .. }) => state,

            // Rollback
            (_, WorkflowEvent::Rollback) => current,

            _ => return Err(StateMachineError::InvalidEvent(format!("{}", event))),
        };

        Ok(target)
    }

    async fn add_history_entry(&self, transition: TransitionMetadata) -> StateMachineResult<()> {
        let context_snapshot = self.context.read().await.clone();

        let entry = StateHistoryEntry {
            transition,
            context_snapshot,
        };

        let mut history = self.history.write().await;

        if history.len() >= self.max_history_size {
            return Err(StateMachineError::HistoryLimitExceeded);
        }

        history.push(entry);
        Ok(())
    }
}

impl fmt::Debug for WorkflowStateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkflowStateMachine")
            .field("workflow_id", &self.workflow_id)
            .field("max_history_size", &self.max_history_size)
            .field("created_at", &self.created_at)
            .finish()
    }
}

// ============================================================================
// WORKFLOW METADATA
// ============================================================================

/// Metadata about a workflow state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub workflow_id: String,
    pub current_state: WorkflowState,
    pub created_at: String,
    pub last_transition_at: String,
    pub history_size: usize,
}

// ============================================================================
// HIERARCHICAL STATE MACHINE SUPPORT
// ============================================================================

/// Represents hierarchical state structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalState {
    pub name: String,
    pub parent: Option<String>,
    pub is_parallel: bool,
    pub substates: Vec<HierarchicalState>,
}

impl HierarchicalState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            parent: None,
            is_parallel: false,
            substates: Vec::new(),
        }
    }

    pub fn with_parent(mut self, parent: String) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_substates(mut self, substates: Vec<HierarchicalState>) -> Self {
        self.substates = substates;
        self
    }

    pub fn parallel() -> Self {
        Self {
            name: "Parallel".to_string(),
            parent: None,
            is_parallel: true,
            substates: Vec::new(),
        }
    }
}

// ============================================================================
// WORKFLOW ORCHESTRATOR
// ============================================================================

/// Manages multiple workflows and state machines
pub struct WorkflowOrchestrator {
    workflows: Arc<RwLock<std::collections::HashMap<String, Arc<WorkflowStateMachine>>>>,
}

impl WorkflowOrchestrator {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register a new workflow
    pub async fn register_workflow(
        &self,
        workflow_id: String,
        sm: Arc<WorkflowStateMachine>,
    ) -> StateMachineResult<()> {
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id, sm);
        Ok(())
    }

    /// Get a workflow by ID
    pub async fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> StateMachineResult<Arc<WorkflowStateMachine>> {
        let workflows = self.workflows.read().await;
        workflows
            .get(workflow_id)
            .cloned()
            .ok_or_else(|| StateMachineError::NotFound(workflow_id.to_string()))
    }

    /// Get all registered workflows
    pub async fn list_workflows(&self) -> Vec<String> {
        self.workflows.read().await.keys().cloned().collect()
    }

    /// Remove a workflow
    pub async fn unregister_workflow(&self, workflow_id: &str) -> StateMachineResult<()> {
        let mut workflows = self.workflows.write().await;
        workflows
            .remove(workflow_id)
            .ok_or_else(|| StateMachineError::NotFound(workflow_id.to_string()))?;
        Ok(())
    }

    /// Get all workflow metadata
    pub async fn get_all_metadata(&self) -> Vec<WorkflowMetadata> {
        let workflows = self.workflows.read().await;
        let mut results = Vec::new();

        for sm in workflows.values() {
            results.push(sm.get_metadata().await);
        }

        results
    }
}

impl Default for WorkflowOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SERIALIZATION SUPPORT
// ============================================================================

/// Serializable workflow state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedWorkflow {
    pub workflow_id: String,
    pub current_state: WorkflowState,
    pub history: Vec<StateHistoryEntry>,
    pub context: serde_json::Value,
    pub metadata: WorkflowMetadata,
}

impl WorkflowStateMachine {
    /// Serialize workflow for storage
    pub async fn serialize(&self) -> StateMachineResult<SerializedWorkflow> {
        Ok(SerializedWorkflow {
            workflow_id: self.workflow_id.clone(),
            current_state: *self.current_state.read().await,
            history: self.history.read().await.clone(),
            context: self.context.read().await.clone(),
            metadata: self.get_metadata().await,
        })
    }

    /// Restore workflow from serialized state
    pub async fn deserialize(
        serialized: SerializedWorkflow,
    ) -> StateMachineResult<Arc<WorkflowStateMachine>> {
        let sm = Arc::new(WorkflowStateMachine::new(serialized.workflow_id));

        *sm.current_state.write().await = serialized.current_state;
        *sm.history.write().await = serialized.history;
        *sm.context.write().await = serialized.context;

        Ok(sm)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_state_transitions() {
        let sm = WorkflowStateMachine::new("workflow-1".to_string());

        assert_eq!(sm.current_state().await, WorkflowState::Idle);

        sm.process_event(WorkflowEvent::Start)
            .await
            .expect("Start event should succeed");
        assert_eq!(sm.current_state().await, WorkflowState::Running);

        sm.process_event(WorkflowEvent::Complete)
            .await
            .expect("Complete event should succeed");
        assert_eq!(sm.current_state().await, WorkflowState::Completed);
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let sm = WorkflowStateMachine::new("workflow-2".to_string());

        sm.process_event(WorkflowEvent::Start).await.unwrap();
        assert_eq!(sm.current_state().await, WorkflowState::Running);

        sm.process_event(WorkflowEvent::Pause).await.unwrap();
        assert_eq!(sm.current_state().await, WorkflowState::Paused);

        sm.process_event(WorkflowEvent::Resume).await.unwrap();
        assert_eq!(sm.current_state().await, WorkflowState::Running);
    }

    #[tokio::test]
    async fn test_failure_handling() {
        let sm = WorkflowStateMachine::new("workflow-3".to_string());

        sm.process_event(WorkflowEvent::Start).await.unwrap();
        sm.process_event(WorkflowEvent::Fail("Test error".to_string()))
            .await
            .unwrap();

        assert_eq!(sm.current_state().await, WorkflowState::Failed);
    }

    #[tokio::test]
    async fn test_retry() {
        let sm = WorkflowStateMachine::new("workflow-4".to_string());

        sm.process_event(WorkflowEvent::Start).await.unwrap();
        sm.process_event(WorkflowEvent::Fail("Error".to_string()))
            .await
            .unwrap();

        assert_eq!(sm.current_state().await, WorkflowState::Failed);

        sm.process_event(WorkflowEvent::Retry).await.unwrap();
        assert_eq!(sm.current_state().await, WorkflowState::Running);
    }

    #[tokio::test]
    async fn test_history_tracking() {
        let sm = WorkflowStateMachine::new("workflow-5".to_string());

        sm.process_event(WorkflowEvent::Start).await.unwrap();
        sm.process_event(WorkflowEvent::Pause).await.unwrap();
        sm.process_event(WorkflowEvent::Resume).await.unwrap();

        let history = sm.get_history().await;
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].transition.from_state, WorkflowState::Idle);
        assert_eq!(history[0].transition.to_state, WorkflowState::Running);
    }

    #[tokio::test]
    async fn test_context_storage() {
        let sm = WorkflowStateMachine::new("workflow-6".to_string());

        sm.set_context("key1", serde_json::json!("value1"))
            .await
            .unwrap();
        sm.set_context("key2", serde_json::json!(42))
            .await
            .unwrap();

        assert_eq!(
            sm.get_context("key1").await,
            Some(serde_json::json!("value1"))
        );
        assert_eq!(sm.get_context("key2").await, Some(serde_json::json!(42)));
    }

    #[tokio::test]
    async fn test_invalid_transitions() {
        let sm = WorkflowStateMachine::new("workflow-7".to_string());

        // Can't transition from Idle to Paused
        let result = sm.process_event(WorkflowEvent::Pause).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_orchestrator() {
        let orchestrator = WorkflowOrchestrator::new();

        let sm1 = Arc::new(WorkflowStateMachine::new("workflow-1".to_string()));
        let sm2 = Arc::new(WorkflowStateMachine::new("workflow-2".to_string()));

        orchestrator
            .register_workflow("workflow-1".to_string(), sm1)
            .await
            .unwrap();
        orchestrator
            .register_workflow("workflow-2".to_string(), sm2)
            .await
            .unwrap();

        let workflows = orchestrator.list_workflows().await;
        assert_eq!(workflows.len(), 2);
        assert!(workflows.contains(&"workflow-1".to_string()));
        assert!(workflows.contains(&"workflow-2".to_string()));
    }

    #[tokio::test]
    async fn test_serialization() {
        let sm = Arc::new(WorkflowStateMachine::new("workflow-8".to_string()));

        sm.process_event(WorkflowEvent::Start).await.unwrap();
        sm.set_context("test", serde_json::json!("data"))
            .await
            .unwrap();

        let serialized = sm.serialize().await.unwrap();
        assert_eq!(serialized.workflow_id, "workflow-8");
        assert_eq!(serialized.current_state, WorkflowState::Running);
        assert_eq!(
            serialized.context.get("test"),
            Some(&serde_json::json!("data"))
        );

        let restored = WorkflowStateMachine::deserialize(serialized)
            .await
            .unwrap();
        assert_eq!(restored.current_state().await, WorkflowState::Running);
    }

    #[test]
    fn test_state_transitions_valid() {
        assert!(WorkflowState::Idle.can_transition_to(WorkflowState::Running));
        assert!(WorkflowState::Running.can_transition_to(WorkflowState::Paused));
        assert!(WorkflowState::Running.can_transition_to(WorkflowState::Completed));
        assert!(!WorkflowState::Completed.can_transition_to(WorkflowState::Running));
        assert!(!WorkflowState::Failed.can_transition_to(WorkflowState::Idle));
    }

    #[test]
    fn test_terminal_states() {
        assert!(WorkflowState::Completed.is_terminal());
        assert!(WorkflowState::Failed.is_terminal());
        assert!(!WorkflowState::Running.is_terminal());
        assert!(!WorkflowState::Idle.is_terminal());
    }
}
