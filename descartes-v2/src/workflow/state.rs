//! Workflow state persistence
//!
//! Tracks workflow progress across sessions, allowing:
//! - Resume from where you left off
//! - Inspect workflow history
//! - Rollback to previous stages

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::gate::ApprovalMethod;
use crate::{Error, Result};

/// Workflow run state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Unique run ID
    pub id: String,
    /// Workflow name
    pub workflow: String,
    /// When the workflow started
    pub started_at: DateTime<Utc>,
    /// When the workflow was last updated
    pub updated_at: DateTime<Utc>,
    /// Current stage
    pub current_stage: String,
    /// Overall status
    pub status: WorkflowStatus,
    /// Per-stage state
    pub stages: HashMap<String, StageState>,
    /// Gate decisions
    pub gates: HashMap<String, GateState>,
    /// Run configuration overrides
    pub config_overrides: RunOverrides,
}

/// Overall workflow status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Currently running
    Running,
    /// Waiting at a gate
    WaitingAtGate,
    /// Paused by user
    Paused,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

/// State of a single stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    /// Stage status
    pub status: StageStatus,
    /// When the stage started
    pub started_at: Option<DateTime<Utc>>,
    /// When the stage completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Session ID for this stage
    pub session_id: Option<String>,
    /// Generated handoff for next stage
    pub handoff: Option<String>,
    /// Stage output/summary
    pub output: Option<String>,
    /// Error if failed
    pub error: Option<String>,
}

impl Default for StageState {
    fn default() -> Self {
        Self {
            status: StageStatus::Pending,
            started_at: None,
            completed_at: None,
            session_id: None,
            handoff: None,
            output: None,
            error: None,
        }
    }
}

/// Status of a stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Not yet started
    Pending,
    /// Currently running
    InProgress,
    /// Completed successfully
    Completed,
    /// Skipped by user
    Skipped,
    /// Failed with error
    Failed,
}

/// State of a gate decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateState {
    /// Gate status
    pub status: GateStatus,
    /// When notification was sent
    pub notified_at: Option<DateTime<Utc>>,
    /// When gate was resolved
    pub resolved_at: Option<DateTime<Utc>>,
    /// How the gate was resolved
    pub method: Option<ApprovalMethod>,
    /// User message if any
    pub message: Option<String>,
}

impl Default for GateState {
    fn default() -> Self {
        Self {
            status: GateStatus::Pending,
            notified_at: None,
            resolved_at: None,
            method: None,
            message: None,
        }
    }
}

/// Status of a gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    /// Not yet reached
    Pending,
    /// Waiting for approval
    Waiting,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Skipped
    Skipped,
}

/// Configuration overrides for a run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunOverrides {
    /// Force all gates to be manual
    pub step_by_step: bool,
    /// Force all gates to be auto
    pub one_shot: bool,
    /// Override specific gate types
    pub gate_overrides: HashMap<String, String>,
    /// Extra context to inject
    pub extra_context: Option<String>,
}

impl WorkflowState {
    /// Create a new workflow state
    pub fn new(workflow: &str, stages: &[String]) -> Self {
        let now = Utc::now();
        let mut stage_states = HashMap::new();

        for stage in stages {
            stage_states.insert(stage.clone(), StageState::default());
        }

        Self {
            id: Uuid::new_v4().to_string(),
            workflow: workflow.to_string(),
            started_at: now,
            updated_at: now,
            current_stage: stages.first().cloned().unwrap_or_default(),
            status: WorkflowStatus::Running,
            stages: stage_states,
            gates: HashMap::new(),
            config_overrides: RunOverrides::default(),
        }
    }

