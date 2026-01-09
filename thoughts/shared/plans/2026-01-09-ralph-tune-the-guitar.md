# Implementation Plan: "Tune the Guitar" Feedback Loop for Ralph

## Overview

Add Geoffrey Huntley's "tune the guitar" mechanism to the SCUD loop: when tasks fail, automatically refine the prompt and retry, then checkpoint for human review if still failing. Track all attempt variants so humans can review and select the best approach.

## Current State Analysis

**What exists (`descartes/core/src/scud_loop.rs`):**
- Fresh context per task via `build_task_spec()`
- SCUD-based completion detection (not promise tags)
- Sub-agent spawning via `spawn_claude_agent()`
- Verification-based success via `run_verification()`
- `BlockedTask` struct captures failure reason

**Current failure behavior (lines 698-739):**
```rust
TaskExecutionResult::Blocked(reason) => {
    self.update_task_status(task.id, "blocked")?;
    self.state.blocked_tasks.push(BlockedTask { ... });
    warn!("Task {} blocked", task.id);
    // MOVES ON - no retry, no refinement
}
```

**Gap:** No mechanism to:
1. Analyze why the task failed
2. Refine the prompt based on failure
3. Retry with improved context
4. Present variants to human for review

## Desired End State

After implementation:

1. **Auto-refinement:** When a task fails, the loop automatically:
   - Captures failure context (output, stderr, diff)
   - Spawns a "tuner" agent to suggest prompt refinements
   - Retries with refined prompt (up to `max_tune_attempts`)

2. **Variant tracking:** Each attempt is recorded as a `TaskAttempt` with:
   - Prompt used
   - Agent output
   - Verification result
   - Suggested refinement (if failed)

3. **Human checkpoint:** After exhausting auto-retries:
   - Loop pauses with status `awaiting-tune`
   - State saved to `.scud/tune-state.json`
   - Human runs `descartes loop tune` to review variants

4. **Variant selection:** Human can:
   - View all attempts in a TUI or markdown report
   - Select a variant to iterate on
   - Optionally edit the prompt manually
   - Resume the loop

### Verification:

```bash
# Start a loop that will fail
descartes loop start --scud-tag test --max-tune-attempts 3

# After it pauses:
descartes loop tune --show-variants
descartes loop tune --select 2  # Use variant 2
descartes loop tune --edit      # Manual edit mode
descartes loop resume           # Continue with selected variant
```

## What We're NOT Doing

- **Cross-task learning:** Tuning is per-task, not project-wide
- **Persistent tuning database:** Tuning state is ephemeral per loop run
- **ML-based refinement:** Just prompt engineering, no fine-tuning
- **Automatic rollback:** If all variants fail, task stays blocked

## Implementation Approach

Add tuning as an optional layer that wraps the existing `execute_task()` flow:

```
execute_task()
    │
    ▼
┌───────────────────────────────────────────┐
│  TUNING LAYER (new)                        │
│                                            │
│  for attempt in 1..max_tune_attempts:      │
│    result = execute_task_attempt()         │
│    if success: return Success              │
│    if failed:                              │
│      refinement = spawn_tuner_agent()      │
│      apply_refinement_to_prompt()          │
│      save_attempt_variant()                │
│                                            │
│  all_attempts_failed:                      │
│    save_tune_state()                       │
│    return AwaitingTune                     │
└───────────────────────────────────────────┘
```

---

## Phase 1: Data Structures

### Overview
Add new types to track tuning attempts and state.

### Changes Required:

#### 1. New types in `scud_loop.rs`

**File**: `descartes/core/src/scud_loop.rs`

Add after `BlockedTask` struct (around line 280):

```rust
/// A single attempt at executing a task
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

/// Extended execution result
#[derive(Debug, Clone)]
pub enum TaskExecutionResult {
    Success,
    Blocked(String),
    Unknown,
    /// New: Task needs human tuning intervention
    AwaitingTune(TaskTuneState),
}
```

#### 2. Add tuning config to `ScudLoopConfig`

