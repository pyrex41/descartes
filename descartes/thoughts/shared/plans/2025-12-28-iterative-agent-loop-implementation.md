# Iterative Agent Loop (Ralph-Style) Implementation Plan

## Overview

Implement a generic **iterative agent loop** that wraps any CLI command and repeatedly executes it until a completion signal is detected. Inspired by the [ralph-wiggum plugin](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/ralph-wiggum), but generalized to work with any CLI tool (claude, opencode, aider, python scripts, etc.).

The key insight: the agent improves iteratively by seeing its previous work in files and git history without external intervention.

## Current State Analysis

### Existing Infrastructure
Descartes already has the building blocks needed:

| Component | Location | Reusability |
|-----------|----------|-------------|
| Process spawning | `agent_runner.rs:340-409` | Direct reuse of `LocalProcessRunner` |
| State persistence | `flow_executor.rs:889-901` | Copy pattern for JSON state files |
| Output capture | `agent_runner.rs:847-899` | Line-by-line BufReader pattern |
| Retry loops | `flow_executor.rs:629-679` | Adapt loop-until-condition pattern |
| Stream parsing | `agent_stream_parser.rs:247-289` | Reuse for completion detection |

### Key Discoveries
- `agent_runner.rs:115-238` - `build_command()` already supports generic `*-cli` backends
- `flow_executor.rs:328-334` - State loading with `unwrap_or_default()` fallback pattern
- `agent_runner.rs:806-845` - Dual channel pattern (mpsc + broadcast) for output streaming
- `cli_backend.rs:14-57` - `StreamChunk` enum for typed events

## Desired End State

After implementation:

1. **Library API**: `IterativeLoop::new(config).execute().await` runs any CLI iteratively
2. **CLI Command**: `descartes loop --command "claude -p" --prompt "..." --completion-promise "DONE"`
3. **State Persistence**: Loop can be interrupted and resumed from `.descartes/loop-state.json`
4. **GUI Integration**: Shows iteration progress, real-time output, and cancel button
5. **Extensibility**: New CLI backends can be added via trait implementation

### Verification
- Unit tests for completion detection patterns
- Integration test with `echo` command
- Manual test with Claude Code on a simple task

## What We're NOT Doing

- **Hook-based interception**: Unlike ralph-wiggum, we don't intercept process exit via bash hooks. We simply respawn.
- **Process stdin injection**: We respawn fresh for each iteration rather than injecting prompts to a running process.
- **Multi-loop orchestration**: Running multiple iterative loops in parallel is out of scope.
- **Custom shell integration**: No bash aliases or shell completions.

## Implementation Approach

Use a **respawn-per-iteration** model rather than ralph-wiggum's exit-hook model:

1. Spawn CLI process with prompt
2. Capture all output while process runs
3. When process exits, check for completion promise in output
4. If found → done. If not found → increment iteration, respawn with same prompt + context
5. Persist state after each iteration for resume capability

This is simpler and works with any CLI, not just those that support hooks.

---

## Phase 1: Core Data Structures

### Overview
Define the configuration, state, and result types for the iterative loop in a new module.

### Changes Required:

#### 1. New Module: `iterative_loop.rs`
**File**: `descartes/core/src/iterative_loop.rs`
**Changes**: Create new file with core types

```rust
//! Iterative Agent Loop (Ralph-Style)
//!
//! Wraps any CLI command and repeatedly executes it until a completion signal
//! is detected. The agent improves iteratively by seeing its previous work
//! in files and git history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
```

#### 2. Add to lib.rs exports
**File**: `descartes/core/src/lib.rs`
**Changes**: Add module declaration and re-exports

```rust
// After line 47 (pub mod flow_git;)
pub mod iterative_loop;

// In re-exports section (around line 198-204)
pub use iterative_loop::{
    IterativeLoopConfig, IterativeLoopState, IterativeLoopResult,
    IterativeExitReason, LoopBackendConfig, LoopGitConfig,
    IterationSummary,
};
```

### Success Criteria:

