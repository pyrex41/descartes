//! SCUD-aware iterative loop
//!
//! Wraps the generic IterativeLoop with SCUD task tracking for objective
//! completion detection and wave-based execution.
//!
//! Key differences from base IterativeLoop:
//! - Completion detected via SCUD task states (not promise tags)
//! - Progress tracked by wave (not iteration count)
//! - Commits after each wave (not each iteration)
//! - Sub-agent spawning for task implementation

use crate::{IterativeExitReason, IterativeLoopResult};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info, warn};

/// Configuration for spec/context loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopSpecConfig {
    /// Include SCUD task details in spec (default: true)
    #[serde(default = "default_true")]
    pub include_task: bool,

    /// Include relevant plan section (default: true)
    #[serde(default = "default_true")]
    pub include_plan_section: bool,

    /// Additional spec files to include
    #[serde(default)]
    pub additional_specs: Vec<PathBuf>,

    /// Max tokens for combined spec (soft limit, will warn if exceeded)
    #[serde(default)]
    pub max_spec_tokens: Option<usize>,

    /// Custom template for combining specs
    /// Placeholders: {task}, {plan}, {custom}, {verification}
    #[serde(default)]
    pub spec_template: Option<String>,
}

impl Default for LoopSpecConfig {
    fn default() -> Self {
        Self {
            include_task: true,
            include_plan_section: true,
            additional_specs: Vec::new(),
            max_spec_tokens: Some(5000),
            spec_template: None,
        }
    }
}

/// Statistics from SCUD CLI
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScudStats {
    pub total: u32,
    pub done: u32,
    pub pending: u32,
    pub in_progress: u32,
    pub blocked: u32,
    pub expanded: u32,
}

impl ScudStats {
    /// Parse stats from scud stats output (handles both JSON and text formats)
    pub fn parse(output: &str) -> Result<Self> {
        // Try JSON first
        if let Ok(stats) = serde_json::from_str::<ScudStats>(output) {
            return Ok(stats);
        }

        // Fall back to text parsing
        // Format: "Total: 12, Done: 5, Pending: 4, In Progress: 2, Blocked: 1"
        let mut stats = ScudStats::default();
        for part in output.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                let value: u32 = value.trim().parse().unwrap_or(0);
                match key.trim().to_lowercase().as_str() {
                    "total" => stats.total = value,
                    "done" => stats.done = value,
                    "pending" => stats.pending = value,
                    "in progress" | "in_progress" => stats.in_progress = value,
                    "blocked" => stats.blocked = value,
                    "expanded" => stats.expanded = value,
                    _ => {}
                }
            }
        }
        Ok(stats)
    }

    /// Check if all work is complete
    pub fn is_complete(&self) -> bool {
        self.pending == 0 && self.in_progress == 0
    }
}

/// A single task in the SCUD loop context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopTask {
    pub id: u32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub complexity: u32,
    #[serde(default)]
    pub depends_on: Vec<u32>,
    #[serde(default)]
    pub test_strategy: Option<String>,
}

/// A wave of tasks that can be executed in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScudWave {
    pub number: u32,
    pub tasks: Vec<LoopTask>,
}

/// Configuration for SCUD-aware loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScudLoopConfig {
    /// SCUD tag for task tracking
    pub tag: String,

    /// Path to implementation plan document (for context)
    #[serde(default)]
    pub plan_path: Option<PathBuf>,

    /// Path to handoff document (for resume context)
    #[serde(default)]
    pub handoff_path: Option<PathBuf>,

    /// Maximum iterations per task (safety)
    #[serde(default = "default_max_per_task")]
    pub max_iterations_per_task: u32,

    /// Maximum total iterations across all tasks (safety)
    #[serde(default = "default_max_total")]
    pub max_total_iterations: u32,

    /// Working directory
    pub working_directory: PathBuf,

    /// Whether to spawn sub-agents per task
    #[serde(default = "default_true")]
    pub use_sub_agents: bool,

    /// Verification command to run after each task (e.g., "make check test")
    #[serde(default)]
    pub verification_command: Option<String>,

    /// Whether to auto-commit after each wave
    #[serde(default = "default_true")]
    pub auto_commit_waves: bool,

    /// State file path for persistence
    #[serde(default)]
    pub state_file: Option<PathBuf>,

    /// Spec configuration for context loading
    #[serde(default)]
    pub spec: LoopSpecConfig,

    /// Tuning configuration for "tune the guitar" feedback loop
    #[serde(default)]
    pub tune: TuneConfig,
}

fn default_max_per_task() -> u32 {
    3
}

fn default_max_total() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

/// Result of task execution by Claude agent
#[derive(Debug, Clone)]
pub enum TaskExecutionResult {
    Success,
    Blocked(String),
    Unknown,
    /// Task needs human tuning intervention
    AwaitingTune(TaskTuneState),
}

impl Default for ScudLoopConfig {
    fn default() -> Self {
        Self {
            tag: String::new(),
            plan_path: None,
            handoff_path: None,
            max_iterations_per_task: default_max_per_task(),
            max_total_iterations: default_max_total(),
            working_directory: PathBuf::from("."),
            use_sub_agents: true,
            verification_command: Some("make check test".to_string()),
            auto_commit_waves: true,
            state_file: None,
            spec: LoopSpecConfig::default(),
            tune: TuneConfig::default(),
        }
    }
}

/// State for SCUD loop (persisted between executions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScudLoopState {
    /// Schema version
    #[serde(default = "default_version")]
    pub version: String,

    /// Configuration (for resume)
    pub config: ScudLoopConfig,

    /// Current wave number (1-indexed)
    pub current_wave: u32,

    /// Total waves detected
    pub total_waves: u32,

    /// Tasks completed so far
    pub tasks_completed: u32,

    /// Total tasks
    pub tasks_total: u32,

    /// Iteration count (for safety limit)
    pub iteration_count: u32,

    /// Commit hashes per wave
    #[serde(default)]
    pub wave_commits: Vec<WaveCommit>,

    /// When the loop started
    pub started_at: DateTime<Utc>,

    /// Last activity timestamp
    #[serde(default)]
    pub last_activity_at: Option<DateTime<Utc>>,

    /// Whether loop has completed
    #[serde(default)]
    pub completed: bool,

    /// Exit reason if completed
    #[serde(default)]
    pub exit_reason: Option<IterativeExitReason>,

    /// Blocked tasks with reasons
    #[serde(default)]
    pub blocked_tasks: Vec<BlockedTask>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// Record of a wave commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveCommit {
    pub wave: u32,
    pub commit_hash: String,
    pub timestamp: DateTime<Utc>,
    pub tasks_completed: Vec<u32>,
}