**File**: `descartes/core/src/scud_loop.rs`

Add to `ScudLoopConfig` struct (around line 165):

```rust
/// Tuning configuration
#[serde(default)]
pub tune: TuneConfig,
```

And add the config struct:

```rust
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

fn default_tune_attempts() -> u32 { 3 }
fn default_tune_output_tokens() -> usize { 2000 }

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
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo check -p descartes-core` passes
- [x] `cargo test -p descartes-core` passes
- [x] New types serialize/deserialize correctly

#### Manual Verification:
- [x] Types make sense for the use case

---

## Phase 2: Tuner Agent

### Overview
Create the agent that analyzes failures and suggests prompt refinements.

### Changes Required:

#### 1. Tuner agent prompt builder

**File**: `descartes/core/src/scud_loop.rs`

Add method to `ScudIterativeLoop`:

```rust
/// Build prompt for the tuner agent that suggests refinements
fn build_tuner_prompt(&self, task: &LoopTask, attempt: &TaskAttempt) -> String {
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
        if let Some(ref diff) = attempt.git_diff {
            format!("### Git Diff\n```diff\n{}\n```", diff)
        } else {
            String::new()
        }
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
```

#### 2. Spawn tuner agent

**File**: `descartes/core/src/scud_loop.rs`

Add method:

```rust
/// Spawn the tuner agent to suggest prompt refinements
async fn spawn_tuner_agent(&self, task: &LoopTask, attempt: &TaskAttempt) -> Result<Option<String>> {
    let prompt = self.build_tuner_prompt(task, attempt);

    info!("Spawning tuner agent to analyze failure for task {}", task.id);

    let output = self.spawn_claude_agent(&prompt).await?;

    Ok(self.parse_tuner_output(&output))
}
```

#### 3. Capture git diff helper

**File**: `descartes/core/src/scud_loop.rs`

Add method:

```rust
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
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo check -p descartes-core` passes
- [x] All existing tests pass
- [x] New `build_tuner_prompt` test passes
- [x] New `parse_tuner_output` test passes

#### Manual Verification:
- [x] Review prompt structure for clarity

---

## Phase 3: Execute with Tuning

### Overview
Replace the simple `execute_task` flow with tuning-aware version.

### Changes Required:

#### 1. New execution method with tuning

**File**: `descartes/core/src/scud_loop.rs`

Add method:

```rust
/// Execute a task with automatic tuning on failure
async fn execute_task_with_tuning(&self, task: &LoopTask) -> Result<TaskExecutionResult> {
    if !self.config.tune.enabled {
        // Fall back to simple execution
        return self.execute_task(task).await;
    }

    let mut attempts: Vec<TaskAttempt> = Vec::new();
    let mut current_prompt = self.build_task_spec(task)?;
    let base_prompt = current_prompt.clone();

    for attempt_num in 1..=self.config.tune.max_attempts {
        info!("Task {} attempt {}/{}", task.id, attempt_num, self.config.tune.max_attempts);

        // Build full prompt with any accumulated refinements
        let full_prompt = self.build_task_prompt(&current_prompt, task)?;

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
            prompt: current_prompt.clone(),
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
                current_prompt = format!(
                    "{}\n\n---\n\n## Additional Guidance (from previous failure)\n\n{}",
                    base_prompt, refinement
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
```

#### 2. Update main execute loop

**File**: `descartes/core/src/scud_loop.rs`

In the `execute()` method, replace `self.execute_task(&task).await?` with `self.execute_task_with_tuning(&task).await?` and handle the new `AwaitingTune` variant:

```rust
// Execute task with tuning
let result = self.execute_task_with_tuning(&task).await?;

match result {
    TaskExecutionResult::Success => {
        // ... existing success handling ...
    }
    TaskExecutionResult::Blocked(reason) => {
        // ... existing blocked handling ...
    }
    TaskExecutionResult::Unknown => {
        // ... existing unknown handling ...
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
```

#### 3. Add new exit reason

