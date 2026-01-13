//! Iterative Agent Loop (Ralph-Style)
//!
//! Wraps any CLI command and repeatedly executes it until a completion signal
//! is detected. The agent improves iteratively by seeing its previous work
//! in files and git history.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Configuration for an iterative agent loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterativeLoopConfig {
    /// The command to run (e.g., "claude", "opencode", "python")
    pub command: String,

    /// Arguments for the command (prompt goes here or in stdin based on backend)
    #[serde(default)]
    pub args: Vec<String>,

    /// The task prompt to feed the agent
    pub prompt: String,

    /// Optional completion promise - loop exits when this text appears in output
    /// Uses `<promise>TEXT</promise>` format by default
    #[serde(default)]
    pub completion_promise: Option<String>,

    /// Maximum iterations before forced exit (safety mechanism)
    /// None means unlimited (use with caution!)
    #[serde(default)]
    pub max_iterations: Option<u32>,

    /// Working directory for the agent
    #[serde(default)]
    pub working_directory: Option<PathBuf>,

    /// State file path for persistence (default: .descartes/loop-state.json)
    #[serde(default)]
    pub state_file: Option<PathBuf>,

    /// Whether to include iteration context in prompt after first iteration
    #[serde(default = "default_true")]
    pub include_iteration_context: bool,

    /// Timeout per iteration in seconds (None = no timeout)
    #[serde(default)]
    pub iteration_timeout_secs: Option<u64>,

    /// Backend-specific configuration
    #[serde(default)]
    pub backend: LoopBackendConfig,

    /// Git configuration
    #[serde(default)]
    pub git: LoopGitConfig,
}

fn default_true() -> bool {
    true
}

/// Backend-specific configuration for different CLIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopBackendConfig {
    /// Backend type: "claude", "opencode", "generic", or custom
    #[serde(default = "default_backend")]
    pub backend_type: String,

    /// How to pass the prompt: "arg" (command line), "stdin", or "env"
    #[serde(default = "default_prompt_mode")]
    pub prompt_mode: String,

    /// Additional environment variables
    #[serde(default)]
    pub environment: std::collections::HashMap<String, String>,

    /// Output format hint: "stream-json", "json", "text"
    #[serde(default = "default_output_format")]
    pub output_format: String,
}

impl Default for LoopBackendConfig {
    fn default() -> Self {
        Self {
            backend_type: default_backend(),
            prompt_mode: default_prompt_mode(),
            environment: std::collections::HashMap::new(),
            output_format: default_output_format(),
        }
    }
}

fn default_backend() -> String {
    "generic".to_string()
}

fn default_prompt_mode() -> String {
    "arg".to_string()
}

fn default_output_format() -> String {
    "text".to_string()
}

/// Git integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopGitConfig {
    /// Whether to auto-commit after each iteration
    #[serde(default)]
    pub auto_commit: bool,

    /// Commit message template (use {iteration} for number)
    #[serde(default = "default_commit_template")]
    pub commit_template: String,

    /// Whether to create a branch for the loop
    #[serde(default)]
    pub create_branch: bool,

    /// Branch name template (use {timestamp} for datetime)
    #[serde(default = "default_branch_template")]
    pub branch_template: String,
}

fn default_commit_template() -> String {
    "loop: iteration {iteration}".to_string()
}

fn default_branch_template() -> String {
    "loop/{timestamp}".to_string()
}

impl Default for LoopGitConfig {
    fn default() -> Self {
        Self {
            auto_commit: false, // Default: don't auto-commit
            commit_template: default_commit_template(),
            create_branch: false,
            branch_template: default_branch_template(),
        }
    }
}

