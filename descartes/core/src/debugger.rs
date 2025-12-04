//! Debugger State Models for Descartes Agent Orchestration
//!
//! This module provides data structures and models for debugging agent execution,
//! including state tracking, breakpoints, stepping, and context inspection.
//!
//! Features:
//! - Debug mode control (enabled/disabled)
//! - Execution state management (running/paused/stepping)
//! - Thought and context snapshots for inspection
//! - Breakpoint management with conditional support
//! - Step-by-step execution control
//! - Call stack and variable inspection
//! - History navigation and replay
//! - Integration with agent state machine

use crate::state_machine::WorkflowState;
use crate::thoughts::ThoughtMetadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Error, Debug)]
pub enum DebuggerError {
    #[error("Debugger not enabled")]
    NotEnabled,

    #[error("Invalid debugger state transition: {0} -> {1}")]
    InvalidStateTransition(String, String),

    #[error("Breakpoint not found: {0}")]
    BreakpointNotFound(Uuid),

    #[error("Invalid breakpoint location: {0}")]
    InvalidBreakpointLocation(String),

    #[error("Cannot step while running")]
    CannotStepWhileRunning,

    #[error("Cannot resume while not paused")]
    CannotResumeWhileNotPaused,

    #[error("History index out of bounds: {0}")]
    HistoryIndexOutOfBounds(usize),

    #[error("No execution history available")]
    NoHistoryAvailable,

    #[error("Condition evaluation failed: {0}")]
    ConditionEvaluationFailed(String),

    #[error("Debugger command failed: {0}")]
    CommandFailed(String),
}

pub type DebuggerResult<T> = Result<T, DebuggerError>;

// ============================================================================
// EXECUTION STATE
// ============================================================================

/// Represents the current execution state of the debugger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    /// Agent is executing normally without interruption
    Running,

    /// Agent execution is paused, awaiting debugger commands
    Paused,

    /// Single-stepping through execution (step into calls)
    SteppingInto,

    /// Stepping over function/thought calls
    SteppingOver,

    /// Stepping out of current context to parent
    SteppingOut,

    /// Continuing execution until next breakpoint
    Continuing,
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionState::Running => write!(f, "Running"),
            ExecutionState::Paused => write!(f, "Paused"),
            ExecutionState::SteppingInto => write!(f, "Stepping Into"),
            ExecutionState::SteppingOver => write!(f, "Stepping Over"),
            ExecutionState::SteppingOut => write!(f, "Stepping Out"),
            ExecutionState::Continuing => write!(f, "Continuing"),
        }
    }
}

impl ExecutionState {
    /// Check if execution is actively running (not paused)
    pub fn is_running(&self) -> bool {
        !matches!(self, ExecutionState::Paused)
    }

    /// Check if execution is paused and awaiting input
    pub fn is_paused(&self) -> bool {
        matches!(self, ExecutionState::Paused)
    }

    /// Check if in a stepping mode
    pub fn is_stepping(&self) -> bool {
        matches!(
            self,
            ExecutionState::SteppingInto
                | ExecutionState::SteppingOver
                | ExecutionState::SteppingOut
        )
    }
}

// ============================================================================
// THOUGHT SNAPSHOT
// ============================================================================

/// A snapshot of agent thought state for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSnapshot {
    /// Unique identifier for the thought
    pub thought_id: String,

    /// Thought content/description
    pub content: String,

    /// When this thought was captured
    pub timestamp: String,

    /// Tags associated with the thought
    pub tags: Vec<String>,

    /// Additional metadata about the thought
    pub metadata: serde_json::Value,

    /// Agent ID that generated this thought
    pub agent_id: Option<Uuid>,

    /// Line/step number where thought occurred
    pub step_number: u64,
}

impl ThoughtSnapshot {
    /// Create a new thought snapshot
    pub fn new(
        thought_id: String,
        content: String,
        step_number: u64,
        agent_id: Option<Uuid>,
    ) -> Self {
        Self {
            thought_id,
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags: Vec::new(),
            metadata: serde_json::json!({}),
            agent_id,
            step_number,
        }
    }

    /// Create from ThoughtMetadata
    pub fn from_thought_metadata(thought: &ThoughtMetadata, step_number: u64) -> Self {
        Self {
            thought_id: thought.id.clone(),
            content: thought.content.clone(),
            timestamp: thought.created_at.clone(),
            tags: thought.tags.clone(),
            metadata: serde_json::json!({
                "title": thought.title,
                "project_id": thought.project_id,
            }),
            agent_id: thought
                .agent_id
                .as_ref()
                .and_then(|id| Uuid::parse_str(id).ok()),
            step_number,
        }
    }

    /// Get a summary of the thought (first 100 chars)
    pub fn summary(&self) -> String {
        if self.content.len() <= 100 {
            self.content.clone()
        } else {
            format!("{}...", &self.content[..97])
        }
    }
}

// ============================================================================
// CALL FRAME
// ============================================================================

/// Represents a single frame in the call stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    /// Unique identifier for this frame
    pub frame_id: Uuid,

    /// Name/description of the function/thought/workflow
    pub name: String,

    /// Current workflow state at this frame
    pub workflow_state: WorkflowState,

    /// Local variables in this frame's scope
    pub local_variables: HashMap<String, serde_json::Value>,

    /// Step number when this frame was entered
    pub entry_step: u64,

    /// Parent frame ID (if this is a nested call)
    pub parent_frame_id: Option<Uuid>,
}

impl CallFrame {
    /// Create a new call frame
    pub fn new(
        name: String,
        workflow_state: WorkflowState,
        entry_step: u64,
        parent_frame_id: Option<Uuid>,
    ) -> Self {
        Self {
            frame_id: Uuid::new_v4(),
            name,
            workflow_state,
            local_variables: HashMap::new(),
            entry_step,
            parent_frame_id,
        }
    }

    /// Set a local variable in this frame
    pub fn set_variable(&mut self, name: String, value: serde_json::Value) {
        self.local_variables.insert(name, value);
    }

    /// Get a local variable from this frame
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.local_variables.get(name)
    }
}

// ============================================================================
// DEBUG CONTEXT
// ============================================================================

/// Current execution context for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugContext {
    /// Agent being debugged
    pub agent_id: Uuid,

    /// Current workflow state
    pub workflow_state: WorkflowState,

    /// Current call stack depth
    pub stack_depth: usize,

    /// Local variables in current scope
    pub local_variables: HashMap<String, serde_json::Value>,

    /// Complete call stack (innermost frame first)
    pub call_stack: Vec<CallFrame>,

    /// Last executed thought
    pub last_thought: Option<ThoughtSnapshot>,

    /// Current line/step being executed
    pub current_step: u64,

    /// Additional context metadata
    pub metadata: serde_json::Value,
}