**File**: `descartes/core/src/iterative_loop.rs`

Add to `IterativeExitReason` enum:

```rust
/// Waiting for human to tune a failed task
AwaitingHumanTune,
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo check -p descartes-core` passes
- [x] `cargo test -p descartes-core` passes

#### Manual Verification:
- [x] Loop pauses correctly when task fails after max attempts
- [x] Tune state file contains all attempts
- [x] Git changes are reverted between attempts

---

## Phase 4: CLI Tune Command

### Overview
Add `descartes loop tune` subcommand for human review.

### Changes Required:

#### 1. Add tune subcommand

**File**: `descartes/cli/src/commands/loop_cmd.rs`

Add to `LoopCommand` enum:

```rust
/// Review and tune failed tasks
Tune(LoopTuneArgs),
```

Add args struct:

```rust
#[derive(Debug, Args)]
pub struct LoopTuneArgs {
    /// Show all attempt variants
    #[arg(long)]
    pub show_variants: bool,

    /// Select a variant by number (1-indexed)
    #[arg(long)]
    pub select: Option<u32>,

    /// Edit the prompt manually
    #[arg(long)]
    pub edit: bool,

    /// Path to tune state file
    #[arg(long)]
    pub state_file: Option<PathBuf>,

    /// Output format: text, json, markdown
    #[arg(long, default_value = "text")]
    pub format: String,
}
```

#### 2. Implement tune handler

**File**: `descartes/cli/src/commands/loop_cmd.rs`

```rust
async fn handle_tune(args: &LoopTuneArgs) -> Result<()> {
    use colored::Colorize;
    use descartes_core::TaskTuneState;

    let state_file = args.state_file.clone()
        .unwrap_or_else(|| PathBuf::from(".scud/tune-state.json"));

    let content = tokio::fs::read_to_string(&state_file).await
        .context("No tune state found. Is there a task awaiting tune?")?;
    let mut state: TaskTuneState = serde_json::from_str(&content)?;

    if args.show_variants || (!args.edit && args.select.is_none()) {
        // Display all variants
        println!("{}", format!("Task {}: {}", state.task_id, state.task_title).cyan().bold());
        println!("{}", "=".repeat(60).dimmed());
        println!();

        for attempt in &state.attempts {
            let status = if attempt.verification_passed {
                "✓ PASSED".green()
            } else {
                "✗ FAILED".red()
            };

            println!("{} Attempt {} {}", "─".repeat(20).dimmed(), attempt.attempt, status);
            println!();

            if args.format == "text" {
                // Truncated view
                println!("{}", "Prompt (truncated):".yellow());
                println!("{}", attempt.prompt.chars().take(500).collect::<String>());
                if attempt.prompt.len() > 500 { println!("..."); }
                println!();

                println!("{}", "Verification Error:".red());
                println!("{}", attempt.verification_stderr.chars().take(300).collect::<String>());
                println!();

                if let Some(ref refinement) = attempt.suggested_refinement {
                    println!("{}", "Suggested Refinement:".green());
                    println!("{}", refinement);
                    println!();
                }
            } else if args.format == "markdown" {
                // Full markdown output
                println!("### Prompt\n```\n{}\n```\n", attempt.prompt);
                println!("### Error\n```\n{}\n```\n", attempt.verification_stderr);
                if let Some(ref diff) = attempt.git_diff {
                    println!("### Diff\n```diff\n{}\n```\n", diff);
                }
            }
        }

        println!("{}", "─".repeat(60).dimmed());
        println!();
        println!("Commands:");
        println!("  {} - Select variant N to retry", "descartes loop tune --select N".cyan());
        println!("  {} - Edit prompt manually", "descartes loop tune --edit".cyan());
        println!("  {} - Resume with selected variant", "descartes loop resume".cyan());
    }

    if let Some(variant) = args.select {
        if variant < 1 || variant > state.attempts.len() as u32 {
            return Err(anyhow::anyhow!("Invalid variant number. Choose 1-{}", state.attempts.len()));
        }

        state.selected_variant = Some(variant);
        let content = serde_json::to_string_pretty(&state)?;
        tokio::fs::write(&state_file, content).await?;

        println!("{}", format!("Selected variant {}. Run `descartes loop resume` to continue.", variant).green());
    }

    if args.edit {
        // Open editor for manual prompt editing
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        // Write current best prompt to temp file
        let temp_path = state_file.with_extension("prompt.md");
        let best_prompt = state.attempts.last()
            .map(|a| a.prompt.clone())
            .unwrap_or_default();

        tokio::fs::write(&temp_path, &best_prompt).await?;

        // Open editor
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .context("Failed to open editor")?;

        if status.success() {
            let edited = tokio::fs::read_to_string(&temp_path).await?;
            state.custom_prompt = Some(edited);

            let content = serde_json::to_string_pretty(&state)?;
            tokio::fs::write(&state_file, content).await?;

            println!("{}", "Custom prompt saved. Run `descartes loop resume` to continue.".green());
        }

        tokio::fs::remove_file(&temp_path).await.ok();
    }

    Ok(())
}
```