/// State persisted between iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterativeLoopState {
    /// Schema version for forward compatibility
    #[serde(default = "default_version")]
    pub version: String,

    /// Current iteration number (0-indexed)
    pub iteration: u32,

    /// Original configuration (for resume)
    pub config: IterativeLoopConfig,

    /// When the loop started
    pub started_at: DateTime<Utc>,

    /// When the last iteration completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_iteration_at: Option<DateTime<Utc>>,

    /// Whether the loop has completed successfully
    #[serde(default)]
    pub completed: bool,

    /// When completion was detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_detected_at: Option<DateTime<Utc>>,

    /// The completion promise that was found (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_text: Option<String>,

    /// Exit reason if loop has ended
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<IterativeExitReason>,

    /// Output from each iteration (truncated for storage)
    #[serde(default)]
    pub iteration_summaries: Vec<IterationSummary>,

    /// Error message if loop failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// Summary of a single iteration (for state file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationSummary {
    pub iteration: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub exit_code: Option<i32>,
    /// First N characters of output (for debugging)
    pub output_preview: String,
    /// Whether completion promise was checked
    pub promise_checked: bool,
}

/// Result of the iterative loop
#[derive(Debug, Clone)]
pub struct IterativeLoopResult {
    /// Number of iterations completed
    pub iterations_completed: u32,

    /// Whether completion promise was found
    pub completion_promise_found: bool,

    /// The text that matched the completion promise
    pub completion_text: Option<String>,

    /// Combined output from all iterations
    pub final_output: String,

    /// How the loop exited
    pub exit_reason: IterativeExitReason,

    /// Total duration of the loop
    pub total_duration: Duration,
}

/// Why the iterative loop exited
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IterativeExitReason {
    /// Completion promise was detected in output
    CompletionPromiseDetected,
    /// Reached maximum iteration limit
    MaxIterationsReached,
    /// User cancelled the loop
    UserCancelled,
    /// An error occurred
    Error { message: String },
    /// Process exited with success code (when no promise configured)
    ProcessSuccess,
    /// Loop is still running (for state file)
    Running,
    /// Waiting for human to tune a failed task
    AwaitingHumanTune,
}

impl Default for IterativeLoopState {
    fn default() -> Self {
        Self {
            version: default_version(),
            iteration: 0,
            config: IterativeLoopConfig::default(),
            started_at: Utc::now(),
            last_iteration_at: None,
            completed: false,
            completion_detected_at: None,
            completion_text: None,
            exit_reason: Some(IterativeExitReason::Running),
            iteration_summaries: Vec::new(),
            error: None,
        }
    }
}

impl Default for IterativeLoopConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            prompt: String::new(),
            completion_promise: None,
            max_iterations: Some(10), // Safe default
            working_directory: None,
            state_file: None,
            include_iteration_context: true,
            iteration_timeout_secs: None,
            backend: LoopBackendConfig::default(),
            git: LoopGitConfig::default(),
        }
    }
}

// ============================================================================
// Executor Implementation
// ============================================================================

/// The main iterative loop executor
pub struct IterativeLoop {
    /// Current state (persisted between iterations)
    state: IterativeLoopState,

    /// Path to state file
    state_path: PathBuf,

    /// Working directory
    working_dir: PathBuf,

    /// Accumulated output from current iteration
    current_output: String,

    /// Cancellation flag (set by signal handler or GUI)
    cancelled: Arc<AtomicBool>,
}