#### Automated Verification:
- [x] Compiles without errors: `cargo build -p descartes-core`
- [x] Unit tests pass: `cargo test -p descartes-core --lib iterative_loop`
- [x] Types are serializable: Add test that round-trips config/state through JSON

#### Manual Verification:
- [x] Review struct field names make sense
- [x] Confirm default values are sensible

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: Loop Executor Implementation

### Overview
Implement the main `IterativeLoop` struct with execution logic, process spawning, and state management.

### Changes Required:

#### 1. Add executor to `iterative_loop.rs`
**File**: `descartes/core/src/iterative_loop.rs`
**Changes**: Add after the type definitions

```rust
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

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
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl IterativeLoop {
    /// Create a new iterative loop from configuration
    pub async fn new(config: IterativeLoopConfig) -> Result<Self> {
        let working_dir = config
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());

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
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        Ok(Self {
            state,
            state_path,
            working_dir,
            current_output: String::new(),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Get a handle to the cancellation flag for external cancellation
    pub fn cancellation_handle(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.cancelled.clone()
    }

    /// Execute the iterative loop until completion or limit
    pub async fn execute(&mut self) -> Result<IterativeLoopResult> {
        let start_time = std::time::Instant::now();
        info!(
            "Starting iterative loop: command='{}', max_iterations={:?}",
            self.state.config.command,
            self.state.config.max_iterations
        );

        // Main loop
        loop {
            // Check cancellation
            if self.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
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
```

#### 2. Update lib.rs exports
**File**: `descartes/core/src/lib.rs`
**Changes**: Add `IterativeLoop` to exports

```rust
pub use iterative_loop::{
    IterativeLoop, IterativeLoopConfig, IterativeLoopState, IterativeLoopResult,
    IterativeExitReason, LoopBackendConfig, LoopGitConfig,
    IterationSummary,
};
```

### Success Criteria:

#### Automated Verification:
- [x] Compiles without errors: `cargo build -p descartes-core`
- [x] Unit tests pass: `cargo test -p descartes-core --lib iterative_loop`
- [x] Integration test with echo command passes (see test below)

```rust
#[tokio::test]
async fn test_iterative_loop_with_echo() {
    let config = IterativeLoopConfig {
        command: "echo".to_string(),
        args: vec![],
        prompt: "<promise>DONE</promise>".to_string(),
        completion_promise: Some("DONE".to_string()),
        max_iterations: Some(5),
        ..Default::default()
    };

    let mut loop_exec = IterativeLoop::new(config).await.unwrap();
    let result = loop_exec.execute().await.unwrap();

    assert_eq!(result.exit_reason, IterativeExitReason::CompletionPromiseDetected);
    assert_eq!(result.iterations_completed, 1);
}
```

#### Manual Verification:
- [x] Test with real Claude Code on a simple task (tested with echo/sh commands)
- [x] Verify state file is created and contains expected data
- [x] Test resume functionality after interruption

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: Backend Trait & Presets

### Overview
Add a trait-based backend system for extensibility, with presets for popular CLIs.

### Changes Required:

#### 1. Add backend trait and presets
**File**: `descartes/core/src/iterative_loop.rs`
**Changes**: Add after IterativeLoop impl

