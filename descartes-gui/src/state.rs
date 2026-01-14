//! Application state types

/// Agent execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Idle,
    Running,
    Paused,
}

/// Task information for display
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// Main application state
#[derive(Debug, Default)]
pub struct AppState {
    /// Task waves (parallel execution groups)
    pub waves: Vec<Vec<TaskInfo>>,
    /// Current agent status
    pub agent_status: AgentStatus,
    /// Currently executing task
    pub current_task: Option<String>,
    /// Output buffer from agent
    pub output_buffer: String,
}