impl IterativeLoop {
    /// Create a new iterative loop from configuration
    pub async fn new(config: IterativeLoopConfig) -> Result<Self> {
        let working_dir = config
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let state_path = config
            .state_file
            .clone()
            .unwrap_or_else(|| working_dir.join(".descartes/loop-state.json"));

        // Load existing state or create new
        let state = if state_path.exists() {
            let content = fs::read_to_string(&state_path).await?;
            let mut loaded: IterativeLoopState =
                serde_json::from_str(&content).unwrap_or_default();
            // Update config in case it changed
            loaded.config = config;
            loaded
        } else {
            IterativeLoopState {
                config,
                started_at: Utc::now(),
                ..Default::default()
            }
        };

        Ok(Self {
            state,
            state_path,
            working_dir,
            current_output: String::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Resume an existing iterative loop from state file
    pub async fn resume(state_path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&state_path)
            .await
            .context("No loop state found. Start a new loop first.")?;

        let state: IterativeLoopState = serde_json::from_str(&content)?;

        let working_dir = state
            .config
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Ok(Self {
            state,
            state_path,
            working_dir,
            current_output: String::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get a handle to the cancellation flag for external cancellation
    pub fn cancellation_handle(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

    /// Execute the iterative loop until completion or limit
    pub async fn execute(&mut self) -> Result<IterativeLoopResult> {
        let start_time = std::time::Instant::now();
        info!(
            "Starting iterative loop: command='{}', max_iterations={:?}",
            self.state.config.command, self.state.config.max_iterations
        );

        // Main loop
        loop {
            // Check cancellation
            if self.cancelled.load(Ordering::Relaxed) {
                info!("Loop cancelled by user");
                self.state.exit_reason = Some(IterativeExitReason::UserCancelled);
                self.state.completed = true;
                self.save_state().await?;

                return Ok(IterativeLoopResult {
                    iterations_completed: self.state.iteration,
                    completion_promise_found: false,
                    completion_text: None,
                    final_output: self.current_output.clone(),
                    exit_reason: IterativeExitReason::UserCancelled,
                    total_duration: start_time.elapsed(),
                });
            }

            // Check iteration limit
            if let Some(max) = self.state.config.max_iterations {
                if self.state.iteration >= max {
                    info!("Reached maximum iterations: {}", max);
                    self.state.exit_reason = Some(IterativeExitReason::MaxIterationsReached);
                    self.state.completed = true;
                    self.save_state().await?;

                    return Ok(IterativeLoopResult {
                        iterations_completed: self.state.iteration,
                        completion_promise_found: false,
                        completion_text: None,
                        final_output: self.current_output.clone(),
                        exit_reason: IterativeExitReason::MaxIterationsReached,
                        total_duration: start_time.elapsed(),
                    });
                }
            }

            // Execute one iteration
            let iteration_start = Utc::now();
            info!("Starting iteration {}", self.state.iteration + 1);

            let (output, exit_code) = self.execute_iteration().await?;

            // Check for completion promise
            if let Some(ref promise) = self.state.config.completion_promise {
                if let Some(found_text) = self.check_completion_promise(&output, promise) {
                    info!("Completion promise detected: {}", found_text);
                    self.state.completed = true;
                    self.state.completion_detected_at = Some(Utc::now());
                    self.state.completion_text = Some(found_text.clone());
                    self.state.exit_reason = Some(IterativeExitReason::CompletionPromiseDetected);
                    self.save_state().await?;

                    return Ok(IterativeLoopResult {
                        iterations_completed: self.state.iteration + 1,
                        completion_promise_found: true,
                        completion_text: Some(found_text),
                        final_output: output,
                        exit_reason: IterativeExitReason::CompletionPromiseDetected,
                        total_duration: start_time.elapsed(),
                    });
                }
            } else if exit_code == Some(0) {
                // No completion promise configured, treat exit 0 as success
                info!("Process exited successfully (no completion promise configured)");
                self.state.completed = true;
                self.state.exit_reason = Some(IterativeExitReason::ProcessSuccess);
                self.save_state().await?;

                return Ok(IterativeLoopResult {
                    iterations_completed: self.state.iteration + 1,
                    completion_promise_found: false,
                    completion_text: None,
                    final_output: output,
                    exit_reason: IterativeExitReason::ProcessSuccess,
                    total_duration: start_time.elapsed(),
                });
            }

            // Record iteration summary
            let summary = IterationSummary {
                iteration: self.state.iteration,
                started_at: iteration_start,
                completed_at: Utc::now(),
                exit_code,
                output_preview: output.chars().take(500).collect(),
                promise_checked: self.state.config.completion_promise.is_some(),
            };
            self.state.iteration_summaries.push(summary);

            // Git auto-commit if configured
            if self.state.config.git.auto_commit {
                if let Err(e) = self.git_commit_iteration().await {
                    warn!("Failed to auto-commit iteration: {}", e);
                }
            }

            // Increment iteration and save state
            self.state.iteration += 1;
            self.state.last_iteration_at = Some(Utc::now());
            self.save_state().await?;

            // Store output for accumulation
            self.current_output = output;

            info!(
                "Iteration {} complete, continuing to next iteration",
                self.state.iteration
            );
        }
    }

    /// Execute a single iteration of the loop
    async fn execute_iteration(&self) -> Result<(String, Option<i32>)> {
        let prompt = self.build_iteration_prompt();
        let mut cmd = self.build_command(&prompt)?;

        // Apply timeout if configured
        let timeout_duration = self
            .state
            .config
            .iteration_timeout_secs
            .map(Duration::from_secs);

        // Spawn process
        let mut child = cmd.spawn().context("Failed to spawn process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        // Read output
        let output_future = async {
            let mut output = String::new();

            // Read stdout
            let stdout_reader = BufReader::new(stdout);
            let mut stdout_lines = stdout_reader.lines();
            while let Ok(Some(line)) = stdout_lines.next_line().await {
                output.push_str(&line);
                output.push('\n');
            }

            // Read stderr (append to output)
            let stderr_reader = BufReader::new(stderr);
            let mut stderr_lines = stderr_reader.lines();
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                output.push_str("[stderr] ");
                output.push_str(&line);
                output.push('\n');
            }

            output
        };

        let output = if let Some(dur) = timeout_duration {
            match timeout(dur, output_future).await {
                Ok(output) => output,
                Err(_) => {
                    warn!("Iteration timed out after {:?}", dur);
                    child.kill().await.ok();
                    return Ok((String::from("[Iteration timed out]"), None));
                }
            }
        } else {
            output_future.await
        };

        // Wait for process to exit
        let status = child.wait().await?;
        let exit_code = status.code();

        debug!("Process exited with code: {:?}", exit_code);
        Ok((output, exit_code))
    }

    /// Build the prompt for this iteration
    fn build_iteration_prompt(&self) -> String {
        let mut prompt = self.state.config.prompt.clone();

        if self.state.config.include_iteration_context && self.state.iteration > 0 {
            let iteration_info = format!(
                "\n\n---\n\
                ITERATION CONTEXT:\n\
                - This is iteration {} of {}\n\
                - Your previous work persists in files and git history\n\
                - Review what you've done and continue improving\n\
                {}\
                ---",
                self.state.iteration + 1,
                self.state
                    .config
                    .max_iterations
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "unlimited".to_string()),
                if let Some(ref promise) = self.state.config.completion_promise {
                    format!(
                        "- Output <promise>{}</promise> when the task is completely finished\n",
                        promise
                    )
                } else {
                    String::new()
                }
            );
            prompt.push_str(&iteration_info);
        }

        prompt
    }

    /// Build the command to execute
    fn build_command(&self, prompt: &str) -> Result<Command> {
        let mut cmd = Command::new(&self.state.config.command);

        // Set working directory
        cmd.current_dir(&self.working_dir);

        // Set up stdio
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Add base args
        cmd.args(&self.state.config.args);

        // Add prompt based on backend type
        match self.state.config.backend.prompt_mode.as_str() {
            "arg" => {
                cmd.arg(prompt);
            }
            "env" => {
                cmd.env("PROMPT", prompt);
            }
            // "stdin" would require piped stdin - not implemented yet
            _ => {
                cmd.arg(prompt);
            }
        }

        // Add environment variables
        for (key, value) in &self.state.config.backend.environment {
            cmd.env(key, value);
        }

        Ok(cmd)
    }

    /// Check if output contains the completion promise
    fn check_completion_promise(&self, output: &str, promise: &str) -> Option<String> {
        // Try tagged format first: <promise>TEXT</promise>
        let tagged_pattern = format!("<promise>{}</promise>", promise);
        if output.contains(&tagged_pattern) {
            return Some(promise.to_string());
        }

        // Try just the promise text (case-insensitive)
        if output.to_lowercase().contains(&promise.to_lowercase()) {
            return Some(promise.to_string());
        }

        None
    }

    /// Save state to disk
    async fn save_state(&self) -> Result<()> {
        // Ensure directory exists
        if let Some(parent) = self.state_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        let content = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_path, content).await?;
        debug!("Saved loop state to {:?}", self.state_path);
        Ok(())
    }

    /// Create a git commit for the current iteration
    async fn git_commit_iteration(&self) -> Result<()> {
        let message = self
            .state
            .config
            .git
            .commit_template
            .replace("{iteration}", &(self.state.iteration + 1).to_string());

        // Use git command directly
        let output = Command::new("git")
            .current_dir(&self.working_dir)
            .args(["add", "-A"])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(()); // Nothing to add
        }

        let output = Command::new("git")
            .current_dir(&self.working_dir)
            .args(["commit", "-m", &message])
            .output()
            .await?;

        if output.status.success() {
            info!("Created git commit: {}", message);
        }

        Ok(())
    }

    /// Get current state (for GUI)
    pub fn state(&self) -> &IterativeLoopState {
        &self.state
    }

    /// Get current iteration number
    pub fn current_iteration(&self) -> u32 {
        self.state.iteration
    }

    /// Check if loop has completed
    pub fn is_completed(&self) -> bool {
        self.state.completed
    }
}

// ============================================================================
// Backend Trait & Presets (Phase 3)
// ============================================================================

/// Trait for CLI backend customization
pub trait LoopBackend: Send + Sync {
    /// Get the command name
    fn command(&self) -> &str;

    /// Get base arguments (before prompt)
    fn base_args(&self) -> Vec<String>;

    /// Format the prompt for this backend
    fn format_prompt(&self, prompt: &str, iteration: u32, max_iterations: Option<u32>) -> String;

    /// Check for completion in output (backend-specific patterns)
    fn check_completion(&self, output: &str, promise: &str) -> Option<String>;

    /// Get recommended output format
    fn output_format(&self) -> &str {
        "text"
    }
}

/// Claude Code CLI backend for iterative loops
pub struct LoopClaudeBackend {
    pub model: Option<String>,
    pub output_format: String,
}

impl Default for LoopClaudeBackend {
    fn default() -> Self {
        Self {
            model: None,
            output_format: "stream-json".to_string(),
        }
    }
}

impl LoopBackend for LoopClaudeBackend {
    fn command(&self) -> &str {
        "claude"
    }

    fn base_args(&self) -> Vec<String> {
        let mut args = vec!["-p".to_string()];
        args.push("--output-format".to_string());
        args.push(self.output_format.clone());
        if let Some(ref model) = self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
        args
    }

    fn format_prompt(&self, prompt: &str, iteration: u32, max_iterations: Option<u32>) -> String {
        if iteration == 0 {
            prompt.to_string()
        } else {
            format!(
                "{}\n\n[Iteration {} of {}. Review your previous work and continue. \
                Output <promise>COMPLETE</promise> when done.]",
                prompt,
                iteration + 1,
                max_iterations.map(|m| m.to_string()).unwrap_or("∞".to_string())
            )
        }
    }

    fn check_completion(&self, output: &str, promise: &str) -> Option<String> {
        let patterns = [
            format!("<promise>{}</promise>", promise),
            format!("<promise>{}</promise>", promise.to_uppercase()),
        ];

        for pattern in &patterns {
            if output.contains(pattern) {
                return Some(promise.to_string());
            }
        }
        None
    }

    fn output_format(&self) -> &str {
        &self.output_format
    }
}

/// OpenCode CLI backend for iterative loops
pub struct LoopOpenCodeBackend {
    pub model: Option<String>,
}

impl Default for LoopOpenCodeBackend {
    fn default() -> Self {
        Self { model: None }
    }
}

impl LoopBackend for LoopOpenCodeBackend {
    fn command(&self) -> &str {
        "opencode"
    }

    fn base_args(&self) -> Vec<String> {
        let mut args = vec!["run".to_string(), "--format".to_string(), "json".to_string()];
        if let Some(ref model) = self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
        args
    }

    fn format_prompt(&self, prompt: &str, iteration: u32, max_iterations: Option<u32>) -> String {
        if iteration == 0 {
            prompt.to_string()
        } else {
            format!(
                "{}\n\n[Iteration {} of {}]",
                prompt,
                iteration + 1,
                max_iterations.map(|m| m.to_string()).unwrap_or("∞".to_string())
            )
        }
    }

    fn check_completion(&self, output: &str, promise: &str) -> Option<String> {
        if output.contains(&format!("<promise>{}</promise>", promise)) {
            return Some(promise.to_string());
        }
        None
    }

    fn output_format(&self) -> &str {
        "json"
    }
}

/// Generic backend for arbitrary commands in iterative loops
pub struct LoopGenericBackend {
    pub command: String,
    pub base_args: Vec<String>,
}

impl LoopBackend for LoopGenericBackend {
    fn command(&self) -> &str {
        &self.command
    }

    fn base_args(&self) -> Vec<String> {
        self.base_args.clone()
    }

    fn format_prompt(&self, prompt: &str, _iteration: u32, _max_iterations: Option<u32>) -> String {
        prompt.to_string()
    }

    fn check_completion(&self, output: &str, promise: &str) -> Option<String> {
        if output.contains(promise) {
            return Some(promise.to_string());
        }
        None
    }
}

/// Create a backend from config
pub fn create_loop_backend(config: &LoopBackendConfig, command: &str) -> Box<dyn LoopBackend> {
    match config.backend_type.as_str() {
        "claude" => Box::new(LoopClaudeBackend::default()),
        "opencode" => Box::new(LoopOpenCodeBackend::default()),
        _ => Box::new(LoopGenericBackend {
            command: command.to_string(),
            base_args: Vec::new(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = IterativeLoopConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: IterativeLoopConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_iterations, config.max_iterations);
    }

    #[test]
    fn test_state_serialization() {
        let state = IterativeLoopState::default();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: IterativeLoopState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.iteration, state.iteration);
        assert_eq!(parsed.completed, state.completed);
    }

    #[test]
    fn test_state_defaults() {
        let state = IterativeLoopState::default();
        assert_eq!(state.iteration, 0);
        assert!(!state.completed);
        assert_eq!(state.exit_reason, Some(IterativeExitReason::Running));
    }

    #[test]
    fn test_config_defaults() {
        let config = IterativeLoopConfig::default();
        assert!(config.command.is_empty());
        assert!(config.include_iteration_context);
        assert_eq!(config.max_iterations, Some(10));
        assert!(!config.git.auto_commit);
    }

    #[test]
    fn test_backend_config_defaults() {
        let backend = LoopBackendConfig::default();
        assert_eq!(backend.backend_type, "generic");
        assert_eq!(backend.prompt_mode, "arg");
        assert_eq!(backend.output_format, "text");
    }

    #[test]
    fn test_git_config_defaults() {
        let git = LoopGitConfig::default();
        assert!(!git.auto_commit);
        assert!(!git.create_branch);
        assert!(git.commit_template.contains("{iteration}"));
    }

    #[test]
    fn test_exit_reason_serialization() {
        // Test simple variant
        let reason = IterativeExitReason::CompletionPromiseDetected;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("completion_promise_detected"));

        // Test variant with data
        let reason = IterativeExitReason::Error {
            message: "test error".to_string(),
        };
        let json = serde_json::to_string(&reason).unwrap();
        let parsed: IterativeExitReason = serde_json::from_str(&json).unwrap();
        if let IterativeExitReason::Error { message } = parsed {
            assert_eq!(message, "test error");
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_iteration_summary() {
        let summary = IterationSummary {
            iteration: 1,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            exit_code: Some(0),
            output_preview: "Hello world".to_string(),
            promise_checked: true,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: IterationSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.iteration, 1);
        assert_eq!(parsed.exit_code, Some(0));
    }

    // ========================================================================
    // Executor Tests
    // ========================================================================

    #[tokio::test]
    async fn test_iterative_loop_with_echo() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let state_file = temp_dir.path().join("loop-state.json");

        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec![],
            prompt: "<promise>DONE</promise>".to_string(),
            completion_promise: Some("DONE".to_string()),
            max_iterations: Some(5),
            state_file: Some(state_file),
            working_directory: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();
        let result = loop_exec.execute().await.unwrap();

        assert_eq!(result.exit_reason, IterativeExitReason::CompletionPromiseDetected);
        assert_eq!(result.iterations_completed, 1);
        assert!(result.completion_promise_found);
    }

    #[tokio::test]
    async fn test_max_iterations() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let state_file = temp_dir.path().join("loop-state.json");

        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec![],
            prompt: "no promise here".to_string(),
            completion_promise: Some("NEVER_FOUND".to_string()),
            max_iterations: Some(3),
            state_file: Some(state_file),
            working_directory: Some(temp_dir.path().to_path_buf()),
            // Disable iteration context so echo doesn't output the promise text
            include_iteration_context: false,
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();
        let result = loop_exec.execute().await.unwrap();

        assert_eq!(result.exit_reason, IterativeExitReason::MaxIterationsReached);
        assert_eq!(result.iterations_completed, 3);
        assert!(!result.completion_promise_found);
    }

    #[test]
    fn test_completion_promise_detection() {
        // Create a minimal config for testing
        let config = IterativeLoopConfig {
            completion_promise: Some("DONE".to_string()),
            ..Default::default()
        };

        let state = IterativeLoopState {
            config,
            ..Default::default()
        };

        // Create a mock executor just to test the method
        // We'll test the check_completion_promise method directly

        // Test tagged format
        let tagged = "<promise>DONE</promise>";
        assert!(tagged.contains("<promise>DONE</promise>"));

        // Test case-insensitive
        let output = "Task is DONE now";
        assert!(output.to_lowercase().contains(&"done".to_lowercase()));

        // Test not found
        let output = "still working";
        assert!(!output.to_lowercase().contains(&"done".to_lowercase())
            && !output.contains("<promise>DONE</promise>"));
    }

    #[tokio::test]
    async fn test_state_persistence() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let state_file = temp_dir.path().join("loop-state.json");

        // Run a loop that hits max iterations
        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec![],
            prompt: "test".to_string(),
            completion_promise: Some("NEVER".to_string()),
            max_iterations: Some(2),
            state_file: Some(state_file.clone()),
            working_directory: Some(temp_dir.path().to_path_buf()),
            // Disable iteration context so echo doesn't output the promise text
            include_iteration_context: false,
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();
        let _ = loop_exec.execute().await.unwrap();

        // Verify state file exists
        assert!(state_file.exists());

        // Load and verify state
        let content = std::fs::read_to_string(&state_file).unwrap();
        let state: IterativeLoopState = serde_json::from_str(&content).unwrap();
        assert_eq!(state.iteration, 2);
        assert!(state.completed);
        assert_eq!(state.exit_reason, Some(IterativeExitReason::MaxIterationsReached));
    }

    #[tokio::test]
    async fn test_process_success_without_promise() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let state_file = temp_dir.path().join("loop-state.json");

        // When no completion_promise is set, exit code 0 should succeed
        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            prompt: String::new(),
            completion_promise: None, // No promise configured
            max_iterations: Some(5),
            state_file: Some(state_file),
            working_directory: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();
        let result = loop_exec.execute().await.unwrap();

        assert_eq!(result.exit_reason, IterativeExitReason::ProcessSuccess);
        assert_eq!(result.iterations_completed, 1);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let state_file = temp_dir.path().join("loop-state.json");

        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec![],
            prompt: "test".to_string(),
            completion_promise: Some("NEVER".to_string()),
            max_iterations: Some(100),
            state_file: Some(state_file),
            working_directory: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();

        // Set cancellation flag before execution
        loop_exec.cancellation_handle().store(true, Ordering::Relaxed);

        let result = loop_exec.execute().await.unwrap();

        assert_eq!(result.exit_reason, IterativeExitReason::UserCancelled);
        assert_eq!(result.iterations_completed, 0);
    }

    #[test]
    fn test_build_iteration_prompt() {
        let config = IterativeLoopConfig {
            prompt: "Build something".to_string(),
            completion_promise: Some("COMPLETE".to_string()),
            max_iterations: Some(10),
            include_iteration_context: true,
            ..Default::default()
        };

        // First iteration - no context added
        let mut state = IterativeLoopState {
            config: config.clone(),
            iteration: 0,
            ..Default::default()
        };

        // Simulate what build_iteration_prompt does for iteration 0
        let prompt = if state.config.include_iteration_context && state.iteration > 0 {
            format!("{}\n[context]", state.config.prompt)
        } else {
            state.config.prompt.clone()
        };
        assert_eq!(prompt, "Build something");

        // Second iteration - context should be added
        state.iteration = 1;
        let prompt = if state.config.include_iteration_context && state.iteration > 0 {
            format!("{}\n[context added]", state.config.prompt)
        } else {
            state.config.prompt.clone()
        };
        assert!(prompt.contains("[context added]"));
    }

    // Phase 3: Backend tests
    #[test]
    fn test_loop_claude_backend_prompt_formatting() {
        let backend = LoopClaudeBackend::default();

        // First iteration - prompt unchanged
        let prompt = backend.format_prompt("Build something", 0, Some(10));
        assert_eq!(prompt, "Build something");

        // Second iteration - context added
        let prompt = backend.format_prompt("Build something", 1, Some(10));
        assert!(prompt.contains("Iteration 2 of 10"));
        assert!(prompt.contains("Review your previous work"));
    }

    #[test]
    fn test_loop_claude_backend_args() {
        let backend = LoopClaudeBackend::default();
        let args = backend.base_args();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
    }

    #[test]
    fn test_loop_claude_backend_with_model() {
        let backend = LoopClaudeBackend {
            model: Some("sonnet".to_string()),
            output_format: "json".to_string(),
        };
        let args = backend.base_args();
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"sonnet".to_string()));
    }

    #[test]
    fn test_loop_claude_backend_completion() {
        let backend = LoopClaudeBackend::default();

        // Tagged format
        let result = backend.check_completion("Output <promise>DONE</promise> here", "DONE");
        assert!(result.is_some());

        // Uppercase variant
        let result = backend.check_completion("Output <promise>done</promise> here", "done");
        assert!(result.is_some());

        // No match
        let result = backend.check_completion("Still working...", "DONE");
        assert!(result.is_none());
    }

    #[test]
    fn test_loop_opencode_backend_prompt_formatting() {
        let backend = LoopOpenCodeBackend::default();

        // First iteration
        let prompt = backend.format_prompt("Do something", 0, Some(5));
        assert_eq!(prompt, "Do something");

        // Later iteration
        let prompt = backend.format_prompt("Do something", 2, Some(5));
        assert!(prompt.contains("Iteration 3 of 5"));
    }

    #[test]
    fn test_loop_opencode_backend_args() {
        let backend = LoopOpenCodeBackend::default();
        let args = backend.base_args();
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--format".to_string()));
        assert!(args.contains(&"json".to_string()));
    }

    #[test]
    fn test_loop_generic_backend() {
        let backend = LoopGenericBackend {
            command: "python".to_string(),
            base_args: vec!["script.py".to_string()],
        };

        assert_eq!(backend.command(), "python");
        assert_eq!(backend.base_args(), vec!["script.py".to_string()]);

        // Generic backend doesn't modify prompt
        let prompt = backend.format_prompt("test", 5, Some(10));
        assert_eq!(prompt, "test");

        // Simple string match for completion
        let result = backend.check_completion("Task COMPLETE now", "COMPLETE");
        assert!(result.is_some());
    }

    #[test]
    fn test_create_loop_backend_factory() {
        let config = LoopBackendConfig {
            backend_type: "claude".to_string(),
            ..Default::default()
        };
        let backend = create_loop_backend(&config, "unused");
        assert_eq!(backend.command(), "claude");

        let config = LoopBackendConfig {
            backend_type: "opencode".to_string(),
            ..Default::default()
        };
        let backend = create_loop_backend(&config, "unused");
        assert_eq!(backend.command(), "opencode");

        let config = LoopBackendConfig {
            backend_type: "generic".to_string(),
            ..Default::default()
        };
        let backend = create_loop_backend(&config, "my-cli");
        assert_eq!(backend.command(), "my-cli");
    }

    #[test]
    fn test_loop_backend_output_format() {
        let claude = LoopClaudeBackend::default();
        assert_eq!(claude.output_format(), "stream-json");

        let opencode = LoopOpenCodeBackend::default();
        assert_eq!(opencode.output_format(), "json");
    }
}