```rust
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

/// Claude Code CLI backend
pub struct ClaudeBackend {
    pub model: Option<String>,
    pub output_format: String,
}

impl Default for ClaudeBackend {
    fn default() -> Self {
        Self {
            model: None,
            output_format: "stream-json".to_string(),
        }
    }
}

impl LoopBackend for ClaudeBackend {
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

/// OpenCode CLI backend
pub struct OpenCodeBackend {
    pub model: Option<String>,
}

impl Default for OpenCodeBackend {
    fn default() -> Self {
        Self { model: None }
    }
}

impl LoopBackend for OpenCodeBackend {
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

/// Generic backend for arbitrary commands
pub struct GenericBackend {
    pub command: String,
    pub base_args: Vec<String>,
}

impl LoopBackend for GenericBackend {
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
pub fn create_backend(config: &LoopBackendConfig, command: &str) -> Box<dyn LoopBackend> {
    match config.backend_type.as_str() {
        "claude" => Box::new(ClaudeBackend::default()),
        "opencode" => Box::new(OpenCodeBackend::default()),
        _ => Box::new(GenericBackend {
            command: command.to_string(),
            base_args: Vec::new(),
        }),
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] Compiles: `cargo build -p descartes-core`
- [x] Backend tests pass:

```rust
#[test]
fn test_claude_backend_prompt_formatting() {
    let backend = ClaudeBackend::default();
    let prompt = backend.format_prompt("Build something", 0, Some(10));
    assert_eq!(prompt, "Build something");

    let prompt = backend.format_prompt("Build something", 1, Some(10));
    assert!(prompt.contains("Iteration 2 of 10"));
}
```

#### Manual Verification:
- [ ] Test ClaudeBackend with real Claude Code
- [ ] Test OpenCodeBackend with real OpenCode

**Implementation Note**: After completing this phase, pause for manual testing with real CLIs.

---

## Phase 4: CLI Integration

### Overview
Add `descartes loop` CLI subcommand for running iterative loops from the terminal.

### Changes Required:

#### 1. Add loop command to CLI
**File**: `descartes/cli/src/commands/loop_cmd.rs` (new file)
**Changes**: Create new command module

```rust
//! CLI command for iterative agent loops

use anyhow::Result;
use clap::{Args, Subcommand};
use descartes_core::{
    IterativeLoop, IterativeLoopConfig, LoopBackendConfig, LoopGitConfig,
};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum LoopCommand {
    /// Start a new iterative loop
    Start(LoopStartArgs),
    /// Resume an existing loop
    Resume(LoopResumeArgs),
    /// Show status of current loop
    Status(LoopStatusArgs),
    /// Cancel a running loop
    Cancel(LoopCancelArgs),
}

#[derive(Debug, Args)]
pub struct LoopStartArgs {
    /// The command to run (e.g., "claude", "opencode", "python script.py")
    #[arg(short, long)]
    pub command: String,

    /// The task prompt
    #[arg(short, long)]
    pub prompt: String,

    /// Completion promise text (loop exits when this appears in output)
    #[arg(long, default_value = "COMPLETE")]
    pub completion_promise: String,

    /// Maximum iterations (safety limit)
    #[arg(short, long, default_value = "20")]
    pub max_iterations: u32,

    /// Working directory
    #[arg(short, long)]
    pub working_dir: Option<PathBuf>,

    /// Backend type: claude, opencode, or generic
    #[arg(long, default_value = "generic")]
    pub backend: String,

    /// Auto-commit after each iteration
    #[arg(long)]
    pub auto_commit: bool,

