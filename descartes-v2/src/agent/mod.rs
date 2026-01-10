//! Agent execution and subagent management
//!
//! Provides:
//! - Agent categories with configurable defaults
//! - Subagent spawning with 1-level depth limit
//! - Tool definitions (read, write, edit, bash)

mod category;
mod subagent;
mod tools;

pub use category::AgentCategory;
pub use subagent::{spawn_subagent, SubagentResult};
pub use tools::{Tool, ToolSet};

use serde::{Deserialize, Serialize};

use crate::config::CategoryConfig;

/// Agent state during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Session ID
    pub session_id: String,
    /// Current status
    pub status: AgentStatus,
    /// Category of this agent
    pub category: AgentCategory,
    /// Number of tool calls made
    pub tool_calls: usize,
    /// Whether this is a subagent
    pub is_subagent: bool,
}

/// Agent execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is starting up
    Starting,
    /// Agent is running
    Running,
    /// Agent is waiting for tool result
    WaitingForTool,
    /// Agent completed successfully
    Completed,
    /// Agent failed
    Failed,
    /// Agent was blocked (e.g., nested spawn attempt)
    Blocked,
}

impl AgentCategory {
    /// Get default configuration for this category
    pub fn default_config(&self) -> CategoryConfig {
        match self {
            AgentCategory::Searcher => CategoryConfig {
                description: "Fast parallel code search".to_string(),
                model: "sonnet".to_string(),
                tools: vec!["read".to_string(), "bash".to_string()],
                parallel: true,
                backpressure: false,
                prompt_template: None,
            },
            AgentCategory::Analyzer => CategoryConfig {
                description: "Deep code analysis".to_string(),
                model: "sonnet".to_string(),
                tools: vec!["read".to_string()],
                parallel: true,
                backpressure: false,
                prompt_template: None,
            },
            AgentCategory::Builder => CategoryConfig {
                description: "Code implementation".to_string(),
                model: "opus".to_string(),
                tools: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "edit".to_string(),
                    "bash".to_string(),
                ],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
            AgentCategory::Validator => CategoryConfig {
                description: "Test runner (backpressure gate)".to_string(),
                model: "sonnet".to_string(),
                tools: vec!["bash".to_string()],
                parallel: false,
                backpressure: true,
                prompt_template: None,
            },
            AgentCategory::Planner => CategoryConfig {
                description: "Task planning and breakdown".to_string(),
                model: "opus".to_string(),
                tools: vec!["read".to_string(), "bash".to_string()],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
            AgentCategory::Custom(_) => CategoryConfig {
                description: "Custom agent category".to_string(),
                model: "sonnet".to_string(),
                tools: vec!["read".to_string(), "bash".to_string()],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        }
    }
}