/// A task that is blocked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedTask {
    pub task_id: u32,
    pub title: String,
    pub reason: String,
    pub attempts: u32,
    pub blocked_at: DateTime<Utc>,
}

/// A single attempt at executing a task (for tuning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAttempt {
    /// Attempt number (1-indexed)
    pub attempt: u32,

    /// The prompt/spec used for this attempt
    pub prompt: String,

    /// Agent output (truncated if too long)
    pub agent_output: String,

    /// Verification stdout
    pub verification_stdout: String,

    /// Verification stderr
    pub verification_stderr: String,

    /// Whether verification passed
    pub verification_passed: bool,

    /// Git diff of changes made (if any)
    pub git_diff: Option<String>,

    /// Suggested refinement for next attempt (from tuner agent)
    pub suggested_refinement: Option<String>,

    /// Timestamp
    pub attempted_at: DateTime<Utc>,
}

/// State for task tuning (persisted separately)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTuneState {
    /// Task being tuned
    pub task_id: u32,
    pub task_title: String,

    /// All attempts made
    pub attempts: Vec<TaskAttempt>,

    /// Currently selected variant (for resume)
    pub selected_variant: Option<u32>,

    /// Custom prompt override (from human)
    pub custom_prompt: Option<String>,

    /// When tuning started
    pub started_at: DateTime<Utc>,
}

/// Configuration for "tune the guitar" feedback loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneConfig {
    /// Enable automatic prompt tuning on failure
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Max auto-tune attempts before human checkpoint
    #[serde(default = "default_tune_attempts")]
    pub max_attempts: u32,

    /// Max tokens of agent output to include in tuning context
    #[serde(default = "default_tune_output_tokens")]
    pub max_output_tokens: usize,

    /// Include git diff in tuning context
    #[serde(default = "default_true")]
    pub include_git_diff: bool,

    /// Path for tune state file
    #[serde(default)]
    pub tune_state_file: Option<PathBuf>,
}

fn default_tune_attempts() -> u32 {
    3
}

fn default_tune_output_tokens() -> usize {
    2000
}

impl Default for TuneConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: default_tune_attempts(),
            max_output_tokens: default_tune_output_tokens(),
            include_git_diff: true,
            tune_state_file: None,
        }
    }
}

impl Default for ScudLoopState {
    fn default() -> Self {
        Self {
            version: default_version(),
            config: ScudLoopConfig::default(),
            current_wave: 1,
            total_waves: 0,
            tasks_completed: 0,
            tasks_total: 0,
            iteration_count: 0,
            wave_commits: Vec::new(),
            started_at: Utc::now(),
            last_activity_at: None,
            completed: false,
            exit_reason: None,
            blocked_tasks: Vec::new(),
        }
    }
}

/// SCUD-aware iterative loop executor
pub struct ScudIterativeLoop {
    config: ScudLoopConfig,
    state: ScudLoopState,
    /// Pending tune state from human intervention
    pending_tune_state: Option<TaskTuneState>,
}

impl ScudIterativeLoop {
    /// Create a new SCUD loop
    pub fn new(config: ScudLoopConfig) -> Result<Self> {
        // Get initial stats
        let stats = Self::get_scud_stats_static(&config.tag, &config.working_directory)?;
        let waves = Self::get_waves_static(&config.tag, &config.working_directory)?;

        let state = ScudLoopState {
            config: config.clone(),
            current_wave: 1,
            total_waves: waves.len() as u32,
            tasks_completed: stats.done,
            tasks_total: stats.total,
            iteration_count: 0,
            started_at: Utc::now(),
            ..Default::default()
        };

        Ok(Self {
            config,
            state,
            pending_tune_state: None,
        })
    }

    /// Resume from existing state file
    pub async fn resume(state_file: PathBuf) -> Result<Self> {
        let content = tokio::fs::read_to_string(&state_file)
            .await
            .context("Failed to read state file")?;
        let state: ScudLoopState =
            serde_json::from_str(&content).context("Failed to parse state file")?;

        let mut executor = Self {
            config: state.config.clone(),
            state,
            pending_tune_state: None,
        };

        // Check for tune state
        executor.load_tune_state().await?;

        Ok(executor)
    }

    /// Load tune state if exists
    async fn load_tune_state(&mut self) -> Result<()> {
        let tune_path = self
            .config
            .tune
            .tune_state_file
            .clone()
            .unwrap_or_else(|| self.config.working_directory.join(".scud/tune-state.json"));

        if tune_path.exists() {
            let content = tokio::fs::read_to_string(&tune_path).await?;
            let tune_state: TaskTuneState = serde_json::from_str(&content)?;
            info!(
                "Loaded tune state for task {}: selected_variant={:?}, custom_prompt={}",
                tune_state.task_id,
                tune_state.selected_variant,
                tune_state.custom_prompt.is_some()
            );
            self.pending_tune_state = Some(tune_state);
        }

        Ok(())
    }

    /// Clear tune state file
    async fn clear_tune_state(&self) -> Result<()> {
        let tune_path = self
            .config
            .tune
            .tune_state_file
            .clone()
            .unwrap_or_else(|| self.config.working_directory.join(".scud/tune-state.json"));

        if tune_path.exists() {
            tokio::fs::remove_file(&tune_path).await?;
            info!("Cleared tune state file");
        }

        Ok(())
    }

    /// Get current wave number
    pub fn current_wave(&self) -> u32 {
        self.state.current_wave
    }

    /// Get iteration count
    pub fn iteration_count(&self) -> u32 {
        self.state.iteration_count
    }

    /// Check if loop is complete via SCUD stats
    pub fn is_complete(&self) -> Result<bool> {
        let stats = self.get_scud_stats()?;
        Ok(stats.is_complete())
    }

    /// Get SCUD statistics for tag
    fn get_scud_stats(&self) -> Result<ScudStats> {
        Self::get_scud_stats_static(&self.config.tag, &self.config.working_directory)
    }