impl DebugContext {
    /// Create a new debug context
    pub fn new(agent_id: Uuid, workflow_state: WorkflowState) -> Self {
        Self {
            agent_id,
            workflow_state,
            stack_depth: 0,
            local_variables: HashMap::new(),
            call_stack: Vec::new(),
            last_thought: None,
            current_step: 0,
            metadata: serde_json::json!({}),
        }
    }

    /// Push a new call frame onto the stack
    pub fn push_frame(&mut self, frame: CallFrame) {
        self.call_stack.push(frame);
        self.stack_depth = self.call_stack.len();
    }

    /// Pop the top call frame from the stack
    pub fn pop_frame(&mut self) -> Option<CallFrame> {
        let frame = self.call_stack.pop();
        self.stack_depth = self.call_stack.len();
        frame
    }

    /// Get the current (top) call frame
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.call_stack.last()
    }

    /// Get the current (top) call frame mutably
    pub fn current_frame_mut(&mut self) -> Option<&mut CallFrame> {
        self.call_stack.last_mut()
    }

    /// Update the current thought
    pub fn update_thought(&mut self, thought: ThoughtSnapshot) {
        self.last_thought = Some(thought);
    }

    /// Increment step counter
    pub fn increment_step(&mut self) {
        self.current_step += 1;
    }

    /// Set a variable in the current scope
    pub fn set_variable(&mut self, name: String, value: serde_json::Value) {
        self.local_variables.insert(name, value);
    }

    /// Get the full call stack as a formatted string
    pub fn format_stack_trace(&self) -> String {
        let mut trace = String::from("Call Stack:\n");
        for (i, frame) in self.call_stack.iter().rev().enumerate() {
            trace.push_str(&format!(
                "  #{}: {} (state: {}, step: {})\n",
                i, frame.name, frame.workflow_state, frame.entry_step
            ));
        }
        trace
    }
}

// ============================================================================
// BREAKPOINT LOCATION
// ============================================================================

/// Specifies where a breakpoint is set
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BreakpointLocation {
    /// Break when entering a specific workflow state
    WorkflowState { state: WorkflowState },

    /// Break when a specific thought is created
    ThoughtId { thought_id: String },

    /// Break at a specific step count
    StepCount { step: u64 },

    /// Break when a specific agent is active
    AgentId { agent_id: Uuid },

    /// Break on any state transition
    AnyTransition,

    /// Break when call stack depth reaches a threshold
    StackDepth { depth: usize },
}

impl fmt::Display for BreakpointLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakpointLocation::WorkflowState { state } => {
                write!(f, "Workflow State: {}", state)
            }
            BreakpointLocation::ThoughtId { thought_id } => {
                write!(f, "Thought ID: {}", thought_id)
            }
            BreakpointLocation::StepCount { step } => {
                write!(f, "Step: {}", step)
            }
            BreakpointLocation::AgentId { agent_id } => {
                write!(f, "Agent: {}", agent_id)
            }
            BreakpointLocation::AnyTransition => {
                write!(f, "Any Transition")
            }
            BreakpointLocation::StackDepth { depth } => {
                write!(f, "Stack Depth: {}", depth)
            }
        }
    }
}

// ============================================================================
// BREAKPOINT
// ============================================================================

/// Represents a debugger breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Unique breakpoint identifier
    pub id: Uuid,

    /// Where the breakpoint is set
    pub location: BreakpointLocation,

    /// Optional condition that must be true to trigger
    pub condition: Option<String>,

    /// Whether this breakpoint is currently enabled
    pub enabled: bool,

    /// Hit count for this breakpoint
    pub hit_count: u64,

    /// Human-readable description
    pub description: Option<String>,

    /// When the breakpoint was created
    pub created_at: String,
}