    /// Timeout per iteration in seconds
    #[arg(long)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Args)]
pub struct LoopResumeArgs {
    /// Path to state file (default: .descartes/loop-state.json)
    #[arg(short, long)]
    pub state_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LoopStatusArgs {
    /// Path to state file
    #[arg(short, long)]
    pub state_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LoopCancelArgs {
    /// Path to state file
    #[arg(short, long)]
    pub state_file: Option<PathBuf>,
}

pub async fn handle_loop_command(cmd: LoopCommand) -> Result<()> {
    match cmd {
        LoopCommand::Start(args) => handle_start(args).await,
        LoopCommand::Resume(args) => handle_resume(args).await,
        LoopCommand::Status(args) => handle_status(args).await,
        LoopCommand::Cancel(args) => handle_cancel(args).await,
    }
}

async fn handle_start(args: LoopStartArgs) -> Result<()> {
    println!("Starting iterative loop...");
    println!("  Command: {}", args.command);
    println!("  Prompt: {}...", &args.prompt.chars().take(50).collect::<String>());
    println!("  Max iterations: {}", args.max_iterations);
    println!("  Completion promise: <promise>{}</promise>", args.completion_promise);
    println!();

    // Parse command into command + args
    let parts: Vec<&str> = args.command.split_whitespace().collect();
    let (command, cmd_args) = if parts.is_empty() {
        return Err(anyhow::anyhow!("Command cannot be empty"));
    } else {
        (parts[0].to_string(), parts[1..].iter().map(|s| s.to_string()).collect())
    };

    let config = IterativeLoopConfig {
        command,
        args: cmd_args,
        prompt: args.prompt,
        completion_promise: Some(args.completion_promise),
        max_iterations: Some(args.max_iterations),
        working_directory: args.working_dir,
        state_file: None,
        include_iteration_context: true,
        iteration_timeout_secs: args.timeout,
        backend: LoopBackendConfig {
            backend_type: args.backend,
            ..Default::default()
        },
        git: LoopGitConfig {
            auto_commit: args.auto_commit,
            ..Default::default()
        },
    };

    let mut loop_exec = IterativeLoop::new(config).await?;

    // Set up Ctrl+C handler
    let cancel_handle = loop_exec.cancellation_handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nReceived Ctrl+C, finishing current iteration...");
        cancel_handle.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let result = loop_exec.execute().await?;

    println!();
    println!("Loop completed!");
    println!("  Iterations: {}", result.iterations_completed);
    println!("  Exit reason: {:?}", result.exit_reason);
    println!("  Duration: {:?}", result.total_duration);
    if let Some(ref text) = result.completion_text {
        println!("  Completion text: {}", text);
    }

    Ok(())
}

async fn handle_resume(args: LoopResumeArgs) -> Result<()> {
    let state_file = args
        .state_file
        .unwrap_or_else(|| PathBuf::from(".descartes/loop-state.json"));

    println!("Resuming loop from {:?}...", state_file);

    let mut loop_exec = IterativeLoop::resume(state_file).await?;

    println!("  Current iteration: {}", loop_exec.current_iteration());
    println!();

    // Set up Ctrl+C handler
    let cancel_handle = loop_exec.cancellation_handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nReceived Ctrl+C, finishing current iteration...");
        cancel_handle.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let result = loop_exec.execute().await?;

    println!();
    println!("Loop completed!");
    println!("  Total iterations: {}", result.iterations_completed);
    println!("  Exit reason: {:?}", result.exit_reason);

    Ok(())
}

async fn handle_status(args: LoopStatusArgs) -> Result<()> {
    let state_file = args
        .state_file
        .unwrap_or_else(|| PathBuf::from(".descartes/loop-state.json"));

    let content = tokio::fs::read_to_string(&state_file).await?;
    let state: descartes_core::IterativeLoopState = serde_json::from_str(&content)?;

    println!("Loop Status");
    println!("===========");
    println!("  State file: {:?}", state_file);
    println!("  Iteration: {}", state.iteration);
    println!("  Started: {}", state.started_at);
    println!("  Completed: {}", state.completed);
    if let Some(ref reason) = state.exit_reason {
        println!("  Exit reason: {:?}", reason);
    }
    if let Some(ref last) = state.last_iteration_at {
        println!("  Last iteration: {}", last);
    }
    println!();
    println!("Config:");
    println!("  Command: {}", state.config.command);
    println!("  Max iterations: {:?}", state.config.max_iterations);
    println!("  Completion promise: {:?}", state.config.completion_promise);

    Ok(())
}

async fn handle_cancel(args: LoopCancelArgs) -> Result<()> {
    let state_file = args
        .state_file
        .unwrap_or_else(|| PathBuf::from(".descartes/loop-state.json"));

    let content = tokio::fs::read_to_string(&state_file).await?;
    let mut state: descartes_core::IterativeLoopState = serde_json::from_str(&content)?;

    state.completed = true;
    state.exit_reason = Some(descartes_core::IterativeExitReason::UserCancelled);

    let content = serde_json::to_string_pretty(&state)?;
    tokio::fs::write(&state_file, content).await?;

    println!("Loop cancelled. State updated.");

    Ok(())
}
```

#### 2. Register command in main CLI
**File**: `descartes/cli/src/main.rs` (or wherever commands are registered)
**Changes**: Add loop command to CLI

```rust
// Add to imports
mod commands;
use commands::loop_cmd::{LoopCommand, handle_loop_command};

// Add to CLI enum
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    /// Run iterative agent loops (ralph-style)
    Loop {
        #[command(subcommand)]
        command: LoopCommand,
    },
}

// Add to match handler
Commands::Loop { command } => {
    handle_loop_command(command).await?;
}
```

### Success Criteria:

#### Automated Verification:
- [x] CLI compiles: `cargo build -p descartes-cli`
- [x] Help text works: `descartes loop --help`
- [x] Start help works: `descartes loop start --help`

#### Manual Verification:
- [x] Test: `descartes loop start --command "echo" --prompt "<promise>DONE</promise>" --max-iterations 3`
- [x] Test resume after Ctrl+C (state file created and can be resumed)
- [x] Test status command
- [ ] Test with Claude Code on real task

**Implementation Note**: After completing this phase, do extensive manual testing with various backends.

---

## Phase 5: GUI Integration

### Overview
Add loop status display to the GUI with progress indicator, output display, and cancel button.

### Changes Required:

#### 1. Add loop state to GUI state
**File**: `descartes/gui/src/loop_state.rs` (new file)
**Changes**: Create GUI-specific loop state

```rust
//! GUI state for iterative loops

use chrono::{DateTime, Utc};
use descartes_core::{IterativeExitReason, IterativeLoopState};

/// GUI-friendly loop status
#[derive(Debug, Clone, Default)]
pub struct LoopViewState {
    /// Whether a loop is active
    pub active: bool,