    fn get_scud_stats_static(tag: &str, working_dir: &PathBuf) -> Result<ScudStats> {
        let output = Command::new("scud")
            .args(["stats", "--tag", tag])
            .current_dir(working_dir)
            .output()
            .context("Failed to run scud stats")?;

        if !output.status.success() {
            // If scud isn't available, try reading from JSON file directly
            let tasks_file = working_dir.join(".scud/tasks").join(format!("{}.json", tag));
            if tasks_file.exists() {
                let content = std::fs::read_to_string(&tasks_file)?;
                let data: serde_json::Value = serde_json::from_str(&content)?;

                let mut stats = ScudStats::default();
                if let Some(tasks) = data.get("tasks").and_then(|t| t.as_array()) {
                    stats.total = tasks.len() as u32;
                    for task in tasks {
                        match task.get("status").and_then(|s| s.as_str()).unwrap_or("pending") {
                            "done" => stats.done += 1,
                            "pending" => stats.pending += 1,
                            "in-progress" | "in_progress" => stats.in_progress += 1,
                            "blocked" => stats.blocked += 1,
                            "expanded" => stats.expanded += 1,
                            _ => stats.pending += 1,
                        }
                    }
                }
                return Ok(stats);
            }
            anyhow::bail!("scud stats failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        ScudStats::parse(&stdout)
    }

    /// Get all waves for tag
    fn get_waves(&self) -> Result<Vec<ScudWave>> {
        Self::get_waves_static(&self.config.tag, &self.config.working_directory)
    }

    fn get_waves_static(tag: &str, working_dir: &PathBuf) -> Result<Vec<ScudWave>> {
        // Try reading from JSON file directly
        let tasks_file = working_dir.join(".scud/tasks").join(format!("{}.json", tag));
        if tasks_file.exists() {
            let content = std::fs::read_to_string(&tasks_file)?;
            let data: serde_json::Value = serde_json::from_str(&content)?;

            if let Some(waves_data) = data.get("waves").and_then(|w| w.as_array()) {
                let tasks: Vec<LoopTask> = data
                    .get("tasks")
                    .and_then(|t| serde_json::from_value(t.clone()).ok())
                    .unwrap_or_default();

                let mut waves = Vec::new();
                for wave_data in waves_data {
                    let number = wave_data
                        .get("number")
                        .and_then(|n| n.as_u64())
                        .unwrap_or(0) as u32;
                    let task_ids: Vec<u32> = wave_data
                        .get("task_ids")
                        .and_then(|ids| ids.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|id| id.as_u64().map(|n| n as u32))
                                .collect()
                        })
                        .unwrap_or_default();

                    let wave_tasks: Vec<LoopTask> = tasks
                        .iter()
                        .filter(|t| task_ids.contains(&t.id))
                        .cloned()
                        .collect();

                    waves.push(ScudWave {
                        number,
                        tasks: wave_tasks,
                    });
                }
                return Ok(waves);
            }
        }

        // Fall back to empty
        Ok(Vec::new())
    }