impl Breakpoint {
    /// Create a new breakpoint
    pub fn new(location: BreakpointLocation) -> Self {
        Self {
            id: Uuid::new_v4(),
            location,
            condition: None,
            enabled: true,
            hit_count: 0,
            description: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a breakpoint with a condition
    pub fn with_condition(location: BreakpointLocation, condition: String) -> Self {
        let mut bp = Self::new(location);
        bp.condition = Some(condition);
        bp
    }

    /// Create a breakpoint with description
    pub fn with_description(location: BreakpointLocation, description: String) -> Self {
        let mut bp = Self::new(location);
        bp.description = Some(description);
        bp
    }

    /// Check if this breakpoint should trigger for the given context
    pub fn should_trigger(&mut self, context: &DebugContext) -> bool {
        if !self.enabled {
            return false;
        }

        // Check location match
        let location_match = match &self.location {
            BreakpointLocation::WorkflowState { state } => &context.workflow_state == state,
            BreakpointLocation::ThoughtId { thought_id } => context
                .last_thought
                .as_ref()
                .map(|t| &t.thought_id == thought_id)
                .unwrap_or(false),
            BreakpointLocation::StepCount { step } => context.current_step == *step,
            BreakpointLocation::AgentId { agent_id } => &context.agent_id == agent_id,
            BreakpointLocation::AnyTransition => true,
            BreakpointLocation::StackDepth { depth } => context.stack_depth == *depth,
        };

        if !location_match {
            return false;
        }

        // TODO: Evaluate condition if present
        // For now, conditions are not evaluated
        if self.condition.is_some() {
            // Would need an expression evaluator here
        }

        // Increment hit count and trigger
        self.hit_count += 1;
        true
    }

    /// Enable the breakpoint
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the breakpoint
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Toggle enabled state
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Reset hit count
    pub fn reset_hit_count(&mut self) {
        self.hit_count = 0;
    }
}

// ============================================================================
// DEBUGGER STATE
// ============================================================================

/// Main debugger state structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerState {
    /// Whether debug mode is enabled
    pub debug_mode: bool,

    /// Current execution state
    pub execution_state: ExecutionState,

    /// Current thought being inspected
    pub current_thought: Option<ThoughtSnapshot>,

    /// Current execution context
    pub current_context: DebugContext,

    /// All registered breakpoints
    pub breakpoints: Vec<Breakpoint>,

    /// Total step count since debugging started
    pub step_count: u64,

    /// Current position in execution history (for replay)
    pub history_index: Option<usize>,

    /// Execution history snapshots
    pub history: Vec<DebugSnapshot>,

    /// Maximum history size (default: 1000)
    pub max_history_size: usize,

    /// When debugging was started
    pub started_at: Option<String>,

    /// Statistics about this debug session
    pub statistics: DebugStatistics,
}

impl DebuggerState {
    /// Create a new debugger state
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            debug_mode: false,
            execution_state: ExecutionState::Running,
            current_thought: None,
            current_context: DebugContext::new(agent_id, WorkflowState::Idle),
            breakpoints: Vec::new(),
            step_count: 0,
            history_index: None,
            history: Vec::new(),
            max_history_size: 1000,
            started_at: None,
            statistics: DebugStatistics::default(),
        }
    }

    /// Enable debug mode
    pub fn enable(&mut self) {
        self.debug_mode = true;
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
        self.statistics.sessions_started += 1;
    }

    /// Disable debug mode
    pub fn disable(&mut self) {
        self.debug_mode = false;
        self.execution_state = ExecutionState::Running;
    }

    /// Check if debugger is enabled
    pub fn is_enabled(&self) -> bool {
        self.debug_mode
    }

    /// Add a breakpoint
    pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) -> Uuid {
        let id = breakpoint.id;
        self.breakpoints.push(breakpoint);
        self.statistics.breakpoints_set += 1;
        id
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: &Uuid) -> DebuggerResult<()> {
        let original_len = self.breakpoints.len();
        self.breakpoints.retain(|bp| &bp.id != id);

        if self.breakpoints.len() == original_len {
            Err(DebuggerError::BreakpointNotFound(*id))
        } else {
            Ok(())
        }
    }

    /// Enable/disable a breakpoint
    pub fn toggle_breakpoint(&mut self, id: &Uuid) -> DebuggerResult<()> {
        self.breakpoints
            .iter_mut()
            .find(|bp| &bp.id == id)
            .map(|bp| bp.toggle())
            .ok_or(DebuggerError::BreakpointNotFound(*id))
    }

    /// Get all breakpoints
    pub fn get_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    /// Check if any breakpoint should trigger
    pub fn check_breakpoints(&mut self) -> Option<&Breakpoint> {
        let mut triggered_id = None;
        for bp in &mut self.breakpoints {
            if bp.should_trigger(&self.current_context) {
                self.statistics.breakpoints_hit += 1;
                triggered_id = Some(bp.id.clone());
                break;
            }
        }
        // Return immutable reference after mutable iteration completes
        triggered_id.and_then(|id| self.breakpoints.iter().find(|b| b.id == id))
    }

    /// Execute a step and capture state
    pub fn step(&mut self) -> DebuggerResult<()> {
        if !self.debug_mode {
            return Err(DebuggerError::NotEnabled);
        }

        self.step_count += 1;
        self.current_context.increment_step();
        self.statistics.total_steps += 1;

        // Capture snapshot
        self.capture_snapshot();

        Ok(())
    }

    /// Capture current state as a snapshot
    fn capture_snapshot(&mut self) {
        if self.history.len() >= self.max_history_size {
            // Remove oldest entry
            self.history.remove(0);
        }

        let snapshot = DebugSnapshot {
            step_number: self.step_count,
            timestamp: chrono::Utc::now().to_rfc3339(),
            execution_state: self.execution_state,
            thought: self.current_thought.clone(),
            context: self.current_context.clone(),
            workflow_state: self.current_context.workflow_state,
        };

        self.history.push(snapshot);
    }

    /// Navigate to a specific point in history
    pub fn goto_history(&mut self, index: usize) -> DebuggerResult<()> {
        if index >= self.history.len() {
            return Err(DebuggerError::HistoryIndexOutOfBounds(index));
        }

        self.history_index = Some(index);
        let snapshot = &self.history[index];

        // Restore state from snapshot
        self.execution_state = snapshot.execution_state;
        self.current_thought = snapshot.thought.clone();
        self.current_context = snapshot.context.clone();

        Ok(())
    }

    /// Get current history snapshot
    pub fn current_snapshot(&self) -> Option<&DebugSnapshot> {
        if let Some(index) = self.history_index {
            self.history.get(index)
        } else {
            self.history.last()
        }
    }

    /// Get all history
    pub fn get_history(&self) -> &[DebugSnapshot] {
        &self.history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_index = None;
    }

    /// Update statistics
    pub fn update_statistics(&mut self, event: DebugEvent) {
        match event {
            DebugEvent::Paused => self.statistics.pauses += 1,
            DebugEvent::Resumed => self.statistics.resumes += 1,
            DebugEvent::Stepped => self.statistics.total_steps += 1,
            DebugEvent::BreakpointHit(_) => self.statistics.breakpoints_hit += 1,
        }
    }
}

// ============================================================================
// DEBUG SNAPSHOT
// ============================================================================

/// A snapshot of execution state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSnapshot {
    /// Step number when snapshot was taken
    pub step_number: u64,

    /// Timestamp of snapshot
    pub timestamp: String,

    /// Execution state at this point
    pub execution_state: ExecutionState,

    /// Thought at this point
    pub thought: Option<ThoughtSnapshot>,

    /// Full context at this point
    pub context: DebugContext,

    /// Workflow state at this point
    pub workflow_state: WorkflowState,
}

// ============================================================================
// DEBUG COMMANDS
// ============================================================================

/// Commands for controlling the debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum DebugCommand {
    /// Enable debug mode
    Enable,

    /// Disable debug mode
    Disable,

    /// Pause execution
    Pause,

    /// Resume execution
    Resume,

    /// Single step (step into)
    Step,

    /// Step over (don't enter nested calls)
    StepOver,

    /// Step into (enter nested calls)
    StepInto,

    /// Step out (exit current frame)
    StepOut,

    /// Continue until next breakpoint
    Continue,

    /// Set a breakpoint
    SetBreakpoint {
        location: BreakpointLocation,
        condition: Option<String>,
    },

    /// Remove a breakpoint
    RemoveBreakpoint { id: Uuid },

    /// Toggle breakpoint enabled state
    ToggleBreakpoint { id: Uuid },

    /// List all breakpoints
    ListBreakpoints,

    /// Inspect current context
    InspectContext,

    /// Show call stack
    ShowStack,

    /// Evaluate an expression in current context
    Evaluate { expression: String },

    /// Navigate to history index
    GotoHistory { index: usize },

    /// Show execution history
    ShowHistory,

    /// Clear history
    ClearHistory,

    /// Get debugger statistics
    GetStatistics,
}

// ============================================================================
// DEBUG EVENTS
// ============================================================================

/// Events emitted by the debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DebugEvent {
    /// Execution was paused
    Paused,

    /// Execution was resumed
    Resumed,

    /// A step was executed
    Stepped,

    /// A breakpoint was hit
    BreakpointHit(Uuid),
}

