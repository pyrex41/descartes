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
use std::time::Duration;
use tracing::{debug, info, warn};

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

        Ok(Self { config, state })
    }

    /// Resume from existing state file
    pub async fn resume(state_file: PathBuf) -> Result<Self> {
        let content = tokio::fs::read_to_string(&state_file)
            .await
            .context("Failed to read state file")?;
        let state: ScudLoopState =
            serde_json::from_str(&content).context("Failed to parse state file")?;

        Ok(Self {
            config: state.config.clone(),
            state,
        })
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

            // Execute task (placeholder - in real implementation, spawn sub-agent)
            let success = self.execute_task(&task).await?;

            if success {
                // Run verification
                let verified = self.run_verification()?;

                if verified {
                    // Mark task as done
                    self.update_task_status(task.id, "done")?;
                    self.state.tasks_completed += 1;
                    wave_task_ids.push(task.id);
                    info!("Task {} completed successfully", task.id);
                } else {
                    // Verification failed, mark as blocked
                    self.update_task_status(task.id, "blocked")?;
                    self.state.blocked_tasks.push(BlockedTask {
                        task_id: task.id,
                        title: task.title.clone(),
                        reason: "Verification failed".to_string(),
                        attempts: 1,
                        blocked_at: Utc::now(),
                    });
                    warn!("Task {} blocked: verification failed", task.id);
                }
            } else {
                // Task execution failed
                self.update_task_status(task.id, "blocked")?;
                self.state.blocked_tasks.push(BlockedTask {
                    task_id: task.id,
                    title: task.title.clone(),
                    reason: "Execution failed".to_string(),
                    attempts: 1,
                    blocked_at: Utc::now(),
                });
                warn!("Task {} blocked: execution failed", task.id);
            }

            self.state.iteration_count += 1;
            self.state.last_activity_at = Some(Utc::now());
            self.save_state().await?;
        }
    }

    /// Execute a single task
    /// In production, this would spawn a sub-agent
    async fn execute_task(&self, task: &LoopTask) -> Result<bool> {
        // For now, this is a placeholder
        // In production, we'd:
        // 1. Read the plan context for this task
        // 2. Spawn a sub-agent with task-implementer prompt
        // 3. Monitor progress
        // 4. Return success/failure

        info!(
            "Executing task {} (placeholder - sub-agent implementation pending)",
            task.id
        );

        // Simulate some work
        tokio::time::sleep(Duration::from_millis(100)).await;

        // For now, return success (actual implementation will come later)
        Ok(true)
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
}
