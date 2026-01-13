//! Flow Executor - Custom executor for the flow workflow with state management.
//!
//! The flow workflow is different from standard workflows:
//! - File-based input (PRD path) rather than topic string
//! - Stateful with pause/resume via .scud/flow-state.json
//! - Concurrent QA monitoring during implementation
//! - Orchestrator agent for intelligent error handling

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
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

    /// Number of times this phase has been retried
    #[serde(default)]
    pub retry_count: u32,

    /// Last error message if phase failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl Default for PhaseState {
    fn default() -> Self {
        Self {
            status: PhaseStatus::Pending,
            completed_at: None,
            data: serde_json::json!({}),
            retry_count: 0,
            last_error: None,
        }
    }
}

/// Orchestrator decision for error handling
#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorDecision {
    /// Retry the failed phase
    Retry { reason: String },
    /// Skip the phase and continue
    Skip { reason: String },
    /// Abort the entire flow
    Abort { reason: String },
    /// Continue as if nothing happened
    Continue { reason: String },
}

impl OrchestratorDecision {
    /// Parse from orchestrator response text
    pub fn parse(output: &str) -> Self {
        let lower = output.to_lowercase();

        // Extract reason if present (look for "reason:" line)
        let reason = lower
            .lines()
            .find(|l| l.trim().starts_with("reason:"))
            .map(|l| l.trim().trim_start_matches("reason:").trim().to_string())
            .unwrap_or_else(|| "No reason provided".to_string());

        if lower.contains("decision: retry") || lower.contains("decision:retry") {
            OrchestratorDecision::Retry { reason }
        } else if lower.contains("decision: skip") || lower.contains("decision:skip") {
            OrchestratorDecision::Skip { reason }
        } else if lower.contains("decision: abort") || lower.contains("decision:abort") {
            OrchestratorDecision::Abort { reason }
        } else {
            // Default to continue if unclear
            OrchestratorDecision::Continue { reason }
        }
    }
}

/// QA log entry for tracking quality checks during implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QALogEntry {
    /// Timestamp of the QA check
    pub timestamp: DateTime<Utc>,
    /// Git commit that was reviewed
    pub commit: String,
    /// Severity level: info, warning, error
    pub severity: String,
    /// QA message/assessment
    pub message: String,
}

/// QA monitoring state during implementation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QAMonitorState {
    /// Whether QA is actively monitoring
    pub active: bool,
    /// Number of issues found
    pub issues_found: u32,
    /// Last commit reviewed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reviewed_commit: Option<String>,
    /// QA log entries
    #[serde(default)]
    pub log_entries: Vec<QALogEntry>,
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

    /// Timeout per phase in seconds (default: 1800 = 30 minutes)
    #[serde(default = "default_phase_timeout")]
    pub phase_timeout_secs: u64,

    /// Watchdog check interval in seconds (default: 60)
    #[serde(default = "default_watchdog_interval")]
    pub watchdog_interval_secs: u64,

    /// Maximum total flow duration in seconds (default: 14400 = 4 hours)
    #[serde(default = "default_max_flow_duration")]
    pub max_flow_duration_secs: u64,

    /// Maximum retries per phase (default: 3)
    #[serde(default = "default_max_retries")]
    pub max_retries_per_phase: u32,

    /// QA check interval in seconds (default: 30)
    #[serde(default = "default_qa_check_interval")]
    pub qa_check_interval_secs: u64,
}

fn default_phase_timeout() -> u64 {
    1800 // 30 minutes
}

fn default_qa_check_interval() -> u64 {
    30 // 30 seconds
}

fn default_watchdog_interval() -> u64 {
    60 // 1 minute
}

fn default_max_flow_duration() -> u64 {
    14400 // 4 hours
}