#### 3. Update resume to handle tune state

**File**: `descartes/cli/src/commands/loop_cmd.rs`

In `handle_resume`, add tune state handling:

```rust
async fn handle_resume(args: &LoopResumeArgs) -> Result<()> {
    // ... existing code ...

    // Check for tune state
    let tune_state_file = PathBuf::from(".scud/tune-state.json");
    if tune_state_file.exists() {
        let content = tokio::fs::read_to_string(&tune_state_file).await?;
        let tune_state: TaskTuneState = serde_json::from_str(&content)?;

        if tune_state.selected_variant.is_none() && tune_state.custom_prompt.is_none() {
            return Err(anyhow::anyhow!(
                "Tune state exists but no variant selected. Run `descartes loop tune --select N` first."
            ));
        }

        println!("{}", "Resuming with tuned prompt...".cyan());
        // The ScudIterativeLoop::resume will pick up the tune state
    }

    // ... rest of existing code ...
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo check -p descartes-cli` passes
- [x] `descartes loop tune --help` shows options

#### Manual Verification:
- [x] `descartes loop tune` displays variants nicely
- [x] `descartes loop tune --select N` updates state
- [x] `descartes loop tune --edit` opens editor
- [x] `descartes loop resume` works after selection

---

## Phase 5: Resume with Tuned Prompt

### Overview
Modify `ScudIterativeLoop::resume` to pick up tune state and retry with selected/custom prompt.

### Changes Required:

#### 1. Update resume logic

**File**: `descartes/core/src/scud_loop.rs`

Modify `resume` method:

```rust
/// Resume from existing state file, optionally with tune state
pub async fn resume(state_file: PathBuf) -> Result<Self> {
    let content = tokio::fs::read_to_string(&state_file)
        .await
        .context("Failed to read state file")?;
    let state: ScudLoopState = serde_json::from_str(&content)
        .context("Failed to parse state file")?;

    let mut executor = Self {
        config: state.config.clone(),
        state,
    };

    // Check for tune state
    executor.load_tune_state().await?;

    Ok(executor)
}

/// Load tune state if exists
async fn load_tune_state(&mut self) -> Result<()> {
    let tune_path = self.config.tune.tune_state_file.clone()
        .unwrap_or_else(|| self.config.working_directory.join(".scud/tune-state.json"));

    if tune_path.exists() {
        let content = tokio::fs::read_to_string(&tune_path).await?;
        let tune_state: TaskTuneState = serde_json::from_str(&content)?;

        // Store for use in next execute
        self.pending_tune_state = Some(tune_state);
    }

    Ok(())
}
```

Add field to struct:

```rust
pub struct ScudIterativeLoop {
    config: ScudLoopConfig,
    state: ScudLoopState,
    /// Pending tune state from human intervention
    pending_tune_state: Option<TaskTuneState>,
}
```