    /// Get next pending task
    fn get_next_task(&self) -> Result<Option<LoopTask>> {
        let waves = self.get_waves()?;

        // Find the current wave
        for wave in waves {
            if wave.number >= self.state.current_wave {
                // Find first pending task in this wave
                for task in wave.tasks {
                    if task.status == "pending" {
                        return Ok(Some(task));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Update task status in the JSON file
    fn update_task_status(&self, task_id: u32, new_status: &str) -> Result<()> {
        let tasks_file = self
            .config
            .working_directory
            .join(".scud/tasks")
            .join(format!("{}.json", self.config.tag));

        if tasks_file.exists() {
            let content = std::fs::read_to_string(&tasks_file)?;
            let mut data: serde_json::Value = serde_json::from_str(&content)?;

            if let Some(tasks) = data.get_mut("tasks").and_then(|t| t.as_array_mut()) {
                for task in tasks {
                    if task.get("id").and_then(|id| id.as_u64()) == Some(task_id as u64) {
                        task["status"] = serde_json::Value::String(new_status.to_string());
                        break;
                    }
                }
            }

            let updated = serde_json::to_string_pretty(&data)?;
            std::fs::write(&tasks_file, updated)?;
        }

        Ok(())
    }

    /// Commit current wave
    fn commit_wave(&mut self, completed_task_ids: Vec<u32>) -> Result<String> {
        let message = format!(
            "feat({}): complete wave {}",
            self.config.tag, self.state.current_wave
        );

        // Git add all changes
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.config.working_directory)
            .output()
            .context("Failed to git add")?;

        // Git commit
        let output = Command::new("git")
            .args(["commit", "-m", &message])
            .current_dir(&self.config.working_directory)
            .output()
            .context("Failed to git commit")?;

        if !output.status.success() {
            // No changes to commit is ok
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("nothing to commit") {
                warn!("Git commit warning: {}", stderr);
            }
            return Ok(String::new());
        }

        // Get commit hash
        let hash_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.config.working_directory)
            .output()
            .context("Failed to get commit hash")?;

        let hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        // Record the commit
        self.state.wave_commits.push(WaveCommit {
            wave: self.state.current_wave,
            commit_hash: hash.clone(),
            timestamp: Utc::now(),
            tasks_completed: completed_task_ids,
        });

        info!("Committed wave {}: {}", self.state.current_wave, hash);
        Ok(hash)
    }

    /// Run verification command
    fn run_verification(&self) -> Result<bool> {
        if let Some(ref cmd) = self.config.verification_command {
            info!("Running verification: {}", cmd);

            let output = Command::new("sh")
                .args(["-c", cmd])
                .current_dir(&self.config.working_directory)
                .output()
                .context("Failed to run verification command")?;

            if !output.status.success() {
                warn!(
                    "Verification failed:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Save state to file
    async fn save_state(&self) -> Result<()> {
        let path = self
            .config
            .state_file
            .clone()
            .unwrap_or_else(|| {
                self.config
                    .working_directory
                    .join(".scud/loop-state.json")
            });

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&self.state)?;
        tokio::fs::write(&path, content).await?;
        debug!("Saved state to {:?}", path);
        Ok(())
    }

    /// Execute the SCUD-aware loop
    pub async fn execute(&mut self) -> Result<IterativeLoopResult> {
        let start_time = std::time::Instant::now();
        let mut wave_task_ids: Vec<u32> = Vec::new();

        info!(
            "Starting SCUD loop for tag '{}' with {} tasks in {} waves",
            self.config.tag, self.state.tasks_total, self.state.total_waves
        );

        loop {
            // Check completion
            if self.is_complete()? {
                info!("All SCUD tasks completed!");

                // Final commit if needed
                if self.config.auto_commit_waves && !wave_task_ids.is_empty() {
                    self.commit_wave(wave_task_ids.clone())?;
                }

                self.state.completed = true;
                self.state.exit_reason = Some(IterativeExitReason::CompletionPromiseDetected);
                self.save_state().await?;

                return Ok(IterativeLoopResult {
                    iterations_completed: self.state.iteration_count,
                    completion_promise_found: true,
                    completion_text: Some("All SCUD tasks completed".to_string()),
                    final_output: format!(
                        "Completed {} tasks across {} waves",
                        self.state.tasks_completed, self.state.current_wave
                    ),
                    exit_reason: IterativeExitReason::CompletionPromiseDetected,
                    total_duration: start_time.elapsed(),
                });
            }

            // Check iteration limit
            if self.state.iteration_count >= self.config.max_total_iterations {
                warn!("Max iterations reached: {}", self.state.iteration_count);
                self.state.exit_reason = Some(IterativeExitReason::MaxIterationsReached);
                self.save_state().await?;

                return Ok(IterativeLoopResult {
                    iterations_completed: self.state.iteration_count,
                    completion_promise_found: false,
                    completion_text: None,
                    final_output: format!(
                        "Max iterations ({}) reached",
                        self.config.max_total_iterations
                    ),
                    exit_reason: IterativeExitReason::MaxIterationsReached,
                    total_duration: start_time.elapsed(),
                });
            }

            // Get next task
            let task = match self.get_next_task()? {
                Some(t) => t,
                None => {
                    // No more pending tasks in current wave
                    if self.config.auto_commit_waves && !wave_task_ids.is_empty() {
                        self.commit_wave(wave_task_ids.clone())?;
                        wave_task_ids.clear();
                    }

                    // Move to next wave
                    self.state.current_wave += 1;
                    info!("Moving to wave {}", self.state.current_wave);

                    // Check if we've exceeded total waves
                    if self.state.current_wave > self.state.total_waves {
                        // All waves complete
                        continue;
                    }
                    continue;
                }
            };

            info!(
                "Processing task {}: {} [complexity: {}]",
                task.id, task.title, task.complexity
            );

            // Mark task as in-progress
            self.update_task_status(task.id, "in-progress")?;

            // Execute task with sub-agent (with tuning if enabled)
            let result = self.execute_task_with_tuning(&task).await?;

            match result {
                TaskExecutionResult::Success => {
                    // Run verification
                    let verified = self.run_verification()?;
                    if verified {
                        self.update_task_status(task.id, "done")?;
                        self.state.tasks_completed += 1;
                        wave_task_ids.push(task.id);
                        info!("Task {} completed successfully", task.id);
                    } else {
                        // Verification failed despite agent claiming success
                        self.update_task_status(task.id, "blocked")?;
                        self.state.blocked_tasks.push(BlockedTask {
                            task_id: task.id,
                            title: task.title.clone(),
                            reason: "Verification failed after agent reported success".to_string(),
                            attempts: 1,
                            blocked_at: Utc::now(),
                        });
                        warn!("Task {} blocked: verification failed", task.id);
                    }
                }
                TaskExecutionResult::Blocked(reason) => {
                    self.update_task_status(task.id, "blocked")?;
                    self.state.blocked_tasks.push(BlockedTask {
                        task_id: task.id,
                        title: task.title.clone(),
                        reason,
                        attempts: 1,
                        blocked_at: Utc::now(),
                    });
                    warn!("Task {} blocked", task.id);
                }
                TaskExecutionResult::Unknown => {
                    // Agent didn't signal clearly - run verification to decide
                    let verified = self.run_verification()?;
                    if verified {
                        self.update_task_status(task.id, "done")?;
                        self.state.tasks_completed += 1;
                        wave_task_ids.push(task.id);
                    } else {
                        self.update_task_status(task.id, "blocked")?;
                        self.state.blocked_tasks.push(BlockedTask {
                            task_id: task.id,
                            title: task.title.clone(),
                            reason: "Unknown result and verification failed".to_string(),
                            attempts: 1,
                            blocked_at: Utc::now(),
                        });
                    }
                }
                TaskExecutionResult::AwaitingTune(tune_state) => {
                    // Pause loop for human intervention
                    self.update_task_status(task.id, "awaiting-tune")?;
                    self.state.exit_reason = Some(IterativeExitReason::AwaitingHumanTune);
                    self.save_state().await?;

                    return Ok(IterativeLoopResult {
                        iterations_completed: self.state.iteration_count,
                        completion_promise_found: false,
                        completion_text: None,
                        final_output: format!(
                            "Task {} awaiting human tune. Run `descartes loop tune` to review {} variants.",
                            task.id, tune_state.attempts.len()
                        ),
                        exit_reason: IterativeExitReason::AwaitingHumanTune,
                        total_duration: start_time.elapsed(),
                    });
                }
            }

            self.state.iteration_count += 1;
            self.state.last_activity_at = Some(Utc::now());
            self.save_state().await?;
        }
    }

    /// Build the spec/context for a task
    fn build_task_spec(&self, task: &LoopTask) -> Result<String> {
        let mut parts: Vec<String> = Vec::new();

        // 1. Task details from SCUD
        if self.config.spec.include_task {
            parts.push(self.format_task_spec(task));
        }

        // 2. Plan section (find relevant section from plan doc)
        if self.config.spec.include_plan_section {
            if let Some(ref plan_path) = self.config.plan_path {
                if let Ok(plan_section) = self.extract_plan_section(plan_path, task.id) {
                    parts.push(plan_section);
                }
            }
        }

        // 3. Additional spec files
        for spec_path in &self.config.spec.additional_specs {
            if let Ok(content) = std::fs::read_to_string(spec_path) {
                parts.push(format!(
                    "## {}\n\n{}",
                    spec_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    content
                ));
            }
        }

        // 4. Apply template or default formatting
        let spec = parts.join("\n\n---\n\n");

        // 5. Warn if exceeds token budget
        if let Some(max_tokens) = self.config.spec.max_spec_tokens {
            let estimated_tokens = spec.len() / 4; // rough estimate
            if estimated_tokens > max_tokens {
                warn!(
                    "Spec exceeds token budget: ~{} tokens (max: {})",
                    estimated_tokens, max_tokens
                );
            }
        }

        Ok(spec)
    }

    fn format_task_spec(&self, task: &LoopTask) -> String {
        format!(
            "# Current Task\n\n\
            **ID:** {}\n\
            **Title:** {}\n\
            **Complexity:** {}\n\n\
            ## Description\n\n{}\n\n\
            ## Test Strategy\n\n{}\n\n\
            ## Dependencies\n\n{}",
            task.id,
            task.title,
            task.complexity,
            task.description.as_deref().unwrap_or("No description"),
            task.test_strategy
                .as_deref()
                .unwrap_or("No test strategy defined"),
            if task.depends_on.is_empty() {
                "None".to_string()
            } else {
                task.depends_on
                    .iter()
                    .map(|d| format!("- Task {}", d))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        )
    }

    fn extract_plan_section(&self, plan_path: &PathBuf, task_id: u32) -> Result<String> {
        let content = std::fs::read_to_string(plan_path)?;

        // Try to find section matching task ID
        // Look for patterns like "## Task 5:" or "### 5." or "#### Task 5"
        let patterns = [
            format!("## Task {}:", task_id),
            format!("### {}.", task_id),
            format!("#### Task {}", task_id),
            format!("## {}", task_id),
        ];

        for pattern in &patterns {
            if let Some(start) = content.find(pattern) {
                // Find next section header or end
                let section_content = &content[start..];
                let end = section_content[pattern.len()..]
                    .find("\n## ")
                    .or_else(|| section_content[pattern.len()..].find("\n### "))
                    .map(|e| e + pattern.len())
                    .unwrap_or(section_content.len());

                return Ok(format!(
                    "# Relevant Plan Section\n\n{}",
                    &section_content[..end].trim()
                ));
            }
        }

        // Fallback: return truncated plan
        Ok(format!(
            "# Implementation Plan (truncated)\n\n{}...",
            &content.chars().take(2000).collect::<String>()
        ))
    }

    /// Build the prompt for a task execution by a Claude agent
    fn build_task_prompt(&self, spec: &str, task: &LoopTask) -> Result<String> {
        let verification = self
            .config
            .verification_command
            .as_deref()
            .unwrap_or("echo 'No verification command configured'");

        Ok(format!(
            "You are implementing SCUD task {} for tag '{}'.\n\n\
            ## Spec\n\n{}\n\n\
            ## Verification\n\n\
            After implementation, run:\n```bash\n{}\n```\n\n\
            ## Instructions\n\n\
            1. Implement the task\n\
            2. Run verification\n\
            3. If verification passes, output: TASK_COMPLETE\n\
            4. If blocked after 3 attempts, output: TASK_BLOCKED: <reason>\n\n\
            Begin implementation.",
            task.id, self.config.tag, spec, verification
        ))
    }

    /// Spawn a Claude agent with the given prompt
    async fn spawn_claude_agent(&self, prompt: &str) -> Result<String> {
        let mut cmd = TokioCommand::new("claude");
        cmd.args(["-p", "--output-format", "text"])
            .arg(prompt)
            .current_dir(&self.config.working_directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = cmd
            .output()
            .await
            .context("Failed to spawn Claude agent")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            debug!("Agent stderr: {}", stderr);
        }

        Ok(stdout)
    }

    /// Parse the result from a Claude agent's output
    #[allow(dead_code)]
    fn parse_task_result(&self, output: &str, _task: &LoopTask) -> Result<TaskExecutionResult> {
        if output.contains("TASK_COMPLETE") {
            Ok(TaskExecutionResult::Success)
        } else if output.contains("TASK_BLOCKED:") {
            let reason = output
                .lines()
                .find(|l| l.contains("TASK_BLOCKED:"))
                .map(|l| l.replace("TASK_BLOCKED:", "").trim().to_string())
                .unwrap_or_else(|| "Unknown reason".to_string());
            Ok(TaskExecutionResult::Blocked(reason))
        } else {
            // No explicit signal - check if verification would pass
            Ok(TaskExecutionResult::Unknown)
        }
    }

    /// Execute a single task by spawning a sub-agent
    async fn execute_task(&self, task: &LoopTask) -> Result<TaskExecutionResult> {
        info!("Spawning sub-agent for task {}: {}", task.id, task.title);

        // 1. Build spec with fresh context
        let spec = self.build_task_spec(task)?;

        // 2. Build the prompt from template
        let prompt = self.build_task_prompt(&spec, task)?;

        // 3. Spawn Claude with the prompt
        let output = self.spawn_claude_agent(&prompt).await?;

        // 4. Parse result
        let result = self.parse_task_result(&output, task)?;

        Ok(result)
    }

    /// Execute a task with automatic tuning on failure
    async fn execute_task_with_tuning(&mut self, task: &LoopTask) -> Result<TaskExecutionResult> {
        // Check if we have a tuned prompt for this task
        if let Some(ref tune_state) = self.pending_tune_state.clone() {
            if tune_state.task_id == task.id {
                let tuned_spec = if let Some(ref custom) = tune_state.custom_prompt {
                    info!("Using custom prompt from tune state for task {}", task.id);
                    custom.clone()
                } else if let Some(variant) = tune_state.selected_variant {
                    info!("Using variant {} from tune state for task {}", variant, task.id);
                    tune_state
                        .attempts
                        .get((variant - 1) as usize)
                        .map(|a| a.prompt.clone())
                        .unwrap_or_else(|| self.build_task_spec(task).unwrap_or_default())
                } else {
                    self.build_task_spec(task)?
                };

                // Clear tune state after use
                self.pending_tune_state = None;
                self.clear_tune_state().await?;

                // Execute with tuned prompt (single attempt, verification decides success)
                let full_prompt = self.build_task_prompt(&tuned_spec, task)?;
                let output = self.spawn_claude_agent(&full_prompt).await?;
                let result = self.parse_task_result(&output, task)?;
                return Ok(result);
            }
        }

        if !self.config.tune.enabled {
            // Fall back to simple execution
            return self.execute_task(task).await;
        }

        let mut attempts: Vec<TaskAttempt> = Vec::new();
        let base_spec = self.build_task_spec(task)?;
        let mut current_spec = base_spec.clone();

        for attempt_num in 1..=self.config.tune.max_attempts {
            info!("Task {} attempt {}/{}", task.id, attempt_num, self.config.tune.max_attempts);

            // Build full prompt with any accumulated refinements
            let full_prompt = self.build_task_prompt(&current_spec, task)?;

            // Execute
            let output = self.spawn_claude_agent(&full_prompt).await?;

            // Run verification and capture details
            let (verified, stdout, stderr) = self.run_verification_detailed()?;

            // Capture git diff before potential revert
            let git_diff = if self.config.tune.include_git_diff {
                Some(self.get_git_diff()?)
            } else {
                None
            };

            let mut attempt = TaskAttempt {
                attempt: attempt_num,
                prompt: current_spec.clone(),
                agent_output: output.clone(),
                verification_stdout: stdout,
                verification_stderr: stderr.clone(),
                verification_passed: verified,
                git_diff,
                suggested_refinement: None,
                attempted_at: Utc::now(),
            };

            if verified {
                attempts.push(attempt);
                info!("Task {} succeeded on attempt {}", task.id, attempt_num);
                return Ok(TaskExecutionResult::Success);
            }

            // Failed - revert changes before retry
            self.revert_changes()?;

            // Get refinement suggestion (unless last attempt)
            if attempt_num < self.config.tune.max_attempts {
                if let Some(refinement) = self.spawn_tuner_agent(task, &attempt).await? {
                    info!("Tuner suggested refinement: {}", &refinement.chars().take(100).collect::<String>());
                    attempt.suggested_refinement = Some(refinement.clone());

                    // Apply refinement to prompt
                    current_spec = format!(
                        "{}\n\n---\n\n## Additional Guidance (from previous failure)\n\n{}",
                        base_spec, refinement
                    );
                }
            }

            attempts.push(attempt);
        }

        // All attempts exhausted - enter human tuning mode
        warn!("Task {} failed after {} attempts, awaiting human tune", task.id, self.config.tune.max_attempts);

        let tune_state = TaskTuneState {
            task_id: task.id,
            task_title: task.title.clone(),
            attempts,
            selected_variant: None,
            custom_prompt: None,
            started_at: Utc::now(),
        };

        // Save tune state
        self.save_tune_state(&tune_state).await?;

        Ok(TaskExecutionResult::AwaitingTune(tune_state))
    }

    // ========================================================================
    // "Tune the Guitar" Methods
    // ========================================================================

    /// Build prompt for the tuner agent that suggests refinements
    fn build_tuner_prompt(&self, task: &LoopTask, attempt: &TaskAttempt) -> String {
        let git_diff_section = if let Some(ref diff) = attempt.git_diff {
            if !diff.is_empty() {
                format!("### Git Diff\n```diff\n{}\n```", diff)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!(
            r#"You are analyzing a failed task execution to suggest prompt refinements.

## Original Task
**ID:** {}
**Title:** {}
**Description:** {}

## Prompt Used
```
{}
```

## What Happened
The agent attempted the task but verification failed.

### Agent Output (truncated)
```
{}
```

### Verification Error
```
{}
```

{}

## Your Job
Analyze why this failed and suggest a **specific refinement** to the prompt that would help the next attempt succeed.

Focus on:
1. What the agent misunderstood
2. Missing context that would help
3. Specific instructions to add
4. Edge cases to handle

Output your refinement as a SHORT, ACTIONABLE addition to the prompt (max 500 tokens).
Start with "REFINEMENT:" on its own line, then the text to add."#,
            task.id,
            task.title,
            task.description.as_deref().unwrap_or("No description"),
            attempt.prompt,
            &attempt.agent_output.chars().take(self.config.tune.max_output_tokens).collect::<String>(),
            attempt.verification_stderr,
            git_diff_section
        )
    }

    /// Parse refinement from tuner agent output
    fn parse_tuner_output(&self, output: &str) -> Option<String> {
        if let Some(idx) = output.find("REFINEMENT:") {
            let refinement = output[idx + 11..].trim();
            if !refinement.is_empty() {
                return Some(refinement.to_string());
            }
        }
        None
    }

    /// Spawn the tuner agent to suggest prompt refinements
    async fn spawn_tuner_agent(&self, task: &LoopTask, attempt: &TaskAttempt) -> Result<Option<String>> {
        let prompt = self.build_tuner_prompt(task, attempt);

        info!("Spawning tuner agent to analyze failure for task {}", task.id);

        let output = self.spawn_claude_agent(&prompt).await?;

        Ok(self.parse_tuner_output(&output))
    }

    /// Get git diff of uncommitted changes
    fn get_git_diff(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(&self.config.working_directory)
            .output()
            .context("Failed to get git diff")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Revert uncommitted changes (for retry)
    fn revert_changes(&self) -> Result<()> {
        Command::new("git")
            .args(["checkout", "."])
            .current_dir(&self.config.working_directory)
            .output()
            .context("Failed to revert changes")?;

        Command::new("git")
            .args(["clean", "-fd"])
            .current_dir(&self.config.working_directory)
            .output()
            .context("Failed to clean untracked files")?;

        Ok(())
    }

    /// Run verification and return detailed output
    fn run_verification_detailed(&self) -> Result<(bool, String, String)> {
        if let Some(ref cmd) = self.config.verification_command {
            let output = Command::new("sh")
                .args(["-c", cmd])
                .current_dir(&self.config.working_directory)
                .output()
                .context("Failed to run verification command")?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            Ok((output.status.success(), stdout, stderr))
        } else {
            Ok((true, String::new(), String::new()))
        }
    }

    /// Save tune state to file
    async fn save_tune_state(&self, state: &TaskTuneState) -> Result<()> {
        let path = self.config.tune.tune_state_file.clone()
            .unwrap_or_else(|| self.config.working_directory.join(".scud/tune-state.json"));

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(state)?;
        tokio::fs::write(&path, content).await?;

        info!("Saved tune state to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scud_stats_parse_text() {
        let output = "Total: 12, Done: 5, Pending: 4, In Progress: 2, Blocked: 1";
        let stats = ScudStats::parse(output).unwrap();
        assert_eq!(stats.total, 12);
        assert_eq!(stats.done, 5);
        assert_eq!(stats.pending, 4);
        assert_eq!(stats.in_progress, 2);
        assert_eq!(stats.blocked, 1);
    }

    #[test]
    fn test_scud_stats_parse_json() {
        let output = r#"{"total": 10, "done": 3, "pending": 5, "in_progress": 1, "blocked": 1, "expanded": 0}"#;
        let stats = ScudStats::parse(output).unwrap();
        assert_eq!(stats.total, 10);
        assert_eq!(stats.done, 3);
        assert_eq!(stats.pending, 5);
    }

    #[test]
    fn test_scud_stats_is_complete() {
        let mut stats = ScudStats::default();
        stats.total = 10;
        stats.done = 10;
        assert!(stats.is_complete());

        stats.pending = 1;
        stats.done = 9;
        assert!(!stats.is_complete());

        stats.pending = 0;
        stats.in_progress = 1;
        assert!(!stats.is_complete());
    }

    #[test]
    fn test_scud_loop_config_default() {
        let config = ScudLoopConfig::default();
        assert_eq!(config.max_iterations_per_task, 3);
        assert_eq!(config.max_total_iterations, 100);
        assert!(config.use_sub_agents);
        assert!(config.auto_commit_waves);
    }

    #[test]
    fn test_scud_loop_state_serialization() {
        let state = ScudLoopState::default();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ScudLoopState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.current_wave, state.current_wave);
        assert_eq!(parsed.version, "1.0");
    }

    #[test]
    fn test_wave_commit_serialization() {
        let commit = WaveCommit {
            wave: 1,
            commit_hash: "abc123".to_string(),
            timestamp: Utc::now(),
            tasks_completed: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&commit).unwrap();
        let parsed: WaveCommit = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.wave, 1);
        assert_eq!(parsed.commit_hash, "abc123");
        assert_eq!(parsed.tasks_completed, vec![1, 2, 3]);
    }

    #[test]
    fn test_blocked_task_serialization() {
        let blocked = BlockedTask {
            task_id: 5,
            title: "Test task".to_string(),
            reason: "Verification failed".to_string(),
            attempts: 3,
            blocked_at: Utc::now(),
        };
        let json = serde_json::to_string(&blocked).unwrap();
        let parsed: BlockedTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_id, 5);
        assert_eq!(parsed.attempts, 3);
    }

    #[test]
    fn test_loop_task_serialization() {
        let task = LoopTask {
            id: 7,
            title: "Implement feature X".to_string(),
            description: Some("Add new functionality".to_string()),
            status: "pending".to_string(),
            complexity: 5,
            depends_on: vec![1, 2, 3],
            test_strategy: Some("Unit tests".to_string()),
        };
        let json = serde_json::to_string(&task).unwrap();
        let parsed: LoopTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 7);
        assert_eq!(parsed.title, "Implement feature X");
        assert_eq!(parsed.complexity, 5);
        assert_eq!(parsed.depends_on, vec![1, 2, 3]);
    }

    #[test]
    fn test_scud_wave_serialization() {
        let wave = ScudWave {
            number: 2,
            tasks: vec![
                LoopTask {
                    id: 4,
                    title: "Task A".to_string(),
                    description: None,
                    status: "pending".to_string(),
                    complexity: 3,
                    depends_on: vec![],
                    test_strategy: None,
                },
                LoopTask {
                    id: 5,
                    title: "Task B".to_string(),
                    description: None,
                    status: "done".to_string(),
                    complexity: 2,
                    depends_on: vec![4],
                    test_strategy: None,
                },
            ],
        };
        let json = serde_json::to_string(&wave).unwrap();
        let parsed: ScudWave = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.number, 2);
        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[0].id, 4);
        assert_eq!(parsed.tasks[1].status, "done");
    }

    #[test]
    fn test_scud_stats_parse_empty() {
        let output = "";
        let stats = ScudStats::parse(output).unwrap();
        assert_eq!(stats.total, 0);
        assert!(stats.is_complete()); // Empty is complete
    }

    #[test]
    fn test_scud_stats_parse_partial() {
        let output = "Total: 5, Done: 3";
        let stats = ScudStats::parse(output).unwrap();
        assert_eq!(stats.total, 5);
        assert_eq!(stats.done, 3);
        assert_eq!(stats.pending, 0);
        assert!(stats.is_complete()); // No pending or in_progress
    }

    #[test]
    fn test_scud_stats_parse_json_with_defaults() {
        // JSON parsing requires all fields defined in the struct due to serde defaults
        // Partial JSON falls back to text parsing which returns 0s
        // This tests the full JSON format with explicit zeros
        let output = r#"{"total": 8, "done": 2, "pending": 0, "in_progress": 0, "blocked": 0, "expanded": 0}"#;
        let stats = ScudStats::parse(output).unwrap();
        assert_eq!(stats.total, 8);
        assert_eq!(stats.done, 2);
        assert_eq!(stats.pending, 0);
    }

    #[test]
    fn test_scud_loop_config_serialization() {
        let config = ScudLoopConfig {
            tag: "test-tag".to_string(),
            plan_path: Some(PathBuf::from("/path/to/plan.md")),
            handoff_path: None,
            max_iterations_per_task: 5,
            max_total_iterations: 50,
            working_directory: PathBuf::from("/project"),
            use_sub_agents: false,
            verification_command: Some("cargo test".to_string()),
            auto_commit_waves: false,
            state_file: Some(PathBuf::from("/state.json")),
            spec: LoopSpecConfig::default(),
            tune: TuneConfig::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ScudLoopConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tag, "test-tag");
        assert_eq!(parsed.max_iterations_per_task, 5);
        assert!(!parsed.use_sub_agents);
        assert!(!parsed.auto_commit_waves);
    }

    #[test]
    fn test_scud_loop_state_with_commits() {
        let mut state = ScudLoopState::default();
        state.wave_commits.push(WaveCommit {
            wave: 1,
            commit_hash: "abc123".to_string(),
            timestamp: Utc::now(),
            tasks_completed: vec![1, 2],
        });
        state.wave_commits.push(WaveCommit {
            wave: 2,
            commit_hash: "def456".to_string(),
            timestamp: Utc::now(),
            tasks_completed: vec![3, 4, 5],
        });

        let json = serde_json::to_string(&state).unwrap();
        let parsed: ScudLoopState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.wave_commits.len(), 2);
        assert_eq!(parsed.wave_commits[0].commit_hash, "abc123");
        assert_eq!(parsed.wave_commits[1].tasks_completed.len(), 3);
    }

    #[test]
    fn test_scud_loop_state_with_blocked_tasks() {
        let mut state = ScudLoopState::default();
        state.blocked_tasks.push(BlockedTask {
            task_id: 10,
            title: "Blocked task".to_string(),
            reason: "Tests failed".to_string(),
            attempts: 2,
            blocked_at: Utc::now(),
        });

        let json = serde_json::to_string(&state).unwrap();
        let parsed: ScudLoopState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.blocked_tasks.len(), 1);
        assert_eq!(parsed.blocked_tasks[0].reason, "Tests failed");
    }

    // Helper function to create a minimal test loop instance
    fn create_test_loop() -> ScudIterativeLoop {
        let config = ScudLoopConfig {
            tag: "test-tag".to_string(),
            plan_path: None,
            handoff_path: None,
            max_iterations_per_task: 3,
            max_total_iterations: 100,
            working_directory: PathBuf::from("/tmp/test"),
            use_sub_agents: false,
            verification_command: Some("cargo check && cargo test".to_string()),
            auto_commit_waves: false,
            state_file: None,
            spec: LoopSpecConfig::default(),
            tune: TuneConfig::default(),
        };

        let state = ScudLoopState {
            config: config.clone(),
            current_wave: 1,
            total_waves: 1,
            tasks_completed: 0,
            tasks_total: 1,
            iteration_count: 0,
            started_at: Utc::now(),
            ..Default::default()
        };

        ScudIterativeLoop {
            config,
            state,
            pending_tune_state: None,
        }
    }

    #[test]
    fn test_spec_config_defaults() {
        let config = LoopSpecConfig::default();
        assert!(config.include_task);
        assert!(config.include_plan_section);
        assert_eq!(config.max_spec_tokens, Some(5000));
    }

    #[test]
    fn test_format_task_spec() {
        let task = LoopTask {
            id: 1,
            title: "Test task".to_string(),
            description: Some("Do the thing".to_string()),
            status: "pending".to_string(),
            complexity: 3,
            depends_on: vec![],
            test_strategy: Some("Unit tests".to_string()),
        };

        let loop_exec = create_test_loop();
        let spec = loop_exec.format_task_spec(&task);

        assert!(spec.contains("Test task"));
        assert!(spec.contains("Do the thing"));
        assert!(spec.contains("Unit tests"));
    }

    #[test]
    fn test_build_task_prompt() {
        let task = LoopTask {
            id: 5,
            title: "Test task".to_string(),
            description: Some("Implement feature".to_string()),
            status: "pending".to_string(),
            complexity: 2,
            depends_on: vec![],
            test_strategy: Some("Run tests".to_string()),
        };
        let loop_exec = create_test_loop();
        let spec = "Test spec content";

        let prompt = loop_exec.build_task_prompt(spec, &task).unwrap();

        assert!(prompt.contains("Test spec content"));
        assert!(prompt.contains("TASK_COMPLETE"));
        assert!(prompt.contains("TASK_BLOCKED"));
        assert!(prompt.contains("cargo check && cargo test"));
    }

    #[test]
    fn test_parse_task_result_success() {
        let loop_exec = create_test_loop();
        let task = LoopTask {
            id: 1,
            title: "Test".to_string(),
            description: None,
            status: "pending".to_string(),
            complexity: 1,
            depends_on: vec![],
            test_strategy: None,
        };

        let output = "Some work was done.\nTASK_COMPLETE\nAll tests passed.";
        let result = loop_exec.parse_task_result(output, &task).unwrap();

        match result {
            TaskExecutionResult::Success => {},
            _ => panic!("Expected Success result"),
        }
    }

    #[test]
    fn test_parse_task_result_blocked() {
        let loop_exec = create_test_loop();
        let task = LoopTask {
            id: 1,
            title: "Test".to_string(),
            description: None,
            status: "pending".to_string(),
            complexity: 1,
            depends_on: vec![],
            test_strategy: None,
        };

        let output = "Attempted implementation.\nTASK_BLOCKED: missing API credentials\nCannot proceed.";
        let result = loop_exec.parse_task_result(output, &task).unwrap();

        match result {
            TaskExecutionResult::Blocked(reason) => {
                assert!(reason.contains("missing API credentials"));
            },
            _ => panic!("Expected Blocked result"),
        }
    }

    #[test]
    fn test_parse_task_result_unknown() {
        let loop_exec = create_test_loop();
        let task = LoopTask {
            id: 1,
            title: "Test".to_string(),
            description: None,
            status: "pending".to_string(),
            complexity: 1,
            depends_on: vec![],
            test_strategy: None,
        };

        let output = "Did some work. No clear signal here. Maybe it worked?";
        let result = loop_exec.parse_task_result(output, &task).unwrap();

        match result {
            TaskExecutionResult::Unknown => {},
            _ => panic!("Expected Unknown result"),
        }
    }

    // ========================================================================
    // Phase 2: Tuner Agent Tests
    // ========================================================================

    #[test]
    fn test_parse_tuner_output_success() {
        let loop_exec = create_test_loop();
        let output = r#"Looking at the error, it seems the function was called with wrong arguments.

REFINEMENT:
Add explicit type hints to the function signature and ensure the return type matches the expected interface. Also handle the edge case when input is empty."#;

        let refinement = loop_exec.parse_tuner_output(output);
        assert!(refinement.is_some());
        let r = refinement.unwrap();
        assert!(r.contains("Add explicit type hints"));
        assert!(r.contains("edge case"));
    }

    #[test]
    fn test_parse_tuner_output_no_refinement() {
        let loop_exec = create_test_loop();
        let output = "Analysis complete. No clear fix found.";

        let refinement = loop_exec.parse_tuner_output(output);
        assert!(refinement.is_none());
    }

    #[test]
    fn test_parse_tuner_output_empty_refinement() {
        let loop_exec = create_test_loop();
        let output = "REFINEMENT:   \n\n";

        let refinement = loop_exec.parse_tuner_output(output);
        assert!(refinement.is_none());
    }

    #[test]
    fn test_build_tuner_prompt() {
        let loop_exec = create_test_loop();
        let task = LoopTask {
            id: 42,
            title: "Implement feature X".to_string(),
            description: Some("Add the X feature to module Y".to_string()),
            status: "in-progress".to_string(),
            complexity: 3,
            depends_on: vec![],
            test_strategy: Some("Unit tests".to_string()),
        };
        let attempt = TaskAttempt {
            attempt: 1,
            prompt: "Please implement feature X".to_string(),
            agent_output: "I tried to implement it but got confused".to_string(),
            verification_stdout: String::new(),
            verification_stderr: "Error: undefined variable 'foo'".to_string(),
            verification_passed: false,
            git_diff: Some("+ let bar = 1;\n- let foo = 2;".to_string()),
            suggested_refinement: None,
            attempted_at: Utc::now(),
        };

        let prompt = loop_exec.build_tuner_prompt(&task, &attempt);

        assert!(prompt.contains("42"));
        assert!(prompt.contains("Implement feature X"));
        assert!(prompt.contains("Add the X feature"));
        assert!(prompt.contains("Please implement feature X"));
        assert!(prompt.contains("undefined variable 'foo'"));
        assert!(prompt.contains("let bar = 1"));
        assert!(prompt.contains("REFINEMENT:"));
    }

    #[test]
    fn test_tune_config_defaults() {
        let config = TuneConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.max_output_tokens, 2000);
        assert!(config.include_git_diff);
        assert!(config.tune_state_file.is_none());
    }

    #[test]
    fn test_tune_config_serialization() {
        let config = TuneConfig {
            enabled: false,
            max_attempts: 5,
            max_output_tokens: 3000,
            include_git_diff: false,
            tune_state_file: Some(PathBuf::from("/custom/tune.json")),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: TuneConfig = serde_json::from_str(&json).unwrap();
        assert!(!parsed.enabled);
        assert_eq!(parsed.max_attempts, 5);
        assert_eq!(parsed.max_output_tokens, 3000);
    }

    #[test]
    fn test_task_attempt_serialization() {
        let attempt = TaskAttempt {
            attempt: 2,
            prompt: "Test prompt".to_string(),
            agent_output: "Agent did stuff".to_string(),
            verification_stdout: "ok".to_string(),
            verification_stderr: "err".to_string(),
            verification_passed: false,
            git_diff: Some("diff content".to_string()),
            suggested_refinement: Some("Try X instead".to_string()),
            attempted_at: Utc::now(),
        };
        let json = serde_json::to_string(&attempt).unwrap();
        let parsed: TaskAttempt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.attempt, 2);
        assert_eq!(parsed.prompt, "Test prompt");
        assert_eq!(parsed.suggested_refinement, Some("Try X instead".to_string()));
    }

    #[test]
    fn test_task_tune_state_serialization() {
        let state = TaskTuneState {
            task_id: 7,
            task_title: "Tune test".to_string(),
            attempts: vec![
                TaskAttempt {
                    attempt: 1,
                    prompt: "First try".to_string(),
                    agent_output: "Failed".to_string(),
                    verification_stdout: String::new(),
                    verification_stderr: "Error".to_string(),
                    verification_passed: false,
                    git_diff: None,
                    suggested_refinement: Some("Try again".to_string()),
                    attempted_at: Utc::now(),
                },
            ],
            selected_variant: Some(1),
            custom_prompt: None,
            started_at: Utc::now(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: TaskTuneState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_id, 7);
        assert_eq!(parsed.attempts.len(), 1);
        assert_eq!(parsed.selected_variant, Some(1));
    }
}