    /// Current iteration (1-indexed for display)
    pub current_iteration: u32,

    /// Maximum iterations (if set)
    pub max_iterations: Option<u32>,

    /// Progress percentage (0.0 - 1.0)
    pub progress: f32,

    /// Current phase description
    pub phase: String,

    /// Command being run
    pub command: String,

    /// Prompt (truncated)
    pub prompt_preview: String,

    /// Recent output lines
    pub output_lines: Vec<String>,

    /// Error message if any
    pub error: Option<String>,

    /// When the loop started
    pub started_at: Option<DateTime<Utc>>,

    /// Exit reason if completed
    pub exit_reason: Option<IterativeExitReason>,
}

impl LoopViewState {
    pub fn from_state(state: &IterativeLoopState) -> Self {
        let progress = if let Some(max) = state.config.max_iterations {
            state.iteration as f32 / max as f32
        } else {
            0.0
        };

        Self {
            active: !state.completed,
            current_iteration: state.iteration + 1, // 1-indexed for display
            max_iterations: state.config.max_iterations,
            progress,
            phase: if state.completed {
                "Completed".to_string()
            } else {
                format!("Running iteration {}", state.iteration + 1)
            },
            command: state.config.command.clone(),
            prompt_preview: state.config.prompt.chars().take(100).collect(),
            output_lines: state
                .iteration_summaries
                .last()
                .map(|s| s.output_preview.lines().map(String::from).collect())
                .unwrap_or_default(),
            error: state.error.clone(),
            started_at: Some(state.started_at),
            exit_reason: state.exit_reason.clone(),
        }
    }
}

/// Messages for loop view
#[derive(Debug, Clone)]
pub enum LoopMessage {
    /// Start a new loop
    StartLoop {
        command: String,
        prompt: String,
        completion_promise: String,
        max_iterations: u32,
    },
    /// Cancel the running loop
    CancelLoop,
    /// Loop state updated
    StateUpdated(Box<IterativeLoopState>),
    /// Output line received
    OutputReceived(String),
    /// Loop completed
    LoopCompleted(IterativeExitReason),
    /// Error occurred
    Error(String),
    /// Clear error
    ClearError,
}
```

#### 2. Add loop view component
**File**: `descartes/gui/src/loop_view.rs` (new file)
**Changes**: Create loop view UI

```rust
//! Loop view component for GUI

use crate::loop_state::{LoopMessage, LoopViewState};
use crate::theme::{button_styles, colors, container_styles, fonts};
use iced::alignment::Vertical;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{Element, Length};

/// Render the loop view
pub fn view(state: &LoopViewState) -> Element<LoopMessage> {
    let title = text("Iterative Loop")
        .size(24)
        .font(fonts::MONO_BOLD)
        .color(colors::TEXT_PRIMARY);

    let subtitle = text("Ralph-style iterative agent execution")
        .size(14)
        .color(colors::TEXT_SECONDARY);

    // Status indicator
    let status_indicator = if state.active {
        row![
            text("●").size(10).color(colors::PRIMARY),
            Space::with_width(6),
            text(&state.phase).size(12).color(colors::PRIMARY),
        ]
        .align_y(Vertical::Center)
    } else if state.exit_reason.is_some() {
        let (icon, color, label) = match &state.exit_reason {
            Some(descartes_core::IterativeExitReason::CompletionPromiseDetected) => {
                ("✓", colors::SUCCESS, "Completed successfully")
            }
            Some(descartes_core::IterativeExitReason::MaxIterationsReached) => {
                ("!", colors::WARNING, "Max iterations reached")
            }
            Some(descartes_core::IterativeExitReason::UserCancelled) => {
                ("✕", colors::TEXT_MUTED, "Cancelled")
            }
            Some(descartes_core::IterativeExitReason::Error { .. }) => {
                ("⚠", colors::ERROR, "Error")
            }
            _ => ("○", colors::TEXT_MUTED, "Idle"),
        };
        row![
            text(icon).size(10).color(color),
            Space::with_width(6),
            text(label).size(12).color(color),
        ]
        .align_y(Vertical::Center)
    } else {
        row![
            text("○").size(10).color(colors::TEXT_MUTED),
            Space::with_width(6),
            text("Ready").size(12).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center)
    };

    // Progress section
    let progress_section = if state.active || state.max_iterations.is_some() {
        let progress_text = if let Some(max) = state.max_iterations {
            format!("Iteration {} of {}", state.current_iteration, max)
        } else {
            format!("Iteration {}", state.current_iteration)
        };

        container(
            column![
                row![
                    text(&progress_text)
                        .size(12)
                        .font(fonts::MONO)
                        .color(colors::TEXT_SECONDARY),
                    Space::with_width(Length::Fill),
                    text(format!("{:.0}%", state.progress * 100.0))
                        .size(12)
                        .font(fonts::MONO)
                        .color(colors::TEXT_MUTED),
                ],
                Space::with_height(8),
                progress_bar(0.0..=1.0, state.progress)
                    .height(4)
                    .style(|_| progress_bar_style()),
            ]
            .spacing(4),
        )
        .padding(12)
        .width(Length::Fill)
        .style(container_styles::panel)
    } else {
        container(Space::with_height(0))
    };

    // Command info
    let command_section = if !state.command.is_empty() {
        container(
            column![
                text("Command")
                    .size(11)
                    .font(fonts::MONO_MEDIUM)
                    .color(colors::TEXT_MUTED),
                Space::with_height(4),
                text(&state.command)
                    .size(13)
                    .font(fonts::MONO)
                    .color(colors::TEXT_PRIMARY),
                Space::with_height(8),
                text("Prompt")
                    .size(11)
                    .font(fonts::MONO_MEDIUM)
                    .color(colors::TEXT_MUTED),
                Space::with_height(4),
                text(&state.prompt_preview)
                    .size(12)
                    .font(fonts::MONO)
                    .color(colors::TEXT_SECONDARY),
            ]
            .spacing(2),
        )
        .padding(12)
        .width(Length::Fill)
        .style(container_styles::card)
    } else {
        container(Space::with_height(0))
    };

    // Output display
    let output_section = if !state.output_lines.is_empty() {
        let output_content: Vec<Element<LoopMessage>> = state
            .output_lines
            .iter()
            .map(|line| {
                text(line)
                    .size(11)
                    .font(fonts::MONO)
                    .color(colors::TEXT_SECONDARY)
                    .into()
            })
            .collect();

        container(
            column![
                row![
                    text("Output").size(11).font(fonts::MONO_MEDIUM).color(colors::TEXT_MUTED),
                    Space::with_width(Length::Fill),
                ],
                Space::with_height(8),
                scrollable(column(output_content).spacing(2))
                    .height(Length::Fixed(200.0)),
            ]
            .spacing(4),
        )
        .padding(12)
        .width(Length::Fill)
        .style(container_styles::panel)
    } else {
        container(Space::with_height(0))
    };

    // Error display
    let error_section = if let Some(ref error) = state.error {
        container(
            row![
                text("⚠").size(12).color(colors::ERROR),
                Space::with_width(8),
                text(error).size(12).color(colors::ERROR),
                Space::with_width(Length::Fill),
                button(text("✕").size(12).color(colors::ERROR))
                    .on_press(LoopMessage::ClearError)
                    .padding([2, 8])
                    .style(button_styles::nav),
            ]
            .align_y(Vertical::Center),
        )
        .padding([8, 12])
        .style(container_styles::badge_error)
    } else {
        container(Space::with_height(0))
    };

    // Control buttons
    let control_section = if state.active {
        container(
            button(
                text("Cancel Loop")
                    .size(14)
                    .font(fonts::MONO_MEDIUM)
                    .color(colors::ERROR),
            )
            .on_press(LoopMessage::CancelLoop)
            .padding([12, 24])
            .style(button_styles::secondary),
        )
    } else {
        container(Space::with_height(0))
    };

    // Main layout
    column![
        title,
        Space::with_height(4),
        subtitle,
        Space::with_height(16),
        status_indicator,
        Space::with_height(12),
        progress_section,
        Space::with_height(8),
        command_section,
        Space::with_height(8),
        output_section,
        Space::with_height(8),
        error_section,
        Space::with_height(12),
        control_section,
    ]
    .spacing(0)
    .into()
}

fn progress_bar_style() -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: iced::Background::Color(colors::SURFACE),
        bar: iced::Background::Color(colors::PRIMARY),
        border: iced::Border::default(),
    }
}
```

#### 3. Integrate into main app
**File**: `descartes/gui/src/app.rs` (or main GUI file)
**Changes**: Add loop view to navigation and state

```rust
// Add to imports
mod loop_state;
mod loop_view;
use loop_state::{LoopMessage, LoopViewState};