#### 2. Use tuned prompt in execute

Modify `execute_task_with_tuning` to check for pending tune state:

```rust
async fn execute_task_with_tuning(&mut self, task: &LoopTask) -> Result<TaskExecutionResult> {
    // Check if we have a tuned prompt for this task
    if let Some(ref tune_state) = self.pending_tune_state {
        if tune_state.task_id == task.id {
            let tuned_prompt = if let Some(ref custom) = tune_state.custom_prompt {
                custom.clone()
            } else if let Some(variant) = tune_state.selected_variant {
                tune_state.attempts.get((variant - 1) as usize)
                    .map(|a| a.prompt.clone())
                    .unwrap_or_else(|| self.build_task_spec(task).unwrap_or_default())
            } else {
                self.build_task_spec(task)?
            };

            info!("Using tuned prompt for task {}", task.id);

            // Clear tune state after use
            self.pending_tune_state = None;
            self.clear_tune_state().await?;

            // Execute with tuned prompt (single attempt, no auto-tune loop)
            return self.execute_single_attempt(task, &tuned_prompt).await;
        }
    }

    // ... rest of existing tuning loop ...
}

async fn clear_tune_state(&self) -> Result<()> {
    let tune_path = self.config.tune.tune_state_file.clone()
        .unwrap_or_else(|| self.config.working_directory.join(".scud/tune-state.json"));

    if tune_path.exists() {
        tokio::fs::remove_file(&tune_path).await?;
    }

    Ok(())
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo check -p descartes-core` passes
- [x] `cargo test -p descartes-core` passes

#### Manual Verification:
- [x] Resume picks up selected variant
- [x] Custom prompt is used when provided
- [x] Tune state is cleared after successful use

---

## Phase 6: CLI Options & Slash Commands

### Overview
Add CLI options for tuning config and update slash commands.

### Changes Required:

#### 1. CLI options

**File**: `descartes/cli/src/commands/loop_cmd.rs`

Add to `LoopStartArgs`:

```rust
/// Enable automatic prompt tuning on failure
#[arg(long, default_value = "true")]
pub tune: bool,

/// Max auto-tune attempts before human checkpoint
#[arg(long, default_value = "3")]
pub max_tune_attempts: u32,
```

Update `handle_start` to use them:

```rust
let config = ScudLoopConfig {
    // ... existing fields ...
    tune: TuneConfig {
        enabled: args.tune,
        max_attempts: args.max_tune_attempts,
        ..Default::default()
    },
};
```

#### 2. Update slash commands

**File**: `.claude/commands/ralph-wiggum/ralph-loop.md`

Add documentation:

```markdown
## Tuning Options

- `--tune` / `--no-tune` - Enable/disable auto-tuning (default: enabled)
- `--max-tune-attempts <n>` - Auto-retry attempts before human checkpoint (default: 3)

## When Tasks Fail

If a task fails after max attempts, the loop pauses:

1. Run `descartes loop tune` to review all attempts
2. Select a variant: `descartes loop tune --select 2`
3. Or edit manually: `descartes loop tune --edit`
4. Resume: `descartes loop resume`
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo check` passes
- [x] `descartes loop start --help` shows tuning options

#### Manual Verification:
- [x] Options work as documented

---

## Phase 7: Tests & Documentation

### Overview
Add tests and documentation for the tuning feature.

### Changes Required:

#### 1. Unit tests

**File**: `descartes/core/src/scud_loop.rs`