fn default_max_retries() -> u32 {
    3
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
            phase_timeout_secs: default_phase_timeout(),
            watchdog_interval_secs: default_watchdog_interval(),
            max_flow_duration_secs: default_max_flow_duration(),
            max_retries_per_phase: default_max_retries(),
            qa_check_interval_secs: default_qa_check_interval(),
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

    /// Git checkpoint commit after each phase
    #[serde(default)]
    pub phase_checkpoints: HashMap<String, String>,
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
    #[serde(default)]
    pub qa_monitor: QAMonitorState,
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
            qa_monitor: QAMonitorState::default(),
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
        let max_duration = Duration::from_secs(self.state.config.max_flow_duration_secs);
        let mut phases_completed = Vec::new();
        let mut phases_failed = Vec::new();

        // Initialize git state
        if let Err(e) = self.init_git_state() {
            warn!("Failed to initialize git state: {}", e);
        }

        // Phase 1-3: Sequential with retry support
        for phase in ["ingest", "review_graph", "plan_tasks"] {
            // Check total flow timeout
            if start_time.elapsed() > max_duration {
                error!(
                    "Flow exceeded maximum duration of {} seconds",
                    self.state.config.max_flow_duration_secs
                );
                break;
            }

            // Skip already completed phases
            if self.is_phase_completed(phase) {
                info!("Skipping already completed phase: {}", phase);
                phases_completed.push(phase.to_string());
                continue;
            }

            match self.execute_phase_with_retry(phase).await {
                Ok(_) => {
                    phases_completed.push(phase.to_string());
                    // Create git checkpoint after successful phase
                    if let Err(e) = self.create_phase_checkpoint(phase) {
                        warn!("Failed to create git checkpoint for phase '{}': {}", phase, e);
                    }
                }
                Err(e) => {
                    phases_failed.push(phase.to_string());
                    error!("Phase {} failed: {}", phase, e);
                    break;
                }
            }
            self.save_state().await?;
        }

        // Phase 4-6: Sequential execution with retry support
        // Note: In a full implementation, implement and qa could run concurrently
        // using separate tasks with shared state. For now, we run them sequentially.
        if !phases_failed.is_empty() {
            // Don't proceed if earlier phases failed
            info!("Skipping remaining phases due to earlier failures");
        } else {
            // Check flow timeout
            if start_time.elapsed() > max_duration {
                error!(
                    "Flow exceeded maximum duration of {} seconds",
                    self.state.config.max_flow_duration_secs
                );
            } else {
                // Execute implement phase with concurrent QA monitoring
                if !self.is_phase_completed("implement") {
                    match self.execute_implement_with_qa().await {
                        Ok(_) => {
                            phases_completed.push("implement".to_string());
                            if let Err(e) = self.create_phase_checkpoint("implement") {
                                warn!("Failed to create git checkpoint for implement phase: {}", e);
                            }
                        }
                        Err(e) => {
                            phases_failed.push("implement".to_string());
                            error!("Implement phase failed: {}", e);
                        }
                    }
                    self.save_state().await?;
                } else {
                    phases_completed.push("implement".to_string());
                }

                // Check flow timeout
                if start_time.elapsed() > max_duration {
                    error!(
                        "Flow exceeded maximum duration of {} seconds",
                        self.state.config.max_flow_duration_secs
                    );
                } else {
                    // Execute QA phase (reviews implementation)
                    if !self.is_phase_completed("qa") && phases_failed.is_empty() {
                        match self.execute_phase_with_retry("qa").await {
                            Ok(_) => {
                                phases_completed.push("qa".to_string());
                                if let Err(e) = self.create_phase_checkpoint("qa") {
                                    warn!("Failed to create git checkpoint for qa phase: {}", e);
                                }
                            }
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
                        if phases_failed.is_empty()
                            || phases_completed.contains(&"implement".to_string())
                        {
                            match self.execute_phase_with_retry("summarize").await {
                                Ok(_) => {
                                    phases_completed.push("summarize".to_string());
                                    if let Err(e) = self.create_phase_checkpoint("summarize") {
                                        warn!("Failed to create git checkpoint for summarize phase: {}", e);
                                    }
                                    // Set end commit
                                    if let Ok(commit) = crate::flow_git::FlowGit::new(&self.working_dir).current_commit() {
                                        self.state.git.end_commit = Some(commit);
                                    }
                                }
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

    /// Execute phase with timeout wrapping
    async fn execute_phase_with_timeout(&mut self, phase: &str) -> Result<()> {
        let timeout_duration = Duration::from_secs(self.state.config.phase_timeout_secs);

        match timeout(timeout_duration, self.execute_phase(phase)).await {
            Ok(result) => result,
            Err(_) => {
                // Phase timed out
                self.update_phase_status(phase, PhaseStatus::Failed);
                self.set_phase_error(
                    phase,
                    &format!(
                        "Phase timed out after {} seconds",
                        self.state.config.phase_timeout_secs
                    ),
                );
                Err(anyhow::anyhow!(
                    "Phase '{}' timed out after {} seconds",
                    phase,
                    self.state.config.phase_timeout_secs
                ))
            }
        }
    }

    /// Execute phase with retry support
    async fn execute_phase_with_retry(&mut self, phase: &str) -> Result<()> {
        let max_retries = self.state.config.max_retries_per_phase;

        loop {
            // Check if we've exceeded max retries
            let retry_count = self.get_phase_retry_count(phase);
            if retry_count >= max_retries {
                return Err(anyhow::anyhow!(
                    "Phase '{}' failed after {} retries",
                    phase,
                    retry_count
                ));
            }

            match self.execute_phase_with_timeout(phase).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // Increment retry count
                    self.increment_phase_retry(phase);
                    self.set_phase_error(phase, &e.to_string());

                    // Ask orchestrator what to do
                    let decision = self.get_orchestrator_decision(phase, &e).await?;

                    match decision {
                        OrchestratorDecision::Retry { reason } => {
                            info!(
                                "Orchestrator decided to retry phase '{}': {}",
                                phase, reason
                            );
                            // Loop continues
                        }
                        OrchestratorDecision::Skip { reason } => {
                            info!("Orchestrator decided to skip phase '{}': {}", phase, reason);
                            self.update_phase_status(phase, PhaseStatus::Skipped);
                            return Ok(());
                        }
                        OrchestratorDecision::Abort { reason } => {
                            error!("Orchestrator decided to abort: {}", reason);
                            return Err(anyhow::anyhow!("Aborted: {}", reason));
                        }
                        OrchestratorDecision::Continue { reason } => {
                            info!("Orchestrator decided to continue: {}", reason);
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Get orchestrator decision for error handling
    async fn get_orchestrator_decision(
        &mut self,
        phase: &str,
        error: &anyhow::Error,
    ) -> Result<OrchestratorDecision> {
        info!("Invoking orchestrator for error recovery");

        // Check if orchestrator agent exists
        if !self.agent_loader.agent_exists("flow-orchestrator") {
            warn!("Orchestrator agent not found, defaulting to abort");
            return Ok(OrchestratorDecision::Abort {
                reason: "Orchestrator agent not found".to_string(),
            });
        }

        let context = WorkflowContext::new(
            self.working_dir.clone(),
            self.state.tag.as_deref().unwrap_or("flow"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create workflow context: {}", e))?;

        let retry_count = self.get_phase_retry_count(phase);

        let task = format!(
            r#"Phase '{}' failed with error: {}

Retry count: {} of {}
Flow tag: {}
Current phase: {}

Analyze the error and decide the best course of action.

Respond with:
Decision: <retry|skip|abort>
Reason: <brief explanation>
"#,
            phase,
            error,
            retry_count,
            self.state.config.max_retries_per_phase,
            self.state.tag.as_deref().unwrap_or("flow"),
            phase
        );

        let step = WorkflowStep {
            name: "Flow: Error Recovery".to_string(),
            agent: "flow-orchestrator".to_string(),
            task: task.clone(),
            parallel: false,
            output: None,
        };

        let config = WorkflowExecutorConfig::default();
        let result = execute_step(&step, &task, &context, self.backend.as_ref(), &config)
            .await
            .map_err(|e| anyhow::anyhow!("Orchestrator execution failed: {}", e))?;

        Ok(OrchestratorDecision::parse(&result.output))
    }

    /// Get retry count for a phase
    fn get_phase_retry_count(&self, phase: &str) -> u32 {
        match phase {
            "ingest" => self.state.phases.ingest.retry_count,
            "review_graph" => self.state.phases.review_graph.retry_count,
            "plan_tasks" => self.state.phases.plan_tasks.retry_count,
            "implement" => self.state.phases.implement.retry_count,
            "qa" => self.state.phases.qa.retry_count,
            "summarize" => self.state.phases.summarize.retry_count,
            _ => 0,
        }
    }

    /// Increment retry count for a phase
    fn increment_phase_retry(&mut self, phase: &str) {
        let phase_state = match phase {
            "ingest" => &mut self.state.phases.ingest,
            "review_graph" => &mut self.state.phases.review_graph,
            "plan_tasks" => &mut self.state.phases.plan_tasks,
            "implement" => &mut self.state.phases.implement,
            "qa" => &mut self.state.phases.qa,
            "summarize" => &mut self.state.phases.summarize,
            _ => return,
        };
        phase_state.retry_count += 1;
    }

    /// Set error message for a phase
    fn set_phase_error(&mut self, phase: &str, error: &str) {
        let phase_state = match phase {
            "ingest" => &mut self.state.phases.ingest,
            "review_graph" => &mut self.state.phases.review_graph,
            "plan_tasks" => &mut self.state.phases.plan_tasks,
            "implement" => &mut self.state.phases.implement,
            "qa" => &mut self.state.phases.qa,
            "summarize" => &mut self.state.phases.summarize,
            _ => return,
        };
        phase_state.last_error = Some(error.to_string());
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

    /// Initialize git state at flow start
    fn init_git_state(&mut self) -> Result<()> {
        if !self.state.config.auto_commit {
            return Ok(());
        }

        let git = crate::flow_git::FlowGit::new(&self.working_dir);

        self.state.git.start_commit = Some(git.current_commit()?);
        self.state.git.branch = git.current_branch()?;

        Ok(())
    }

    /// Create checkpoint after phase completion
    fn create_phase_checkpoint(&mut self, phase: &str) -> Result<()> {
        if !self.state.config.auto_commit {
            return Ok(());
        }

        let git = crate::flow_git::FlowGit::new(&self.working_dir);
        let tag = self.state.tag.as_deref().unwrap_or("flow");

        let commit = git.create_checkpoint(phase, tag)?;
        self.state
            .git
            .phase_checkpoints
            .insert(phase.to_string(), commit.clone());

        info!(
            "Created git checkpoint for phase '{}': {}",
            phase,
            crate::flow_git::FlowGit::short_hash(&commit)
        );
        Ok(())
    }

    /// Rollback to checkpoint from specific phase
    pub fn rollback_to_phase(&mut self, phase: &str) -> Result<()> {
        let commit = self
            .state
            .git
            .phase_checkpoints
            .get(phase)
            .ok_or_else(|| anyhow::anyhow!("No checkpoint found for phase '{}'", phase))?
            .clone();

        let git = crate::flow_git::FlowGit::new(&self.working_dir);
        git.rollback(&commit)?;

        // Reset subsequent phases to pending
        self.reset_phases_after(phase);

        info!(
            "Rolled back to phase '{}' checkpoint: {}",
            phase,
            crate::flow_git::FlowGit::short_hash(&commit)
        );
        Ok(())
    }

    /// Reset phases after the given phase to pending status
    fn reset_phases_after(&mut self, phase: &str) {
        let phases = ["ingest", "review_graph", "plan_tasks", "implement", "qa", "summarize"];
        let mut found = false;

        for p in phases.iter() {
            if *p == phase {
                found = true;
                continue;
            }
            if found {
                let phase_state = match *p {
                    "ingest" => &mut self.state.phases.ingest,
                    "review_graph" => &mut self.state.phases.review_graph,
                    "plan_tasks" => &mut self.state.phases.plan_tasks,
                    "implement" => &mut self.state.phases.implement,
                    "qa" => &mut self.state.phases.qa,
                    "summarize" => &mut self.state.phases.summarize,
                    _ => continue,
                };
                phase_state.status = PhaseStatus::Pending;
                phase_state.completed_at = None;
                phase_state.retry_count = 0;
                phase_state.last_error = None;
            }
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

    /// Execute implement phase with concurrent QA monitoring
    async fn execute_implement_with_qa(&mut self) -> Result<()> {
        // Create channel for QA events
        let (qa_tx, _qa_rx) = mpsc::channel::<QALogEntry>(100);

        // Mark QA monitoring as active
        self.state.qa_monitor.active = true;

        // Clone what we need for the QA task
        let working_dir = self.working_dir.clone();
        let tag = self.state.tag.clone();
        let check_interval = self.state.config.qa_check_interval_secs;
        let backend = self.backend.clone();

        // Spawn QA monitoring task
        let qa_handle = tokio::spawn(async move {
            Self::run_qa_monitor(
                working_dir,
                tag,
                check_interval,
                backend,
                qa_tx,
            )
            .await
        });

        // Execute implement phase
        let implement_result = self.execute_phase_with_retry("implement").await;

        // Signal QA to stop by dropping the receiver side
        // The sender will fail on closed channel
        drop(_qa_rx);

        // Wait for QA to finish with a timeout
        match tokio::time::timeout(Duration::from_secs(10), qa_handle).await {
            Ok(Ok(Ok(entries))) => {
                info!("QA monitoring completed with {} entries", entries.len());
                // Add entries to state
                for entry in entries {
                    if entry.severity == "error" || entry.severity == "warning" {
                        self.state.qa_monitor.issues_found += 1;
                    }
                    self.state.qa_monitor.log_entries.push(entry);
                }
            }
            Ok(Ok(Err(e))) => warn!("QA monitoring error: {}", e),
            Ok(Err(e)) => warn!("QA task join error: {}", e),
            Err(_) => warn!("QA task timed out during shutdown"),
        }

        // Mark QA monitoring as inactive
        self.state.qa_monitor.active = false;

        // Save QA log to file
        if let Err(e) = self.save_qa_log().await {
            warn!("Failed to save QA log: {}", e);
        }

        implement_result
    }

    /// Background QA monitoring task
    async fn run_qa_monitor(
        working_dir: PathBuf,
        tag: Option<String>,
        check_interval_secs: u64,
        backend: Arc<dyn ModelBackend + Send + Sync>,
        tx: mpsc::Sender<QALogEntry>,
    ) -> Result<Vec<QALogEntry>> {
        let git = crate::flow_git::FlowGit::new(&working_dir);
        let mut last_commit = git.current_commit().unwrap_or_default();
        let check_interval = Duration::from_secs(check_interval_secs);
        let mut entries = Vec::new();

        // Create agent loader for this task
        let agent_loader = match AgentDefinitionLoader::new() {
            Ok(loader) => loader,
            Err(e) => {
                warn!("Failed to create agent loader for QA: {}", e);
                return Ok(entries);
            }
        };

        info!("Starting QA monitoring from commit {}", crate::flow_git::FlowGit::short_hash(&last_commit));

        loop {
            tokio::time::sleep(check_interval).await;

            // Check if channel is closed (implement phase finished)
            if tx.is_closed() {
                info!("QA monitoring stopping - implement phase completed");
                break;
            }

            // Check for new commits
            let current_commit = match git.current_commit() {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to get current commit: {}", e);
                    continue;
                }
            };

            if current_commit != last_commit {
                // New commit detected - run QA check
                info!(
                    "QA: New commit detected {} -> {}",
                    crate::flow_git::FlowGit::short_hash(&last_commit),
                    crate::flow_git::FlowGit::short_hash(&current_commit)
                );

                match Self::qa_check_commit(
                    &working_dir,
                    &tag,
                    &agent_loader,
                    backend.as_ref(),
                    &last_commit,
                    &current_commit,
                )
                .await
                {
                    Ok(entry) => {
                        let _ = tx.send(entry.clone()).await;
                        entries.push(entry);
                    }
                    Err(e) => {
                        warn!("QA check failed: {}", e);
                    }
                }
                last_commit = current_commit;
            }
        }

        Ok(entries)
    }

    /// Run QA check on commit diff
    async fn qa_check_commit(
        working_dir: &Path,
        tag: &Option<String>,
        agent_loader: &AgentDefinitionLoader,
        backend: &(dyn ModelBackend + Send + Sync),
        from_commit: &str,
        to_commit: &str,
    ) -> Result<QALogEntry> {
        // Get diff using git command
        let diff_output = Command::new("git")
            .args(["diff", from_commit, to_commit])
            .current_dir(working_dir)
            .output()?;

        let diff = String::from_utf8_lossy(&diff_output.stdout);

        // Limit diff size to avoid token limits
        let diff_truncated = if diff.len() > 4000 {
            format!("{}...(truncated)", &diff[..4000])
        } else {
            diff.to_string()
        };

        // Check if flow-qa agent exists
        if !agent_loader.agent_exists("flow-qa") {
            // Return a simple pass if no QA agent
            return Ok(QALogEntry {
                timestamp: Utc::now(),
                commit: to_commit.to_string(),
                severity: "info".to_string(),
                message: "QA agent not available, skipping check".to_string(),
            });
        }

        // Create workflow context
        let context = WorkflowContext::new(
            working_dir.to_path_buf(),
            tag.as_deref().unwrap_or("flow"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create workflow context: {}", e))?;

        let task = format!(
            r#"Review this commit diff for quality issues:

```diff
{}
```

Provide a brief assessment (1-2 sentences).
Format your response as:
SEVERITY: <info|warning|error>
MESSAGE: <your assessment>"#,
            diff_truncated
        );

        let step = WorkflowStep {
            name: "QA: Commit Review".to_string(),
            agent: "flow-qa".to_string(),
            task: task.clone(),
            parallel: false,
            output: None,
        };

        let config = WorkflowExecutorConfig::default();
        let result = execute_step(&step, &task, &context, backend, &config).await?;

        // Parse response
        let (severity, message) = Self::parse_qa_response(&result.output);

        Ok(QALogEntry {
            timestamp: Utc::now(),
            commit: to_commit.to_string(),
            severity,
            message,
        })
    }

    /// Parse QA response to extract severity and message
    fn parse_qa_response(output: &str) -> (String, String) {
        let lower = output.to_lowercase();

        // Try to extract structured response
        let severity = if lower.contains("severity: error") || lower.contains("severity:error") {
            "error"
        } else if lower.contains("severity: warning") || lower.contains("severity:warning") {
            "warning"
        } else {
            "info"
        }
        .to_string();

        // Extract message if present
        let message = output
            .lines()
            .find(|l| l.to_lowercase().starts_with("message:"))
            .map(|l| l.trim_start_matches("MESSAGE:").trim_start_matches("message:").trim().to_string())
            .unwrap_or_else(|| output.to_string());

        (severity, message)
    }

    /// Save QA log to .scud/qa-log.json
    async fn save_qa_log(&self) -> Result<()> {
        let qa_log_path = self.working_dir.join(".scud/qa-log.json");
        let content = serde_json::to_string_pretty(&self.state.qa_monitor)?;
        fs::write(&qa_log_path, content).await?;
        info!("Saved QA log to {:?}", qa_log_path);
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
        // New timeout fields
        assert_eq!(config.phase_timeout_secs, 1800);
        assert_eq!(config.watchdog_interval_secs, 60);
        assert_eq!(config.max_flow_duration_secs, 14400);
        assert_eq!(config.max_retries_per_phase, 3);
    }

    #[test]
    fn test_orchestrator_decision_parse_retry() {
        let output = "Decision: retry\nReason: transient error, should succeed on retry";
        let decision = OrchestratorDecision::parse(output);
        assert!(matches!(decision, OrchestratorDecision::Retry { .. }));
        if let OrchestratorDecision::Retry { reason } = decision {
            assert!(reason.contains("transient"));
        }
    }

    #[test]
    fn test_orchestrator_decision_parse_skip() {
        let output = "Decision: skip\nReason: optional phase, not needed";
        let decision = OrchestratorDecision::parse(output);
        assert!(matches!(decision, OrchestratorDecision::Skip { .. }));
    }

    #[test]
    fn test_orchestrator_decision_parse_abort() {
        let output = "Decision: abort\nReason: critical error, cannot continue";
        let decision = OrchestratorDecision::parse(output);
        assert!(matches!(decision, OrchestratorDecision::Abort { .. }));
    }

    #[test]
    fn test_orchestrator_decision_parse_continue_default() {
        let output = "Some unclear response without a clear decision keyword";
        let decision = OrchestratorDecision::parse(output);
        assert!(matches!(decision, OrchestratorDecision::Continue { .. }));
    }

    #[test]
    fn test_orchestrator_decision_parse_no_colon_space() {
        // Test parsing without space after colon
        let output = "Decision:retry\nReason: testing";
        let decision = OrchestratorDecision::parse(output);
        assert!(matches!(decision, OrchestratorDecision::Retry { .. }));
    }

    #[test]
    fn test_phase_state_default_has_retry_fields() {
        let phase_state = PhaseState::default();
        assert_eq!(phase_state.retry_count, 0);
        assert!(phase_state.last_error.is_none());
    }

    #[test]
    fn test_flow_git_state_has_phase_checkpoints() {
        let git_state = FlowGitState::default();
        assert!(git_state.phase_checkpoints.is_empty());
    }

    #[test]
    fn test_qa_log_entry_serde() {
        let entry = QALogEntry {
            timestamp: Utc::now(),
            commit: "abc123def456".to_string(),
            severity: "warning".to_string(),
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: QALogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.commit, "abc123def456");
        assert_eq!(parsed.severity, "warning");
        assert_eq!(parsed.message, "Test message");
    }

    #[test]
    fn test_qa_monitor_state_default() {
        let state = QAMonitorState::default();
        assert!(!state.active);
        assert_eq!(state.issues_found, 0);
        assert!(state.last_reviewed_commit.is_none());
        assert!(state.log_entries.is_empty());
    }

    #[test]
    fn test_flow_config_has_qa_check_interval() {
        let config = FlowConfig::default();
        assert_eq!(config.qa_check_interval_secs, 30);
    }

    #[test]
    fn test_flow_state_has_qa_monitor() {
        let state = FlowState::default();
        assert!(!state.qa_monitor.active);
        assert_eq!(state.qa_monitor.issues_found, 0);
    }

    #[test]
    fn test_parse_qa_response_error() {
        let output = "SEVERITY: error\nMESSAGE: Critical bug found in authentication";
        let (severity, message) = FlowExecutor::parse_qa_response(output);
        assert_eq!(severity, "error");
        assert!(message.contains("Critical bug"));
    }

    #[test]
    fn test_parse_qa_response_warning() {
        let output = "SEVERITY: warning\nMESSAGE: Minor code smell detected";
        let (severity, message) = FlowExecutor::parse_qa_response(output);
        assert_eq!(severity, "warning");
        assert!(message.contains("code smell"));
    }

    #[test]
    fn test_parse_qa_response_info() {
        let output = "SEVERITY: info\nMESSAGE: Code looks good";
        let (severity, message) = FlowExecutor::parse_qa_response(output);
        assert_eq!(severity, "info");
        assert!(message.contains("looks good"));
    }

    #[test]
    fn test_parse_qa_response_default_info() {
        let output = "Some random response without structured format";
        let (severity, message) = FlowExecutor::parse_qa_response(output);
        assert_eq!(severity, "info");
        assert_eq!(message, output);
    }

    #[test]
    fn test_qa_monitor_state_serde() {
        let mut state = QAMonitorState::default();
        state.active = true;
        state.issues_found = 3;
        state.log_entries.push(QALogEntry {
            timestamp: Utc::now(),
            commit: "test123".to_string(),
            severity: "warning".to_string(),
            message: "Test warning".to_string(),
        });

        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: QAMonitorState = serde_json::from_str(&json).unwrap();

        assert!(parsed.active);
        assert_eq!(parsed.issues_found, 3);
        assert_eq!(parsed.log_entries.len(), 1);
        assert_eq!(parsed.log_entries[0].severity, "warning");
    }
}