// ============================================================================
// DEBUG STATISTICS
// ============================================================================

/// Statistics about a debug session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugStatistics {
    /// Total debug sessions started
    pub sessions_started: u64,

    /// Total steps executed
    pub total_steps: u64,

    /// Number of times paused
    pub pauses: u64,

    /// Number of times resumed
    pub resumes: u64,

    /// Total breakpoints set
    pub breakpoints_set: u64,

    /// Total breakpoints hit
    pub breakpoints_hit: u64,
}

// ============================================================================
// INTEGRATION WITH AGENT STATE
// ============================================================================

/// Extension trait to add debugger state to agent info
pub trait DebuggerStateExt {
    /// Get debugger state if available
    fn debugger_state(&self) -> Option<&DebuggerState>;

    /// Get mutable debugger state if available
    fn debugger_state_mut(&mut self) -> Option<&mut DebuggerState>;

    /// Check if debugging is enabled
    fn is_debugging(&self) -> bool {
        self.debugger_state()
            .map(|d| d.is_enabled())
            .unwrap_or(false)
    }
}

// ============================================================================
// SERIALIZATION HELPERS
// ============================================================================

impl DebuggerState {
    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Save to file
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load from file
    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// ============================================================================
// CORE DEBUGGER LOGIC
// ============================================================================

/// Debugger controller for managing agent execution and debugging operations
pub struct Debugger {
    /// The current debugger state
    state: DebuggerState,

    /// Callback for when a breakpoint is hit
    on_breakpoint: Option<Box<dyn Fn(&Breakpoint, &DebugContext) + Send + Sync>>,

    /// Callback for when execution is paused
    on_pause: Option<Box<dyn Fn(&DebugContext) + Send + Sync>>,

