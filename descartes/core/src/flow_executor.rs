//! Flow Executor - Custom executor for the flow workflow with state management.
//!
//! The flow workflow is different from standard workflows:
//! - File-based input (PRD path) rather than topic string
//! - Stateful with pause/resume via .scud/flow-state.json
//! - Concurrent QA monitoring during implementation
//! - Orchestrator agent for intelligent error handling

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{error, info, warn};

use crate::agent_definitions::AgentDefinitionLoader;
use crate::traits::ModelBackend;
use crate::workflow_commands::{WorkflowContext, WorkflowStep};
use crate::workflow_executor::{execute_step, WorkflowExecutorConfig};

/// Flow phase status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Active,
    Completed,
    Failed,
    Skipped,
}

/// Individual phase state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub status: PhaseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl Default for PhaseState {
    fn default() -> Self {
        Self {
            status: PhaseStatus::Pending,
            completed_at: None,
            data: serde_json::json!({}),
        }
    }
}

/// Flow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    pub orchestrator_model: String,
    pub implementation_model: String,
    pub qa_model: String,
    pub max_parallel_tasks: usize,
    pub auto_commit: bool,
    pub pause_between_phases: bool,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            orchestrator_model: "opus".to_string(),
            implementation_model: "sonnet".to_string(),
            qa_model: "sonnet".to_string(),
            max_parallel_tasks: 3,
            auto_commit: true,
            pause_between_phases: false,
        }
    }
}

/// Git tracking for flow
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowGitState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Artifact paths
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowArtifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plans_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_log_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_final_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_path: Option<PathBuf>,
}

/// All phase states
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowPhases {
    #[serde(default)]
    pub ingest: PhaseState,
    #[serde(default)]
    pub review_graph: PhaseState,
    #[serde(default)]
    pub plan_tasks: PhaseState,
    #[serde(default)]
    pub implement: PhaseState,
    #[serde(default)]
    pub qa: PhaseState,
    #[serde(default)]
    pub summarize: PhaseState,
}

/// Complete flow state - matches .scud/flow-state.json schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowState {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,
    #[serde(default)]
    pub phases: FlowPhases,
    #[serde(default)]
    pub config: FlowConfig,
    #[serde(default)]
    pub artifacts: FlowArtifacts,
    #[serde(default)]
    pub git: FlowGitState,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Default for FlowState {
    fn default() -> Self {
        Self {
            version: default_version(),
            started_at: None,
            prd_file: None,
            tag: None,
            current_phase: None,
            phases: FlowPhases::default(),
            config: FlowConfig::default(),
            artifacts: FlowArtifacts::default(),
            git: FlowGitState::default(),
        }
    }
}

/// Result from flow execution
#[derive(Debug)]
pub struct FlowResult {
    pub success: bool,
    pub phases_completed: Vec<String>,
    pub phases_failed: Vec<String>,
    pub summary_path: Option<PathBuf>,
    pub duration_secs: u64,
}

/// Flow executor with state management
pub struct FlowExecutor {
    state: FlowState,
    state_path: PathBuf,
    working_dir: PathBuf,
    agent_loader: AgentDefinitionLoader,
    backend: Arc<dyn ModelBackend + Send + Sync>,
}