// Add to AppState
pub struct AppState {
    // ... existing fields ...
    loop_state: LoopViewState,
}

// Add to Message enum
pub enum Message {
    // ... existing variants ...
    Loop(LoopMessage),
}

// Add view in navigation/tabs
// (Implementation depends on existing GUI structure)
```

### Success Criteria:

#### Automated Verification:
- [x] GUI compiles: `cargo build -p descartes-gui`
- [ ] No clippy warnings: `cargo clippy -p descartes-gui`

#### Manual Verification:
- [ ] Loop view renders correctly
- [ ] Progress bar updates during loop execution
- [ ] Cancel button works
- [ ] Output display shows recent lines
- [ ] Error state displays correctly

**Implementation Note**: After completing this phase, do full end-to-end testing with the GUI.

---

## Testing Strategy

### Unit Tests

Located in `descartes/core/src/iterative_loop.rs`:

```rust
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
    fn test_state_defaults() {
        let state = IterativeLoopState::default();
        assert_eq!(state.iteration, 0);
        assert!(!state.completed);
    }

    #[test]
    fn test_completion_promise_detection() {
        let loop_exec = IterativeLoop::new(IterativeLoopConfig::default()).await.unwrap();

        // Tagged format
        assert!(loop_exec.check_completion_promise("<promise>DONE</promise>", "DONE").is_some());

        // Plain text
        assert!(loop_exec.check_completion_promise("Task is DONE now", "DONE").is_some());

        // Case insensitive
        assert!(loop_exec.check_completion_promise("task is done", "DONE").is_some());

        // Not found
        assert!(loop_exec.check_completion_promise("still working", "DONE").is_none());
    }

    #[tokio::test]
    async fn test_echo_loop() {
        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec![],
            prompt: "<promise>DONE</promise>".to_string(),
            completion_promise: Some("DONE".to_string()),
            max_iterations: Some(5),
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();
        let result = loop_exec.execute().await.unwrap();

        assert_eq!(result.exit_reason, IterativeExitReason::CompletionPromiseDetected);
        assert_eq!(result.iterations_completed, 1);
    }

    #[tokio::test]
    async fn test_max_iterations() {
        let config = IterativeLoopConfig {
            command: "echo".to_string(),
            args: vec![],
            prompt: "no promise here".to_string(),
            completion_promise: Some("NEVER_FOUND".to_string()),
            max_iterations: Some(3),
            ..Default::default()
        };

        let mut loop_exec = IterativeLoop::new(config).await.unwrap();
        let result = loop_exec.execute().await.unwrap();

        assert_eq!(result.exit_reason, IterativeExitReason::MaxIterationsReached);
        assert_eq!(result.iterations_completed, 3);
    }
}
```

### Integration Tests

Located in `descartes/core/tests/iterative_loop_tests.rs`:

```rust
use descartes_core::{IterativeLoop, IterativeLoopConfig, IterativeExitReason};
use tempfile::TempDir;