    /// Callback for each step
    on_step: Option<Box<dyn Fn(&DebugSnapshot) + Send + Sync>>,
}

impl Debugger {
    /// Create a new debugger for an agent
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            state: DebuggerState::new(agent_id),
            on_breakpoint: None,
            on_pause: None,
            on_step: None,
        }
    }

    /// Get the current debugger state
    pub fn state(&self) -> &DebuggerState {
        &self.state
    }

    /// Get mutable debugger state
    pub fn state_mut(&mut self) -> &mut DebuggerState {
        &mut self.state
    }

    /// Set breakpoint hit callback
    pub fn on_breakpoint<F>(&mut self, callback: F)
    where
        F: Fn(&Breakpoint, &DebugContext) + Send + Sync + 'static,
    {
        self.on_breakpoint = Some(Box::new(callback));
    }

    /// Set pause callback
    pub fn on_pause<F>(&mut self, callback: F)
    where
        F: Fn(&DebugContext) + Send + Sync + 'static,
    {
        self.on_pause = Some(Box::new(callback));
    }

    /// Set step callback
    pub fn on_step<F>(&mut self, callback: F)
    where
        F: Fn(&DebugSnapshot) + Send + Sync + 'static,
    {
        self.on_step = Some(Box::new(callback));
    }

    // ========================================================================
    // CONTROL FUNCTIONS
    // ========================================================================

    /// Pause agent execution
    pub fn pause_agent(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        if self.state.execution_state == ExecutionState::Paused {
            return Ok(()); // Already paused
        }

        self.state.execution_state = ExecutionState::Paused;
        self.state.update_statistics(DebugEvent::Paused);

        // Trigger pause callback
        if let Some(ref callback) = self.on_pause {
            callback(&self.state.current_context);
        }

        Ok(())
    }

    /// Resume agent execution
    pub fn resume_agent(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        if !self.state.execution_state.is_paused() {
            return Err(DebuggerError::CannotResumeWhileNotPaused);
        }

        self.state.execution_state = ExecutionState::Running;
        self.state.update_statistics(DebugEvent::Resumed);

        Ok(())
    }

    /// Execute one step (generic step)
    pub fn step_agent(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        self.state.execution_state = ExecutionState::SteppingInto;
        self.state.step()?;
        self.state.update_statistics(DebugEvent::Stepped);

        // Trigger step callback
        if let Some(ref callback) = self.on_step {
            if let Some(snapshot) = self.state.current_snapshot() {
                callback(snapshot);
            }
        }

        // Pause after step
        self.state.execution_state = ExecutionState::Paused;

        Ok(())
    }

    /// Step over - execute without entering nested calls
    pub fn step_over(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        let current_depth = self.state.current_context.stack_depth;
        self.state.execution_state = ExecutionState::SteppingOver;

        // Execute steps until we return to the same stack depth
        loop {
            self.state.step()?;

            // If we're at or above the original depth, stop
            if self.state.current_context.stack_depth <= current_depth {
                break;
            }

            // Check for breakpoints
            if let Some(bp) = self.state.check_breakpoints() {
                let bp_clone = bp.clone();
                let _ = bp; // Release the borrow
                self.handle_breakpoint_hit(&bp_clone)?;
                break;
            }
        }

        self.state.execution_state = ExecutionState::Paused;
        Ok(())
    }

    /// Step into - enter nested calls
    pub fn step_into(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        self.state.execution_state = ExecutionState::SteppingInto;
        self.state.step()?;
        self.state.update_statistics(DebugEvent::Stepped);

        // Trigger step callback
        if let Some(ref callback) = self.on_step {
            if let Some(snapshot) = self.state.current_snapshot() {
                callback(snapshot);
            }
        }

        self.state.execution_state = ExecutionState::Paused;
        Ok(())
    }

    /// Step out - execute until exiting current call frame
    pub fn step_out(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        let current_depth = self.state.current_context.stack_depth;

        if current_depth == 0 {
            // Already at top level, just step once
            return self.step_agent();
        }

        self.state.execution_state = ExecutionState::SteppingOut;

        // Execute until we're at a lower depth (exited current frame)
        loop {
            self.state.step()?;

            if self.state.current_context.stack_depth < current_depth {
                break;
            }

            // Check for breakpoints
            if let Some(bp) = self.state.check_breakpoints() {
                let bp_clone = bp.clone();
                let _ = bp; // Release the borrow
                self.handle_breakpoint_hit(&bp_clone)?;
                break;
            }
        }

        self.state.execution_state = ExecutionState::Paused;
        Ok(())
    }

    /// Continue execution until next breakpoint
    pub fn continue_execution(&mut self) -> DebuggerResult<()> {
        if !self.state.is_enabled() {
            return Err(DebuggerError::NotEnabled);
        }

        self.state.execution_state = ExecutionState::Continuing;
        Ok(())
    }

    // ========================================================================
    // BREAKPOINT LOGIC
    // ========================================================================

    /// Handle breakpoint hit
    pub fn handle_breakpoint_hit(&mut self, breakpoint: &Breakpoint) -> DebuggerResult<()> {
        // Pause execution
        self.state.execution_state = ExecutionState::Paused;
        self.state
            .update_statistics(DebugEvent::BreakpointHit(breakpoint.id));

        // Trigger breakpoint callback
        if let Some(ref callback) = self.on_breakpoint {
            callback(breakpoint, &self.state.current_context);
        }

        Ok(())
    }

    /// Check and handle breakpoints at current execution point
    pub fn check_and_handle_breakpoints(&mut self) -> DebuggerResult<Option<Uuid>> {
        if let Some(bp) = self.state.check_breakpoints() {
            let bp_id = bp.id;
            let bp_clone = bp.clone();
            let _ = bp; // Release the borrow
            self.handle_breakpoint_hit(&bp_clone)?;
            return Ok(Some(bp_id));
        }
        Ok(None)
    }

    // ========================================================================
    // STATE CAPTURE
    // ========================================================================

    /// Capture current thought as a snapshot
    pub fn capture_thought_snapshot(
        &mut self,
        thought_id: String,
        content: String,
    ) -> ThoughtSnapshot {
        let snapshot = ThoughtSnapshot::new(
            thought_id,
            content,
            self.state.step_count,
            Some(self.state.current_context.agent_id),
        );

        self.state.current_thought = Some(snapshot.clone());
        self.state.current_context.update_thought(snapshot.clone());

        snapshot
    }

    /// Capture current execution context as a snapshot
    pub fn capture_context_snapshot(&mut self) -> DebugContext {
        self.state.current_context.clone()
    }

    /// Save current state to history
    pub fn save_to_history(&mut self) {
        self.state.capture_snapshot();
    }

    /// Update context with new workflow state
    pub fn update_workflow_state(&mut self, new_state: WorkflowState) {
        self.state.current_context.workflow_state = new_state;
    }

    /// Push a new call frame onto the stack
    pub fn push_call_frame(&mut self, name: String, workflow_state: WorkflowState) -> Uuid {
        let parent_id = self
            .state
            .current_context
            .current_frame()
            .map(|f| f.frame_id);

        let frame = CallFrame::new(name, workflow_state, self.state.step_count, parent_id);

        let frame_id = frame.frame_id;
        self.state.current_context.push_frame(frame);
        frame_id
    }

    /// Pop the current call frame from the stack
    pub fn pop_call_frame(&mut self) -> Option<CallFrame> {
        self.state.current_context.pop_frame()
    }

    /// Set a variable in current context
    pub fn set_context_variable(&mut self, name: String, value: serde_json::Value) {
        self.state.current_context.set_variable(name, value);
    }

    // ========================================================================
    // COMMAND PROCESSING
    // ========================================================================

    /// Process a debug command and return result
    pub fn process_command(&mut self, command: DebugCommand) -> DebuggerResult<CommandResult> {
        match command {
            DebugCommand::Enable => {
                self.state.enable();
                Ok(CommandResult::Success {
                    message: "Debug mode enabled".to_string(),
                })
            }

            DebugCommand::Disable => {
                self.state.disable();
                Ok(CommandResult::Success {
                    message: "Debug mode disabled".to_string(),
                })
            }

            DebugCommand::Pause => {
                self.pause_agent()?;
                Ok(CommandResult::Success {
                    message: "Agent paused".to_string(),
                })
            }

            DebugCommand::Resume => {
                self.resume_agent()?;
                Ok(CommandResult::Success {
                    message: "Agent resumed".to_string(),
                })
            }

            DebugCommand::Step => {
                self.step_agent()?;
                Ok(CommandResult::StepComplete {
                    step_number: self.state.step_count,
                    snapshot: self.state.current_snapshot().cloned(),
                })
            }

            DebugCommand::StepOver => {
                self.step_over()?;
                Ok(CommandResult::StepComplete {
                    step_number: self.state.step_count,
                    snapshot: self.state.current_snapshot().cloned(),
                })
            }

            DebugCommand::StepInto => {
                self.step_into()?;
                Ok(CommandResult::StepComplete {
                    step_number: self.state.step_count,
                    snapshot: self.state.current_snapshot().cloned(),
                })
            }

            DebugCommand::StepOut => {
                self.step_out()?;
                Ok(CommandResult::StepComplete {
                    step_number: self.state.step_count,
                    snapshot: self.state.current_snapshot().cloned(),
                })
            }

            DebugCommand::Continue => {
                self.continue_execution()?;
                Ok(CommandResult::Success {
                    message: "Continuing execution".to_string(),
                })
            }

            DebugCommand::SetBreakpoint {
                location,
                condition,
            } => {
                let bp = if let Some(cond) = condition {
                    Breakpoint::with_condition(location.clone(), cond)
                } else {
                    Breakpoint::new(location.clone())
                };
                let bp_id = self.state.add_breakpoint(bp);
                Ok(CommandResult::BreakpointSet {
                    breakpoint_id: bp_id,
                    location,
                })
            }

            DebugCommand::RemoveBreakpoint { id } => {
                self.state.remove_breakpoint(&id)?;
                Ok(CommandResult::Success {
                    message: format!("Breakpoint {} removed", id),
                })
            }

            DebugCommand::ToggleBreakpoint { id } => {
                self.state.toggle_breakpoint(&id)?;
                Ok(CommandResult::Success {
                    message: format!("Breakpoint {} toggled", id),
                })
            }

            DebugCommand::ListBreakpoints => Ok(CommandResult::BreakpointList {
                breakpoints: self.state.get_breakpoints().to_vec(),
            }),

            DebugCommand::InspectContext => Ok(CommandResult::ContextInspection {
                context: self.state.current_context.clone(),
            }),

            DebugCommand::ShowStack => Ok(CommandResult::StackTrace {
                trace: self.state.current_context.format_stack_trace(),
                frames: self.state.current_context.call_stack.clone(),
            }),

            DebugCommand::Evaluate { expression } => {
                // TODO: Implement expression evaluation
                // For now, return a placeholder
                Ok(CommandResult::EvaluationResult {
                    expression: expression.clone(),
                    result: serde_json::json!({
                        "error": "Expression evaluation not yet implemented",
                        "expression": expression
                    }),
                })
            }

            DebugCommand::GotoHistory { index } => {
                self.state.goto_history(index)?;
                Ok(CommandResult::Success {
                    message: format!("Navigated to history index {}", index),
                })
            }

            DebugCommand::ShowHistory => Ok(CommandResult::HistoryList {
                history: self.state.get_history().to_vec(),
            }),

            DebugCommand::ClearHistory => {
                self.state.clear_history();
                Ok(CommandResult::Success {
                    message: "History cleared".to_string(),
                })
            }

            DebugCommand::GetStatistics => Ok(CommandResult::Statistics {
                stats: self.state.statistics.clone(),
            }),
        }
    }

    /// Check if execution should pause (for integration with agent runtime)
    pub fn should_pause(&self) -> bool {
        self.state.execution_state.is_paused()
    }

    /// Check if in stepping mode
    pub fn is_stepping(&self) -> bool {
        self.state.execution_state.is_stepping()
    }

    /// Get agent ID being debugged
    pub fn agent_id(&self) -> Uuid {
        self.state.current_context.agent_id
    }
}