    /// Load workflow state from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read workflow state: {}", e)))?;
        serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse workflow state: {}", e)))
    }

    /// Save workflow state to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Io(e))?;
        }

        let content = serde_yaml::to_string(self)
            .map_err(|e| Error::Config(format!("Failed to serialize workflow state: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| Error::Io(e))
    }

    /// Get the state file path for this workflow
    pub fn state_path(base_dir: &Path, workflow: &str, id: &str) -> PathBuf {
        base_dir
            .join("workflow-state")
            .join(format!("{}-{}.yaml", workflow, id))
    }

    /// Start a stage
    pub fn start_stage(&mut self, stage: &str, session_id: &str) {
        self.current_stage = stage.to_string();
        self.updated_at = Utc::now();

        if let Some(state) = self.stages.get_mut(stage) {
            state.status = StageStatus::InProgress;
            state.started_at = Some(Utc::now());
            state.session_id = Some(session_id.to_string());
        }
    }

    /// Complete a stage
    pub fn complete_stage(&mut self, stage: &str, handoff: Option<String>, output: Option<String>) {
        self.updated_at = Utc::now();

        if let Some(state) = self.stages.get_mut(stage) {
            state.status = StageStatus::Completed;
            state.completed_at = Some(Utc::now());
            state.handoff = handoff;
            state.output = output;
        }
    }

    /// Fail a stage
    pub fn fail_stage(&mut self, stage: &str, error: &str) {
        self.updated_at = Utc::now();
        self.status = WorkflowStatus::Failed;

        if let Some(state) = self.stages.get_mut(stage) {
            state.status = StageStatus::Failed;
            state.completed_at = Some(Utc::now());
            state.error = Some(error.to_string());
        }
    }

    /// Skip a stage
    pub fn skip_stage(&mut self, stage: &str) {
        self.updated_at = Utc::now();

        if let Some(state) = self.stages.get_mut(stage) {
            state.status = StageStatus::Skipped;
            state.completed_at = Some(Utc::now());
        }
    }

    /// Record gate waiting
    pub fn gate_waiting(&mut self, from: &str, to: &str) {
        let key = format!("{}_to_{}", from, to);
        self.status = WorkflowStatus::WaitingAtGate;
        self.updated_at = Utc::now();

        self.gates.insert(
            key,
            GateState {
                status: GateStatus::Waiting,
                notified_at: Some(Utc::now()),
                resolved_at: None,
                method: None,
                message: None,
            },
        );
    }

    /// Record gate approval
    pub fn gate_approved(&mut self, from: &str, to: &str, method: ApprovalMethod, message: Option<String>) {
        let key = format!("{}_to_{}", from, to);
        self.status = WorkflowStatus::Running;
        self.updated_at = Utc::now();

        if let Some(gate) = self.gates.get_mut(&key) {
            gate.status = GateStatus::Approved;
            gate.resolved_at = Some(Utc::now());
            gate.method = Some(method);
            gate.message = message;
        }
    }

    /// Record gate rejection
    pub fn gate_rejected(&mut self, from: &str, to: &str, reason: &str) {
        let key = format!("{}_to_{}", from, to);
        self.status = WorkflowStatus::Cancelled;
        self.updated_at = Utc::now();

        if let Some(gate) = self.gates.get_mut(&key) {
            gate.status = GateStatus::Rejected;
            gate.resolved_at = Some(Utc::now());
            gate.message = Some(reason.to_string());
        }
    }

    /// Complete the workflow
    pub fn complete(&mut self) {
        self.status = WorkflowStatus::Completed;
        self.updated_at = Utc::now();
    }

    /// Pause the workflow
    pub fn pause(&mut self) {
        self.status = WorkflowStatus::Paused;
        self.updated_at = Utc::now();
    }

    /// Cancel the workflow
    pub fn cancel(&mut self) {
        self.status = WorkflowStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    /// Get handoff from previous stage
    pub fn get_previous_handoff(&self, current: &str, stages: &[String]) -> Option<&str> {
        // Find current stage index
        let current_idx = stages.iter().position(|s| s == current)?;
        if current_idx == 0 {
            return None;
        }

        // Get previous stage
        let prev = &stages[current_idx - 1];
        self.stages.get(prev)?.handoff.as_deref()
    }

    /// Check if all stages are complete
    pub fn is_complete(&self) -> bool {
        self.stages.values().all(|s| {
            matches!(s.status, StageStatus::Completed | StageStatus::Skipped)
        })
    }

    /// Get summary of workflow progress
    pub fn summary(&self) -> String {
        let mut lines = vec![
            format!("Workflow: {} ({})", self.workflow, self.id),
            format!("Status: {:?}", self.status),
            format!("Current: {}", self.current_stage),
            String::new(),
            "Stages:".to_string(),
        ];

        for (name, state) in &self.stages {
            let icon = match state.status {
                StageStatus::Pending => "⬜",
                StageStatus::InProgress => "🔄",
                StageStatus::Completed => "✅",
                StageStatus::Skipped => "⏭️",
                StageStatus::Failed => "❌",
            };
            lines.push(format!("  {} {}: {:?}", icon, name, state.status));
        }

        lines.join("\n")
    }
}

/// Manage workflow state files
pub struct StateManager {
    base_dir: PathBuf,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create from default location
    pub fn default() -> Self {
        let base_dir = PathBuf::from(".descartes");
        Self::new(base_dir)
    }

    /// Save workflow state
    pub fn save(&self, state: &WorkflowState) -> Result<PathBuf> {
        let path = WorkflowState::state_path(&self.base_dir, &state.workflow, &state.id);
        state.save(&path)?;
        Ok(path)
    }

    /// Load workflow state by ID
    pub fn load(&self, workflow: &str, id: &str) -> Result<WorkflowState> {
        let path = WorkflowState::state_path(&self.base_dir, workflow, id);
        WorkflowState::load(&path)
    }

    /// Find most recent workflow state
    pub fn find_latest(&self, workflow: &str) -> Result<Option<WorkflowState>> {
        let dir = self.base_dir.join("workflow-state");
        if !dir.exists() {
            return Ok(None);
        }

        let prefix = format!("{}-", workflow);
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| Error::Io(e))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&prefix)
            })
            .collect();

        // Sort by modification time (newest first)
        entries.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        if let Some(entry) = entries.first() {
            let state = WorkflowState::load(&entry.path())?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// List all workflow states
    pub fn list(&self, workflow: Option<&str>) -> Result<Vec<WorkflowState>> {
        let dir = self.base_dir.join("workflow-state");
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut states = Vec::new();

        for entry in std::fs::read_dir(&dir).map_err(|e| Error::Io(e))? {
            let entry = entry.map_err(|e| Error::Io(e))?;
            let path = entry.path();

            if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                if let Ok(state) = WorkflowState::load(&path) {
                    if workflow.map(|w| w == state.workflow).unwrap_or(true) {
                        states.push(state);
                    }
                }
            }
        }

        // Sort by start time (newest first)
        states.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        Ok(states)
    }

    /// Clean up old workflow states
    pub fn cleanup(&self, keep: usize) -> Result<usize> {
        let states = self.list(None)?;
        let mut removed = 0;

        for state in states.into_iter().skip(keep) {
            let path = WorkflowState::state_path(&self.base_dir, &state.workflow, &state.id);
            if std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_new() {
        let stages = vec!["research".to_string(), "plan".to_string()];
        let state = WorkflowState::new("test", &stages);

        assert_eq!(state.workflow, "test");
        assert_eq!(state.current_stage, "research");
        assert_eq!(state.status, WorkflowStatus::Running);
        assert_eq!(state.stages.len(), 2);
    }

    #[test]
    fn test_stage_transitions() {
        let stages = vec!["a".to_string(), "b".to_string()];
        let mut state = WorkflowState::new("test", &stages);

        state.start_stage("a", "sess1");
        assert_eq!(state.stages["a"].status, StageStatus::InProgress);

        state.complete_stage("a", Some("handoff".to_string()), None);
        assert_eq!(state.stages["a"].status, StageStatus::Completed);
        assert_eq!(state.stages["a"].handoff, Some("handoff".to_string()));
    }

    #[test]
    fn test_get_previous_handoff() {
        let stages = vec!["a".to_string(), "b".to_string()];
        let mut state = WorkflowState::new("test", &stages);

        state.complete_stage("a", Some("handoff from a".to_string()), None);

        assert_eq!(
            state.get_previous_handoff("b", &stages),
            Some("handoff from a")
        );
        assert_eq!(state.get_previous_handoff("a", &stages), None);
    }
}
