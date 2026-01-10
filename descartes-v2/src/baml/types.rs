//! Rust types that mirror BAML definitions
//!
//! These types are used to communicate with the BAML runtime and
//! represent the structured outputs from LLM calls.

use serde::{Deserialize, Serialize};

/// Agent categories for subagent spawning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BamlAgentCategory {
    Searcher,
    Analyzer,
    Builder,
    Validator,
}

impl From<BamlAgentCategory> for crate::agent::AgentCategory {
    fn from(cat: BamlAgentCategory) -> Self {
        match cat {
            BamlAgentCategory::Searcher => crate::agent::AgentCategory::Searcher,
            BamlAgentCategory::Analyzer => crate::agent::AgentCategory::Analyzer,
            BamlAgentCategory::Builder => crate::agent::AgentCategory::Builder,
            BamlAgentCategory::Validator => crate::agent::AgentCategory::Validator,
        }
    }
}

/// Task status from SCUD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BamlTaskStatus {
    Pending,
    InProgress,
    Blocked,
    Done,
}

/// Task complexity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BamlComplexity {
    Low,
    Medium,
    High,
}

/// A task in the SCUD graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamlTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: BamlTaskStatus,
    pub complexity: BamlComplexity,
    pub depends_on: Vec<String>,
}

/// Current state of the task graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamlTaskGraphStatus {
    pub total_tasks: i32,
    pub completed: i32,
    pub in_progress: i32,
    pub blocked: i32,
    pub pending: i32,
    pub next_ready: Option<BamlTask>,
    pub blockers: Vec<String>,
}

/// Result from validation/testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamlValidationResult {
    pub passed: bool,
    pub test_summary: Option<String>,
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
}

/// Git status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamlGitStatus {
    pub branch: String,
    pub modified_files: i32,
    pub staged_files: i32,
    pub untracked_files: i32,
    pub has_conflicts: bool,
}

/// Context available to the agent decision function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamlAgentContext {
    pub task_status: BamlTaskGraphStatus,
    pub recent_output: String,
    pub validation: Option<BamlValidationResult>,
    pub git_status: Option<BamlGitStatus>,
    pub iteration: i32,
    pub elapsed_minutes: i32,
}

// Decision types returned by DecideNextStep
// Note: action field is implicit in the enum variant via serde(tag = "action")

/// Decision: Continue building (more tasks remain)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueBuilding {
    pub next_task_id: Option<String>,
    pub approach: String,
}

/// Decision: Request replanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestReplan {
    pub reason: String,
    pub context: String,
    pub preserve_completed: bool,
}

/// Decision: Work is complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Complete {
    pub summary: String,
    pub artifacts: Vec<String>,
}

/// Decision: Need human input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedHumanInput {
    pub question: String,
    pub options: Option<Vec<String>>,
    pub blocking: bool,
}

/// Decision: Spawn a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSubagent {
    pub category: BamlAgentCategory,
    pub task_id: String,
    pub prompt: String,
    pub timeout_seconds: Option<i32>,
}

/// Decision: Run validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunValidation {
    pub scope: String,
    pub continue_on_failure: bool,
}

/// Union type for all possible decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum LoopDecision {
    #[serde(rename = "continue")]
    Continue(ContinueBuilding),
    #[serde(rename = "replan")]
    Replan(RequestReplan),
    #[serde(rename = "complete")]
    Complete(Complete),
    #[serde(rename = "human")]
    Human(NeedHumanInput),
    #[serde(rename = "spawn")]
    Spawn(SpawnSubagent),
    #[serde(rename = "validate")]
    Validate(RunValidation),
}

impl LoopDecision {
    /// Get the action name
    pub fn action_name(&self) -> &str {
        match self {
            LoopDecision::Continue(_) => "continue",
            LoopDecision::Replan(_) => "replan",
            LoopDecision::Complete(_) => "complete",
            LoopDecision::Human(_) => "human",
            LoopDecision::Spawn(_) => "spawn",
            LoopDecision::Validate(_) => "validate",
        }
    }

    /// Check if this decision indicates the loop should stop
    pub fn is_terminal(&self) -> bool {
        matches!(self, LoopDecision::Complete(_) | LoopDecision::Human(_))
    }

    /// Check if this decision requires human intervention
    pub fn needs_human(&self) -> bool {
        matches!(self, LoopDecision::Human(_))
    }
}

// Tool types for subagent operations

/// SCUD operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool_name")]
pub enum ScudTool {
    #[serde(rename = "scud_next")]
    Next,
    #[serde(rename = "scud_complete")]
    Complete { task_id: String, notes: Option<String> },
    #[serde(rename = "scud_block")]
    Block { task_id: String, reason: String },
    #[serde(rename = "scud_waves")]
    Waves,
    #[serde(rename = "scud_add")]
    Add {
        title: String,
        description: String,
        depends_on: Vec<String>,
        complexity: BamlComplexity,
    },
}

/// File operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool_name")]
pub enum FileTool {
    #[serde(rename = "read_file")]
    Read {
        path: String,
        start_line: Option<i32>,
        end_line: Option<i32>,
    },
    #[serde(rename = "write_file")]
    Write { path: String, content: String },
    #[serde(rename = "edit_file")]
    Edit {
        path: String,
        old_content: String,
        new_content: String,
    },
}

/// Search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool_name")]
pub enum SearchTool {
    #[serde(rename = "grep")]
    Grep {
        pattern: String,
        path: Option<String>,
        file_type: Option<String>,
    },
    #[serde(rename = "glob")]
    Glob { pattern: String },
}

/// Git operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool_name")]
pub enum GitTool {
    #[serde(rename = "git_status")]
    Status,
    #[serde(rename = "git_diff")]
    Diff { staged: bool, path: Option<String> },
    #[serde(rename = "git_commit")]
    Commit {
        message: String,
        files: Option<Vec<String>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_decision_parsing() {
        let json = r#"{"action": "continue", "next_task_id": "T3", "approach": "Start implementation"}"#;
        let decision: LoopDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision.action_name(), "continue");
        assert!(!decision.is_terminal());
    }

    #[test]
    fn test_complete_decision() {
        let json = r#"{"action": "complete", "summary": "All done", "artifacts": ["file.rs"]}"#;
        let decision: LoopDecision = serde_json::from_str(json).unwrap();
        assert!(decision.is_terminal());
    }

    #[test]
    fn test_spawn_decision() {
        let json = r#"{
            "action": "spawn",
            "category": "builder",
            "task_id": "T1",
            "prompt": "Implement the feature",
            "timeout_seconds": 300
        }"#;
        let decision: LoopDecision = serde_json::from_str(json).unwrap();
        if let LoopDecision::Spawn(spawn) = decision {
            assert_eq!(spawn.category, BamlAgentCategory::Builder);
            assert_eq!(spawn.task_id, "T1");
        } else {
            panic!("Expected Spawn decision");
        }
    }
}