#[tokio::test]
async fn test_state_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let state_file = temp_dir.path().join("loop-state.json");

    // Start loop
    let config = IterativeLoopConfig {
        command: "echo".to_string(),
        prompt: "test".to_string(),
        completion_promise: Some("NEVER".to_string()),
        max_iterations: Some(2),
        state_file: Some(state_file.clone()),
        ..Default::default()
    };

    let mut loop_exec = IterativeLoop::new(config).await.unwrap();
    let _ = loop_exec.execute().await.unwrap();

    // Verify state file exists
    assert!(state_file.exists());

    // Load and verify state
    let content = std::fs::read_to_string(&state_file).unwrap();
    let state: descartes_core::IterativeLoopState = serde_json::from_str(&content).unwrap();
    assert_eq!(state.iteration, 2);
    assert!(state.completed);
}

#[tokio::test]
async fn test_resume() {
    let temp_dir = TempDir::new().unwrap();
    let state_file = temp_dir.path().join("loop-state.json");

    // Create initial state
    let initial_state = descartes_core::IterativeLoopState {
        iteration: 5,
        config: IterativeLoopConfig {
            command: "echo".to_string(),
            prompt: "<promise>DONE</promise>".to_string(),
            completion_promise: Some("DONE".to_string()),
            max_iterations: Some(10),
            ..Default::default()
        },
        ..Default::default()
    };

    let content = serde_json::to_string_pretty(&initial_state).unwrap();
    std::fs::write(&state_file, content).unwrap();

    // Resume
    let mut loop_exec = IterativeLoop::resume(state_file).await.unwrap();
    assert_eq!(loop_exec.current_iteration(), 5);

    let result = loop_exec.execute().await.unwrap();
    assert_eq!(result.exit_reason, IterativeExitReason::CompletionPromiseDetected);
}
```

### Manual Testing Steps

1. **Basic echo test**:
   ```bash
   descartes loop start --command "echo" --prompt "<promise>DONE</promise>" --max-iterations 5
   ```

2. **Claude Code test**:
   ```bash
   descartes loop start \
     --command "claude -p" \
     --prompt "Write a hello world script in Python. Output <promise>COMPLETE</promise> when done." \
     --completion-promise "COMPLETE" \
     --max-iterations 5 \
     --backend claude
   ```

3. **Resume test**:
   - Start a loop with high max-iterations
   - Press Ctrl+C after first iteration
   - Run `descartes loop resume`
   - Verify it continues from where it left off

4. **GUI test**:
   - Start GUI
   - Navigate to Loop view
   - Start a loop
   - Verify progress updates
   - Test cancel button

## Performance Considerations

- **Output buffering**: Limit stored output to prevent memory issues with verbose tools
- **State file writes**: Write state only after each iteration, not continuously
- **Process cleanup**: Ensure child processes are killed on cancellation

## Migration Notes

N/A - New feature, no migration needed.

## References

- Research document: `descartes/thoughts/shared/research/2025-12-28-iterative-agent-loop-ralph-style.md`
- Ralph-Wiggum plugin: https://github.com/anthropics/claude-plugins-official/tree/main/plugins/ralph-wiggum
- Existing patterns: `descartes/core/src/flow_executor.rs`, `descartes/core/src/agent_runner.rs`