// ============================================================================
// COMMAND RESULT
// ============================================================================

/// Result of executing a debug command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandResult {
    /// Command completed successfully
    Success { message: String },

    /// Step completed
    StepComplete {
        step_number: u64,
        snapshot: Option<DebugSnapshot>,
    },

    /// Breakpoint was set
    BreakpointSet {
        breakpoint_id: Uuid,
        location: BreakpointLocation,
    },

    /// List of breakpoints
    BreakpointList { breakpoints: Vec<Breakpoint> },

    /// Context inspection result
    ContextInspection { context: DebugContext },

    /// Stack trace result
    StackTrace {
        trace: String,
        frames: Vec<CallFrame>,
    },

    /// Expression evaluation result
    EvaluationResult {
        expression: String,
        result: serde_json::Value,
    },

    /// History list
    HistoryList { history: Vec<DebugSnapshot> },

    /// Statistics
    Statistics { stats: DebugStatistics },
}

// ============================================================================
// AGENT RUNTIME INTEGRATION
// ============================================================================

/// Trait for agent execution that can be debugged
pub trait DebuggableAgent {
    /// Get the debugger for this agent
    fn debugger(&self) -> Option<&Debugger>;

    /// Get mutable debugger for this agent
    fn debugger_mut(&mut self) -> Option<&mut Debugger>;

    /// Execute one step of agent logic
    fn execute_step(&mut self) -> DebuggerResult<()>;

    /// Called before each step - returns true if should continue
    fn before_step(&mut self) -> bool {
        if let Some(debugger) = self.debugger_mut() {
            // Check if we should pause
            if debugger.should_pause() {
                return false;
            }

            // Increment step and capture state
            let _ = debugger.state_mut().step();
            debugger.save_to_history();

            // Check breakpoints
            let _ = debugger.check_and_handle_breakpoints();

            // If we hit a breakpoint, pause
            if debugger.should_pause() {
                return false;
            }
        }
        true
    }

    /// Called after each step
    fn after_step(&mut self) {
        if let Some(debugger) = self.debugger_mut() {
            debugger.save_to_history();
        }
    }
}