impl FlowExecutor {
    /// Create new flow executor
    pub async fn new(
        prd_path: PathBuf,
        tag: Option<String>,
        working_dir: Option<PathBuf>,
        backend: Arc<dyn ModelBackend + Send + Sync>,
    ) -> Result<Self> {
        let working_dir = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        let state_path = working_dir.join(".scud/flow-state.json");

        // Load existing state or create new
        let mut state = if state_path.exists() {
            let content = fs::read_to_string(&state_path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            FlowState::default()
        };

        // Update with new execution params
        state.started_at = Some(Utc::now());
        state.prd_file = Some(prd_path.clone());
        state.tag = tag.or(state.tag);
        state.artifacts.prd_path = Some(prd_path);

        let agent_loader = AgentDefinitionLoader::new()
            .map_err(|e| anyhow::anyhow!("Failed to create agent loader: {}", e))?;

        Ok(Self {
            state,
            state_path,
            working_dir,
            agent_loader,
            backend,
        })
    }

    /// Resume from existing state
    pub async fn resume(
        working_dir: Option<PathBuf>,
        backend: Arc<dyn ModelBackend + Send + Sync>,
    ) -> Result<Self> {
        let working_dir = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        let state_path = working_dir.join(".scud/flow-state.json");

        let content = fs::read_to_string(&state_path)
            .await
            .context("No flow state found. Start a new flow first.")?;
        let state: FlowState = serde_json::from_str(&content)?;

        let agent_loader = AgentDefinitionLoader::new()
            .map_err(|e| anyhow::anyhow!("Failed to create agent loader: {}", e))?;

        Ok(Self {
            state,
            state_path,
            working_dir,
            agent_loader,
            backend,
        })
    }

    /// Get the current state
    pub fn state(&self) -> &FlowState {
        &self.state
    }

    /// Execute the full flow workflow
    pub async fn execute(&mut self) -> Result<FlowResult> {
        let start_time = std::time::Instant::now();
        let mut phases_completed = Vec::new();
        let mut phases_failed = Vec::new();

        // Phase 1-3: Sequential
        for phase in ["ingest", "review_graph", "plan_tasks"] {
            // Skip already completed phases
            if self.is_phase_completed(phase) {
                info!("Skipping already completed phase: {}", phase);
                phases_completed.push(phase.to_string());
                continue;
            }

            match self.execute_phase(phase).await {
                Ok(_) => phases_completed.push(phase.to_string()),
                Err(e) => {
                    phases_failed.push(phase.to_string());
                    error!("Phase {} failed: {}", phase, e);
                    // Invoke orchestrator for decision
                    if !self.handle_phase_error(phase, &e).await? {
                        break;
                    }
                }
            }
            self.save_state().await?;
        }

        // Phase 4-6: Sequential execution
        // Note: In a full implementation, implement and qa could run concurrently
        // using separate tasks with shared state. For now, we run them sequentially.
        if !phases_failed.is_empty() {
            // Don't proceed if earlier phases failed
            info!("Skipping remaining phases due to earlier failures");
        } else {
            // Execute implement phase
            if !self.is_phase_completed("implement") {
                match self.execute_phase("implement").await {
                    Ok(_) => phases_completed.push("implement".to_string()),
                    Err(e) => {
                        phases_failed.push("implement".to_string());
                        error!("Implement phase failed: {}", e);
                    }
                }
                self.save_state().await?;
            } else {
                phases_completed.push("implement".to_string());
            }

            // Execute QA phase (reviews implementation)
            if !self.is_phase_completed("qa") && phases_failed.is_empty() {
                match self.execute_phase("qa").await {
                    Ok(_) => phases_completed.push("qa".to_string()),
                    Err(e) => {
                        phases_failed.push("qa".to_string());
                        error!("QA phase failed: {}", e);
                    }
                }
                self.save_state().await?;
            } else if self.is_phase_completed("qa") {
                phases_completed.push("qa".to_string());
            }

            // Phase 6: Summarize
            if !self.is_phase_completed("summarize") {
                if phases_failed.is_empty() || phases_completed.contains(&"implement".to_string()) {
                    match self.execute_phase("summarize").await {
                        Ok(_) => phases_completed.push("summarize".to_string()),
                        Err(e) => {
                            phases_failed.push("summarize".to_string());
                            error!("Summarize phase failed: {}", e);
                        }
                    }
                    self.save_state().await?;
                }
            } else {
                phases_completed.push("summarize".to_string());
            }
        }

        let duration = start_time.elapsed();

        Ok(FlowResult {
            success: phases_failed.is_empty(),
            phases_completed,
            phases_failed,
            summary_path: self.state.artifacts.summary_path.clone(),
            duration_secs: duration.as_secs(),
        })
    }

    /// Check if a phase is already completed
    fn is_phase_completed(&self, phase: &str) -> bool {
        let phase_state = match phase {
            "ingest" => &self.state.phases.ingest,
            "review_graph" => &self.state.phases.review_graph,
            "plan_tasks" => &self.state.phases.plan_tasks,
            "implement" => &self.state.phases.implement,
            "qa" => &self.state.phases.qa,
            "summarize" => &self.state.phases.summarize,
            _ => return false,
        };
        phase_state.status == PhaseStatus::Completed
    }

    /// Execute a single phase
    async fn execute_phase(&mut self, phase: &str) -> Result<()> {
        let agent_name = format!("flow-{}", phase.replace('_', "-"));

        info!("Executing phase: {} with agent: {}", phase, agent_name);

        // Update state
        self.state.current_phase = Some(phase.to_string());
        self.update_phase_status(phase, PhaseStatus::Active);

        // Verify agent exists
        if !self.agent_loader.agent_exists(&agent_name) {
            return Err(anyhow::anyhow!("Agent '{}' not found", agent_name));
        }

        // Build context
        let context = WorkflowContext::new(
            self.working_dir.clone(),
            self.state.tag.as_deref().unwrap_or("flow"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create workflow context: {}", e))?;

        // Build task with context
        let task = format!(
            "Execute {} phase for flow workflow.\n\nPRD: {:?}\nTag: {:?}\nState file: {:?}\n\nFollow your agent instructions to complete this phase.",
            phase,
            self.state.prd_file,
            self.state.tag,
            self.state_path
        );

        // Create workflow step
        let step = WorkflowStep {
            name: format!("Flow: {}", phase),
            agent: agent_name,
            task: task.clone(),
            parallel: false,
            output: None,
        };

        let config = WorkflowExecutorConfig::default();
        let result = execute_step(&step, &task, &context, self.backend.as_ref(), &config).await
            .map_err(|e| anyhow::anyhow!("Step execution failed: {}", e))?;

        if result.success {
            self.update_phase_status(phase, PhaseStatus::Completed);
            info!("Phase {} completed successfully", phase);
        } else {
            self.update_phase_status(phase, PhaseStatus::Failed);
            return Err(anyhow::anyhow!(
                "Phase {} failed: {}",
                phase,
                result.error.unwrap_or_else(|| "unknown error".to_string())
            ));
        }

        Ok(())
    }

    /// Handle phase error with orchestrator agent
    async fn handle_phase_error(&mut self, phase: &str, error: &anyhow::Error) -> Result<bool> {
        info!("Invoking orchestrator for error recovery");

        // Check if orchestrator agent exists
        if !self.agent_loader.agent_exists("flow-orchestrator") {
            warn!("Orchestrator agent not found, aborting");
            return Ok(false);
        }

        let context = WorkflowContext::new(
            self.working_dir.clone(),
            self.state.tag.as_deref().unwrap_or("flow"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create workflow context: {}", e))?;

        let task = format!(
            "Phase '{}' failed with error: {}\n\nDecide: retry, skip, or abort?\n\nProvide your decision in the format:\nDecision: <retry|skip|abort>\nReason: <explanation>",
            phase, error
        );

        let step = WorkflowStep {
            name: "Flow: Error Recovery".to_string(),
            agent: "flow-orchestrator".to_string(),
            task: task.clone(),
            parallel: false,
            output: None,
        };

        let config = WorkflowExecutorConfig::default();
        let result = execute_step(&step, &task, &context, self.backend.as_ref(), &config).await
            .map_err(|e| anyhow::anyhow!("Orchestrator execution failed: {}", e))?;

        // Parse decision from result
        // Simple heuristic: check if output contains "abort" or not
        let should_continue = result.success
            && !result.output.to_lowercase().contains("decision: abort")
            && !result.output.to_lowercase().contains("abort");

        if should_continue {
            info!("Orchestrator decided to continue");
        } else {
            info!("Orchestrator decided to abort");
        }

        Ok(should_continue)
    }

    fn update_phase_status(&mut self, phase: &str, status: PhaseStatus) {
        let phase_state = match phase {
            "ingest" => &mut self.state.phases.ingest,
            "review_graph" => &mut self.state.phases.review_graph,
            "plan_tasks" => &mut self.state.phases.plan_tasks,
            "implement" => &mut self.state.phases.implement,
            "qa" => &mut self.state.phases.qa,
            "summarize" => &mut self.state.phases.summarize,
            _ => return,
        };

        phase_state.status = status.clone();
        if status == PhaseStatus::Completed {
            phase_state.completed_at = Some(Utc::now());
        }
    }

    /// Save state to disk
    async fn save_state(&self) -> Result<()> {
        // Ensure .scud directory exists
        let scud_dir = self.state_path.parent().unwrap();
        if !scud_dir.exists() {
            fs::create_dir_all(scud_dir).await?;
        }

        let content = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_path, content).await?;
        info!("Saved flow state to {:?}", self.state_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_state_default() {
        let state = FlowState::default();
        assert_eq!(state.version, "1.0");
        assert!(state.started_at.is_none());
        assert!(state.tag.is_none());
    }

    #[test]
    fn test_phase_status_serde() {
        let status = PhaseStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""completed""#);

        let parsed: PhaseStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PhaseStatus::Completed);
    }

    #[test]
    fn test_flow_state_serde() {
        let mut state = FlowState::default();
        state.tag = Some("test-flow".to_string());
        state.phases.ingest.status = PhaseStatus::Completed;

        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: FlowState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.tag, Some("test-flow".to_string()));
        assert_eq!(parsed.phases.ingest.status, PhaseStatus::Completed);
    }

    #[test]
    fn test_flow_config_default() {
        let config = FlowConfig::default();
        assert_eq!(config.orchestrator_model, "opus");
        assert_eq!(config.max_parallel_tasks, 3);
        assert!(config.auto_commit);
    }
}