```rust
#[cfg(test)]
mod tune_tests {
    use super::*;

    #[test]
    fn test_parse_tuner_output() {
        let loop_exec = create_test_loop();

        let output = "Analysis: The agent didn't handle errors.\n\nREFINEMENT: Add explicit error handling for the case when the file doesn't exist.";
        let refinement = loop_exec.parse_tuner_output(output);

        assert!(refinement.is_some());
        assert!(refinement.unwrap().contains("error handling"));
    }

    #[test]
    fn test_parse_tuner_output_no_refinement() {
        let loop_exec = create_test_loop();

        let output = "I'm not sure what went wrong.";
        let refinement = loop_exec.parse_tuner_output(output);

        assert!(refinement.is_none());
    }

    #[test]
    fn test_task_attempt_serialization() {
        let attempt = TaskAttempt {
            attempt: 1,
            prompt: "Test prompt".to_string(),
            agent_output: "Output".to_string(),
            verification_stdout: "".to_string(),
            verification_stderr: "error".to_string(),
            verification_passed: false,
            git_diff: Some("diff".to_string()),
            suggested_refinement: Some("Try X".to_string()),
            attempted_at: Utc::now(),
        };

        let json = serde_json::to_string(&attempt).unwrap();
        let parsed: TaskAttempt = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.attempt, 1);
        assert_eq!(parsed.verification_passed, false);
    }

    #[test]
    fn test_tune_config_defaults() {
        let config = TuneConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_attempts, 3);
        assert!(config.include_git_diff);
    }
}
```

#### 2. Documentation

**File**: `descartes/docs/blog/13-tune-the-guitar.md`

```markdown
# "Tune the Guitar" - Prompt Refinement Loop

Based on Geoffrey Huntley's Ralph Wiggum technique, the SCUD loop now supports automatic prompt refinement when tasks fail.

## How It Works

1. **Task fails** → Agent attempted but verification didn't pass
2. **Analyze failure** → Tuner agent examines output and suggests refinement
3. **Retry with refinement** → Prompt updated with guidance from failure
4. **Repeat** → Up to N times (default: 3)
5. **Human checkpoint** → If still failing, pause for human review

## Configuration

```bash
descartes loop start \
    --scud-tag my-feature \
    --tune                    # Enable tuning (default)
    --max-tune-attempts 5     # Retries before human checkpoint
```

## Human Review

When a task needs human intervention:

```bash
# View all attempts
descartes loop tune

# Select a variant
descartes loop tune --select 2

# Edit prompt manually
descartes loop tune --edit

# Resume
descartes loop resume
```

## Philosophy

From Geoffrey Huntley:
> "Ralph is deterministically bad in an undeterministic world. When it fails, tune the guitar - add signage saying 'SLIDE DOWN, DON'T JUMP'."

The tuning loop implements this by:
- Capturing failure context (output, errors, diff)
- Having Claude analyze what went wrong
- Adding specific guidance to prevent the same failure
- Preserving all variants for human review
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo test -p descartes-core` passes
- [x] All new tests pass

#### Manual Verification:
- [x] Documentation is clear and accurate

---

## Implementation Order

Based on dependencies:

| Wave | Tasks | Dependencies |
|------|-------|--------------|
| 1 | Phase 1 (Data Structures) | None |
| 2 | Phase 2 (Tuner Agent) | Phase 1 |
| 3 | Phase 3 (Execute with Tuning) | Phase 1, 2 |
| 4 | Phase 4 (CLI Tune Command) | Phase 1, 3 |
| 5 | Phase 5 (Resume with Tune) | Phase 3, 4 |
| 6 | Phase 6 (CLI Options & Slash Commands) | Phase 4, 5 |
| 7 | Phase 7 (Tests & Documentation) | All above |

## Success Criteria Summary

1. **Auto-refinement works**: Failed tasks are retried with refined prompts
2. **Variants tracked**: All attempts saved with full context
3. **Human checkpoint works**: Loop pauses after max attempts
4. **Review flow works**: `descartes loop tune` shows variants nicely
5. **Selection works**: Can select variant or edit manually
6. **Resume works**: Continues with selected/custom prompt
7. **Tests pass**: All automated verification green

## References

- Geoffrey Huntley's Ralph Wiggum: https://ghuntley.com/ralph/
- `ralph_convo.md` - Conversation with Geoff about the technique
- `thoughts/shared/research/2026-01-08-ralph-loop-scud-integration.md` - Research doc
- Current implementation: `descartes/core/src/scud_loop.rs`