/// Helper function to run agent with debugging support
pub async fn run_with_debugging<A: DebuggableAgent>(agent: &mut A) -> DebuggerResult<()> {
    loop {
        // Check if we should continue
        if !agent.before_step() {
            // Paused, wait for resume command
            break;
        }

        // Execute the step
        agent.execute_step()?;

        // After step processing
        agent.after_step();
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_state_creation() {
        let agent_id = Uuid::new_v4();
        let state = DebuggerState::new(agent_id);

        assert!(!state.is_enabled());
        assert_eq!(state.execution_state, ExecutionState::Running);
        assert_eq!(state.step_count, 0);
        assert_eq!(state.breakpoints.len(), 0);
    }

    #[test]
    fn test_enable_disable_debug_mode() {
        let agent_id = Uuid::new_v4();
        let mut state = DebuggerState::new(agent_id);

        state.enable();
        assert!(state.is_enabled());
        assert!(state.started_at.is_some());

        state.disable();
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_add_remove_breakpoint() {
        let agent_id = Uuid::new_v4();
        let mut state = DebuggerState::new(agent_id);

        let bp = Breakpoint::new(BreakpointLocation::WorkflowState {
            state: WorkflowState::Running,
        });
        let bp_id = bp.id;

        state.add_breakpoint(bp);
        assert_eq!(state.breakpoints.len(), 1);

        state.remove_breakpoint(&bp_id).unwrap();
        assert_eq!(state.breakpoints.len(), 0);
    }

    #[test]
    fn test_execution_state_checks() {
        assert!(ExecutionState::Running.is_running());
        assert!(!ExecutionState::Paused.is_running());
        assert!(ExecutionState::Paused.is_paused());
        assert!(ExecutionState::SteppingInto.is_stepping());
    }

    #[test]
    fn test_thought_snapshot() {
        let snapshot = ThoughtSnapshot::new(
            "test-thought".to_string(),
            "Test thought content that is very long and needs to be summarized".to_string(),
            1,
            None,
        );

        assert_eq!(snapshot.thought_id, "test-thought");
        assert_eq!(snapshot.step_number, 1);
        assert!(snapshot.summary().len() <= 103); // 100 chars + "..."
    }

    #[test]
    fn test_call_frame() {
        let mut frame = CallFrame::new(
            "test_function".to_string(),
            WorkflowState::Running,
            10,
            None,
        );

        frame.set_variable("x".to_string(), serde_json::json!(42));
        assert_eq!(frame.get_variable("x"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_debug_context() {
        let agent_id = Uuid::new_v4();
        let mut context = DebugContext::new(agent_id, WorkflowState::Idle);

        context.set_variable("test".to_string(), serde_json::json!("value"));
        assert_eq!(context.stack_depth, 0);

        let frame = CallFrame::new("frame1".to_string(), WorkflowState::Running, 0, None);
        context.push_frame(frame);
        assert_eq!(context.stack_depth, 1);

        context.pop_frame();
        assert_eq!(context.stack_depth, 0);
    }

    #[test]
    fn test_breakpoint_location_display() {
        let loc = BreakpointLocation::WorkflowState {
            state: WorkflowState::Running,
        };
        assert!(loc.to_string().contains("Running"));

        let loc = BreakpointLocation::StepCount { step: 42 };
        assert!(loc.to_string().contains("42"));
    }

    #[test]
    fn test_breakpoint_enable_disable() {
        let mut bp = Breakpoint::new(BreakpointLocation::AnyTransition);

        assert!(bp.enabled);
        bp.disable();
        assert!(!bp.enabled);
        bp.enable();
        assert!(bp.enabled);
        bp.toggle();
        assert!(!bp.enabled);
    }

    #[test]
    fn test_step_execution() {
        let agent_id = Uuid::new_v4();
        let mut state = DebuggerState::new(agent_id);

        state.enable();
        state.step().unwrap();

        assert_eq!(state.step_count, 1);
        assert_eq!(state.current_context.current_step, 1);
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_history_navigation() {
        let agent_id = Uuid::new_v4();
        let mut state = DebuggerState::new(agent_id);

        state.enable();
        state.step().unwrap();
        state.step().unwrap();
        state.step().unwrap();

        assert_eq!(state.history.len(), 3);

        state.goto_history(1).unwrap();
        assert_eq!(state.history_index, Some(1));
    }

    #[test]
    fn test_serialization() {
        let agent_id = Uuid::new_v4();
        let state = DebuggerState::new(agent_id);

        let json = state.to_json().unwrap();
        let deserialized = DebuggerState::from_json(&json).unwrap();

        assert_eq!(state.debug_mode, deserialized.debug_mode);
        assert_eq!(state.step_count, deserialized.step_count);
    }

    // ========================================================================
    // TESTS FOR CORE DEBUGGER LOGIC (Phase 3:6.2)
    // ========================================================================

    #[test]
    fn test_debugger_creation() {
        let agent_id = Uuid::new_v4();
        let debugger = Debugger::new(agent_id);

        assert!(!debugger.state().is_enabled());
        assert_eq!(debugger.agent_id(), agent_id);
    }

    #[test]
    fn test_pause_resume_agent() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        // Enable debugging first
        debugger.state_mut().enable();

        // Pause
        debugger.pause_agent().unwrap();
        assert!(debugger.should_pause());
        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);

        // Resume
        debugger.resume_agent().unwrap();
        assert!(!debugger.should_pause());
        assert_eq!(debugger.state().execution_state, ExecutionState::Running);
    }

    #[test]
    fn test_pause_without_debug_enabled() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        // Try to pause without enabling debug mode
        let result = debugger.pause_agent();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DebuggerError::NotEnabled));
    }

    #[test]
    fn test_resume_when_not_paused() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // Try to resume when not paused
        let result = debugger.resume_agent();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DebuggerError::CannotResumeWhileNotPaused
        ));
    }

    #[test]
    fn test_step_agent() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // Execute a step
        debugger.step_agent().unwrap();

        assert_eq!(debugger.state().step_count, 1);
        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
        assert_eq!(debugger.state().history.len(), 1);
    }

    #[test]
    fn test_step_into() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // Execute step into
        debugger.step_into().unwrap();

        assert_eq!(debugger.state().step_count, 1);
        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    }

    #[test]
    fn test_step_over() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // Execute step over (with no nested calls, behaves like step)
        debugger.step_over().unwrap();

        assert!(debugger.state().step_count >= 1);
        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    }

    #[test]
    fn test_step_out_at_top_level() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // At top level (depth 0), step out should just step once
        debugger.step_out().unwrap();

        assert_eq!(debugger.state().step_count, 1);
        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    }

    #[test]
    fn test_step_out_with_stack() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // Push a call frame to increase depth
        debugger.push_call_frame("test_function".to_string(), WorkflowState::Running);
        assert_eq!(debugger.state().current_context.stack_depth, 1);

        // Step out should reduce depth
        debugger.step_out().unwrap();

        assert!(debugger.state().step_count >= 1);
        assert_eq!(debugger.state().current_context.stack_depth, 0);
    }

    #[test]
    fn test_continue_execution() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        debugger.pause_agent().unwrap();

        // Continue execution
        debugger.continue_execution().unwrap();

        assert_eq!(debugger.state().execution_state, ExecutionState::Continuing);
    }

    #[test]
    fn test_capture_thought_snapshot() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        let snapshot = debugger.capture_thought_snapshot(
            "test-thought-123".to_string(),
            "This is a test thought".to_string(),
        );

        assert_eq!(snapshot.thought_id, "test-thought-123");
        assert_eq!(snapshot.content, "This is a test thought");
        assert_eq!(snapshot.step_number, 0);
        assert!(debugger.state().current_thought.is_some());
    }

    #[test]
    fn test_capture_context_snapshot() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        debugger.update_workflow_state(WorkflowState::Running);

        let context = debugger.capture_context_snapshot();

        assert_eq!(context.agent_id, agent_id);
        assert_eq!(context.workflow_state, WorkflowState::Running);
    }

    #[test]
    fn test_save_to_history() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        assert_eq!(debugger.state().history.len(), 0);

        debugger.save_to_history();

        assert_eq!(debugger.state().history.len(), 1);
    }

    #[test]
    fn test_update_workflow_state() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.update_workflow_state(WorkflowState::Running);
        assert_eq!(
            debugger.state().current_context.workflow_state,
            WorkflowState::Running
        );

        debugger.update_workflow_state(WorkflowState::Paused);
        assert_eq!(
            debugger.state().current_context.workflow_state,
            WorkflowState::Paused
        );
    }

    #[test]
    fn test_push_pop_call_frame() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        assert_eq!(debugger.state().current_context.stack_depth, 0);

        // Push a frame
        let frame_id = debugger.push_call_frame("function1".to_string(), WorkflowState::Running);
        assert_eq!(debugger.state().current_context.stack_depth, 1);

        // Push another frame
        debugger.push_call_frame("function2".to_string(), WorkflowState::Running);
        assert_eq!(debugger.state().current_context.stack_depth, 2);

        // Pop a frame
        let popped = debugger.pop_call_frame();
        assert!(popped.is_some());
        assert_eq!(debugger.state().current_context.stack_depth, 1);

        // Pop last frame
        debugger.pop_call_frame();
        assert_eq!(debugger.state().current_context.stack_depth, 0);
    }

    #[test]
    fn test_set_context_variable() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.set_context_variable("test_var".to_string(), serde_json::json!(42));

        let var = debugger
            .state()
            .current_context
            .local_variables
            .get("test_var");
        assert_eq!(var, Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_handle_breakpoint_hit() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 5 });
        let bp_id = bp.id;

        debugger.handle_breakpoint_hit(&bp).unwrap();

        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
        assert_eq!(debugger.state().statistics.breakpoints_hit, 1);
    }

    #[test]
    fn test_check_and_handle_breakpoints() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        // Set a breakpoint at step 1
        let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 1 });
        debugger.state_mut().add_breakpoint(bp);

        // Step to trigger the breakpoint
        debugger.state_mut().step().unwrap();

        // Check breakpoints
        let result = debugger.check_and_handle_breakpoints();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
        assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    }

    #[test]
    fn test_process_command_enable() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        let result = debugger.process_command(DebugCommand::Enable);
        assert!(result.is_ok());
        assert!(debugger.state().is_enabled());

        match result.unwrap() {
            CommandResult::Success { message } => {
                assert!(message.contains("enabled"));
            }
            _ => panic!("Expected Success result"),
        }
    }

    #[test]
    fn test_process_command_disable() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        let result = debugger.process_command(DebugCommand::Disable);
        assert!(result.is_ok());
        assert!(!debugger.state().is_enabled());
    }

    #[test]
    fn test_process_command_pause() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        let result = debugger.process_command(DebugCommand::Pause);
        assert!(result.is_ok());
        assert!(debugger.should_pause());
    }

    #[test]
    fn test_process_command_resume() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        debugger.pause_agent().unwrap();

        let result = debugger.process_command(DebugCommand::Resume);
        assert!(result.is_ok());
        assert!(!debugger.should_pause());
    }

    #[test]
    fn test_process_command_step() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();

        let result = debugger.process_command(DebugCommand::Step);
        assert!(result.is_ok());

        match result.unwrap() {
            CommandResult::StepComplete { step_number, .. } => {
                assert_eq!(step_number, 1);
            }
            _ => panic!("Expected StepComplete result"),
        }
    }

    #[test]
    fn test_process_command_set_breakpoint() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        let location = BreakpointLocation::StepCount { step: 10 };
        let result = debugger.process_command(DebugCommand::SetBreakpoint {
            location: location.clone(),
            condition: None,
        });

        assert!(result.is_ok());
        assert_eq!(debugger.state().breakpoints.len(), 1);

        match result.unwrap() {
            CommandResult::BreakpointSet { breakpoint_id, .. } => {
                assert_ne!(breakpoint_id, Uuid::nil());
            }
            _ => panic!("Expected BreakpointSet result"),
        }
    }

    #[test]
    fn test_process_command_remove_breakpoint() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        // Add a breakpoint first
        let bp = Breakpoint::new(BreakpointLocation::AnyTransition);
        let bp_id = debugger.state_mut().add_breakpoint(bp);

        // Remove it
        let result = debugger.process_command(DebugCommand::RemoveBreakpoint { id: bp_id });
        assert!(result.is_ok());
        assert_eq!(debugger.state().breakpoints.len(), 0);
    }

    #[test]
    fn test_process_command_list_breakpoints() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        // Add some breakpoints
        debugger
            .state_mut()
            .add_breakpoint(Breakpoint::new(BreakpointLocation::AnyTransition));
        debugger
            .state_mut()
            .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 5 }));

        let result = debugger.process_command(DebugCommand::ListBreakpoints);
        assert!(result.is_ok());

        match result.unwrap() {
            CommandResult::BreakpointList { breakpoints } => {
                assert_eq!(breakpoints.len(), 2);
            }
            _ => panic!("Expected BreakpointList result"),
        }
    }

    #[test]
    fn test_process_command_inspect_context() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.set_context_variable("test".to_string(), serde_json::json!("value"));

        let result = debugger.process_command(DebugCommand::InspectContext);
        assert!(result.is_ok());

        match result.unwrap() {
            CommandResult::ContextInspection { context } => {
                assert_eq!(context.agent_id, agent_id);
                assert!(context.local_variables.contains_key("test"));
            }
            _ => panic!("Expected ContextInspection result"),
        }
    }

    #[test]
    fn test_process_command_show_stack() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.push_call_frame("main".to_string(), WorkflowState::Running);
        debugger.push_call_frame("helper".to_string(), WorkflowState::Running);

        let result = debugger.process_command(DebugCommand::ShowStack);
        assert!(result.is_ok());

        match result.unwrap() {
            CommandResult::StackTrace { trace, frames } => {
                assert_eq!(frames.len(), 2);
                assert!(trace.contains("main"));
                assert!(trace.contains("helper"));
            }
            _ => panic!("Expected StackTrace result"),
        }
    }

    #[test]
    fn test_process_command_show_history() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        debugger.step_agent().unwrap();
        debugger.step_agent().unwrap();

        let result = debugger.process_command(DebugCommand::ShowHistory);
        assert!(result.is_ok());

        match result.unwrap() {
            CommandResult::HistoryList { history } => {
                assert_eq!(history.len(), 2);
            }
            _ => panic!("Expected HistoryList result"),
        }
    }

    #[test]
    fn test_process_command_get_statistics() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        // After enable(), state is Running. Pause to get a pause count.
        debugger.pause_agent().unwrap();
        // Resume so we can step (step requires not already paused at some points)
        debugger.resume_agent().unwrap();
        debugger.step_agent().unwrap();

        let result = debugger.process_command(DebugCommand::GetStatistics);
        assert!(result.is_ok());

        match result.unwrap() {
            CommandResult::Statistics { stats } => {
                assert_eq!(stats.sessions_started, 1);
                assert!(stats.total_steps > 0);
                assert!(stats.pauses > 0);
            }
            _ => panic!("Expected Statistics result"),
        }
    }

    #[test]
    fn test_is_stepping() {
        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        debugger.state_mut().enable();
        assert!(!debugger.is_stepping());

        debugger.state_mut().execution_state = ExecutionState::SteppingInto;
        assert!(debugger.is_stepping());

        debugger.state_mut().execution_state = ExecutionState::SteppingOver;
        assert!(debugger.is_stepping());

        debugger.state_mut().execution_state = ExecutionState::SteppingOut;
        assert!(debugger.is_stepping());
    }

    #[test]
    fn test_callbacks() {
        use std::sync::{Arc, Mutex};

        let agent_id = Uuid::new_v4();
        let mut debugger = Debugger::new(agent_id);

        // Test pause callback
        let pause_called = Arc::new(Mutex::new(false));
        let pause_called_clone = pause_called.clone();
        debugger.on_pause(move |_ctx| {
            *pause_called_clone.lock().unwrap() = true;
        });

        debugger.state_mut().enable();
        debugger.pause_agent().unwrap();
        assert!(*pause_called.lock().unwrap());

        // Test step callback
        let step_called = Arc::new(Mutex::new(false));
        let step_called_clone = step_called.clone();
        debugger.on_step(move |_snapshot| {
            *step_called_clone.lock().unwrap() = true;
        });

        debugger.resume_agent().unwrap();
        debugger.step_agent().unwrap();
        assert!(*step_called.lock().unwrap());
    }

    #[test]
    fn test_command_result_serialization() {
        let result = CommandResult::Success {
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CommandResult = serde_json::from_str(&json).unwrap();

        match deserialized {
            CommandResult::Success { message } => {
                assert_eq!(message, "Test message");
            }
            _ => panic!("Unexpected variant"),
        }
    }
}
