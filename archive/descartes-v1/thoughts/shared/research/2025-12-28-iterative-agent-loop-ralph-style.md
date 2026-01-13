---
date: 2025-12-28T18:32:45Z
researcher: Claude Code
git_commit: 2cf0a97443ed932d99fdd7c6f3a1054ab10210f0
branch: master
repository: backbone
topic: "Iterative Agent Loop - Ralph Wiggum Style Implementation for Arbitrary Agents"
tags: [research, codebase, ralph-loop, iterative-agents, orchestration]
status: complete
last_updated: 2025-12-28
last_updated_by: Claude Code
---

# Research: Iterative Agent Loop - Ralph Wiggum Style Implementation

**Date**: 2025-12-28T18:32:45Z
**Researcher**: Claude Code
**Git Commit**: 2cf0a97443ed932d99fdd7c6f3a1054ab10210f0
**Branch**: master
**Repository**: backbone

## Research Question

User asked: "Can we implement [ralph-wiggum plugin concept] directly, for any arbitrary agent? And the ability to use with whatever command?"

## Summary

The ralph-wiggum plugin implements an **iterative, self-referential AI development loop** that repeatedly feeds the same prompt to an agent until a completion signal is detected. Descartes already has all the foundational infrastructure to implement this pattern generically:

1. **Agent lifecycle management** via `LocalProcessRunner` and `AgentHandle`
2. **Stream parsing** with completion detection via `AgentStreamParser` and `StreamChunk`
3. **State persistence** patterns from `FlowState` and `PhaseState`
4. **Retry/loop patterns** from `execute_phase_with_retry()` in flow_executor
5. **Tool interception** via `InterceptingClaudeBackend` for detecting custom signals

A generic `IterativeLoop` executor could wrap any CLI command and implement the ralph pattern without modifying the underlying tool.

## Detailed Findings

### 1. Ralph-Wiggum Plugin Mechanics

**Source**: https://github.com/anthropics/claude-plugins-official/tree/main/plugins/ralph-wiggum

**Core Mechanism**:
- User runs `/ralph-loop "<prompt>" --max-iterations N --completion-promise "<text>"`
- Claude Code starts working on the task
- When Claude attempts to exit, a **Stop hook** intercepts
- Hook checks for `<promise>...</promise>` tags in output
- If completion promise not found and iterations remaining, **blocks exit and re-feeds same prompt**
- Claude sees its previous work in modified files and git history
- Loop continues until completion promise detected or max iterations reached

**State Storage**:
- State stored in `.claude/ralph-loop.local.md`
- Contains: iteration count, max iterations, completion promise, original prompt

**Key Insight**: Claude improves iteratively by reading its own past work (files, git history) without external intervention.

### 2. Descartes Infrastructure for Implementation

#### 2.1 Agent Spawning (`agent_runner.rs:340-409`)

Descartes already spawns CLI agents as child processes with piped stdio:

```rust
// agent_runner.rs:350-358
fn build_command(&self, config: &AgentConfig) -> Command {
    let mut cmd = Command::new(&config.command);
    cmd.args(&config.args)
       .stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());
    cmd
}
```

**Supports**: Any CLI command, not just Claude Code. The `AgentConfig` takes arbitrary `command` and `args`.

#### 2.2 Stream Parsing for Completion Detection (`agent_stream_parser.rs:247-289`)

The `AgentStreamParser` processes NDJSON output and can detect various signals:

```rust
// agent_stream_parser.rs:83-130 - StreamHandler trait
pub trait StreamHandler: Send + Sync {
    fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, timestamp: Option<DateTime<Utc>>);
    fn on_output(&mut self, agent_id: Uuid, stream: OutputStream, content: &str, timestamp: Option<DateTime<Utc>>);
    // ... other hooks
}
```

**For ralph-style**: We'd implement a `StreamHandler` that watches for `<promise>` tags in output.

#### 2.3 Retry Loop Pattern (`flow_executor.rs:629-679`)

The flow executor already has a retry-until-success pattern with orchestrator decisions:

```rust
// flow_executor.rs:629-679
async fn execute_phase_with_retry(&mut self, phase: &str) -> Result<()> {
    let max_retries = self.state.config.max_retries_per_phase;

    loop {
        let retry_count = self.get_phase_retry_count(phase);
        if retry_count >= max_retries {
            return Err(anyhow::anyhow!("Phase '{}' failed after {} retries", phase, retry_count));
        }

        match self.execute_phase_with_timeout(phase).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                self.increment_phase_retry(phase);
                let decision = self.get_orchestrator_decision(phase, &e).await?;
                // Handle Retry/Skip/Abort/Continue decisions
            }
        }
    }
}
```

**Key pattern**: Loop with max iterations, success/failure detection, decision-based continuation.

#### 2.4 State Persistence (`flow_executor.rs:890-901`)

Flow state is persisted to disk for resume:

```rust
// flow_executor.rs:890-901
async fn save_state(&self) -> Result<()> {
    let content = serde_json::to_string_pretty(&self.state)?;
    fs::write(&self.state_path, content).await?;
    Ok(())
}
```

**For ralph-style**: Store iteration count, prompt, completion promise in similar state file.

#### 2.5 Exit Status Observation (`agent_runner.rs:291-329`)

The exit observer monitors when the agent process terminates:

```rust
// agent_runner.rs:303-314
match child_guard.try_wait() {
    Ok(Some(status)) => {
        let exit_status = ExitStatus {
            code: status.code(),
            success: status.success(),
        };
        handle_write.record_exit_status(exit_status);
        break;
    }
    // ...
}
```

**For ralph-style**: On process exit, check completion state, decide whether to respawn.

### 3. Proposed Design: Generic `IterativeLoop` Executor

```rust
/// Configuration for an iterative agent loop
pub struct IterativeLoopConfig {
    /// The command to run (e.g., "claude", "opencode", "python", etc.)
    pub command: String,

    /// Base arguments for the command
    pub args: Vec<String>,

    /// The task prompt to feed the agent
    pub prompt: String,

    /// Optional completion promise - loop exits when this text appears in output
    pub completion_promise: Option<String>,

    /// Maximum iterations before forced exit (safety mechanism)
    pub max_iterations: Option<u32>,

    /// Working directory for the agent
    pub working_directory: PathBuf,

    /// State file path for persistence
    pub state_file: PathBuf,

    /// Whether to include iteration context in prompt
    pub include_iteration_context: bool,

    /// Timeout per iteration in seconds
    pub iteration_timeout_secs: Option<u64>,
}

/// State persisted between iterations
#[derive(Serialize, Deserialize)]
pub struct IterativeLoopState {
    pub iteration: u32,
    pub max_iterations: Option<u32>,
    pub completion_promise: Option<String>,
    pub prompt: String,
    pub started_at: DateTime<Utc>,
    pub last_iteration_at: Option<DateTime<Utc>>,
    pub completed: bool,
    pub completion_detected_at: Option<DateTime<Utc>>,
}

/// Result of the iterative loop
pub struct IterativeLoopResult {
    pub iterations_completed: u32,
    pub completion_promise_found: bool,
    pub final_output: String,
    pub exit_reason: IterativeExitReason,
}

pub enum IterativeExitReason {
    CompletionPromiseDetected,
    MaxIterationsReached,
    UserCancelled,
    Error(String),
}
```

### 4. Implementation Approach

#### 4.1 The Main Loop

```rust
impl IterativeLoop {
    pub async fn execute(&mut self) -> Result<IterativeLoopResult> {
        loop {
            // 1. Check iteration limits
            if let Some(max) = self.config.max_iterations {
                if self.state.iteration >= max {
                    return Ok(IterativeLoopResult {
                        exit_reason: IterativeExitReason::MaxIterationsReached,
                        // ...
                    });
                }
            }

            // 2. Build prompt with optional iteration context
            let iteration_prompt = self.build_iteration_prompt();

            // 3. Spawn agent process
            let handle = self.spawn_agent(&iteration_prompt).await?;

            // 4. Capture output while monitoring for completion promise
            let output = self.capture_output_with_detection(&handle).await?;

            // 5. Check for completion promise in output
            if self.check_completion_promise(&output) {
                self.state.completed = true;
                self.state.completion_detected_at = Some(Utc::now());
                self.save_state().await?;

                return Ok(IterativeLoopResult {
                    exit_reason: IterativeExitReason::CompletionPromiseDetected,
                    // ...
                });
            }

            // 6. Increment iteration and persist state
            self.state.iteration += 1;
            self.state.last_iteration_at = Some(Utc::now());
            self.save_state().await?;

            // 7. Continue loop - agent will see its previous work via files/git
        }
    }
}
```

#### 4.2 Completion Promise Detection

Multiple detection strategies:

1. **Simple string matching** (ralph-style):
   ```rust
   fn check_completion_promise(&self, output: &str) -> bool {
       if let Some(ref promise) = self.config.completion_promise {
           output.contains(&format!("<promise>{}</promise>", promise))
       } else {
           false
       }
   }
   ```

2. **Stream-based detection** (for real-time):
   ```rust
   impl StreamHandler for CompletionDetector {
       fn on_output(&mut self, _: Uuid, _: OutputStream, content: &str, _: Option<DateTime<Utc>>) {
           if let Some(ref promise) = self.promise {
               if content.contains(&format!("<promise>{}</promise>", promise)) {
                   self.completion_detected = true;
               }
           }
       }
   }
   ```

3. **Exit code based** (for CLI tools with specific exit codes):
   ```rust
   if exit_status.code == Some(0) && self.config.treat_zero_as_complete {
       return true;
   }
   ```

#### 4.3 Prompt Building with Context

```rust
fn build_iteration_prompt(&self) -> String {
    let mut prompt = self.config.prompt.clone();

    if self.config.include_iteration_context && self.state.iteration > 0 {
        prompt = format!(
            "{}\n\n---\n\
            ITERATION CONTEXT:\n\
            - This is iteration {} of {}\n\
            - Your previous work persists in files and git history\n\
            - Review what you've done and continue improving\n\
            - Output <promise>{}</promise> when the task is completely finished\n\
            ---",
            prompt,
            self.state.iteration + 1,
            self.config.max_iterations.map(|m| m.to_string()).unwrap_or("unlimited".to_string()),
            self.config.completion_promise.as_deref().unwrap_or("COMPLETE")
        );
    }

    prompt
}
```

### 5. Usage Examples

#### 5.1 With Claude Code

```rust
let config = IterativeLoopConfig {
    command: "claude".to_string(),
    args: vec!["-p".to_string(), "--output-format".to_string(), "stream-json".to_string()],
    prompt: "Build a REST API for todos with CRUD operations and tests. \
             Output <promise>COMPLETE</promise> when done.".to_string(),
    completion_promise: Some("COMPLETE".to_string()),
    max_iterations: Some(20),
    working_directory: PathBuf::from("/path/to/project"),
    state_file: PathBuf::from(".descartes/ralph-loop.json"),
    include_iteration_context: true,
    iteration_timeout_secs: Some(3600), // 1 hour per iteration
};

let mut loop_executor = IterativeLoop::new(config)?;
let result = loop_executor.execute().await?;
```

#### 5.2 With Arbitrary Python Script

```rust
let config = IterativeLoopConfig {
    command: "python".to_string(),
    args: vec!["train_model.py".to_string()],
    prompt: String::new(), // Prompt handled by the script
    completion_promise: Some("TRAINING_COMPLETE".to_string()),
    max_iterations: Some(100),
    working_directory: PathBuf::from("/path/to/ml-project"),
    state_file: PathBuf::from(".descartes/training-loop.json"),
    include_iteration_context: false,
    iteration_timeout_secs: Some(7200), // 2 hours per iteration
};
```

#### 5.3 As a CLI Command

```bash
# Proposed descartes CLI command
descartes loop \
    --command "claude -p" \
    --prompt "Build a CLI tool for managing tasks" \
    --completion-promise "COMPLETE" \
    --max-iterations 30 \
    --working-dir ./my-project
```

### 6. Integration Points in Descartes

| Component | File | Integration |
|-----------|------|-------------|
| Agent spawning | `agent_runner.rs:340` | Reuse `LocalProcessRunner::spawn()` |
| Stream parsing | `agent_stream_parser.rs:247` | Reuse for completion detection |
| State persistence | `flow_executor.rs:890` | Copy pattern for loop state |
| Exit observation | `agent_runner.rs:291` | Reuse for process monitoring |
| Retry logic | `flow_executor.rs:629` | Adapt loop pattern |

### 7. Key Differences from Ralph-Wiggum

| Aspect | Ralph-Wiggum | Proposed Descartes |
|--------|--------------|-------------------|
| Hook mechanism | Bash stop hook | Rust loop executor |
| Scope | Claude Code only | Any CLI command |
| State storage | Markdown file | JSON state file |
| Detection | Exit interception | Process completion + output parsing |
| Context passing | Same prompt only | Configurable iteration context |
| Integration | Plugin system | Native library |

## Code References

- `descartes/core/src/agent_runner.rs:340` - Agent spawning implementation
- `descartes/core/src/agent_runner.rs:291-329` - Exit status observation
- `descartes/core/src/flow_executor.rs:629-679` - Retry loop pattern
- `descartes/core/src/flow_executor.rs:890-901` - State persistence
- `descartes/core/src/agent_stream_parser.rs:83-130` - StreamHandler trait
- `descartes/core/src/agent_stream_parser.rs:247-289` - Stream processing
- `descartes/core/src/cli_backend.rs:14-57` - StreamChunk event types

## Architecture Documentation

**Existing Patterns Used**:
1. **Factory pattern** - `LocalProcessRunner::new()` and `with_config()`
2. **Observer pattern** - Health checker and exit observer
3. **Strategy pattern** - Different command building per backend
4. **State machine pattern** - Agent status transitions
5. **Callback pattern** - Tool interception callbacks

**New Pattern Introduced**:
- **Iterative executor pattern** - Loop-until-condition with state persistence

## Related Research

- `descartes/thoughts/shared/plans/2025-12-26-autonomous-flow-workflow.md` - Related flow orchestration
- `docs/prd/` - Product requirements that may benefit from iterative execution

## Open Questions

1. **Stdin injection vs respawn**: Should we inject prompts to a running process (like Claude's stream-json mode) or respawn for each iteration?
   - Respawn is simpler and works with any CLI
   - Injection is more efficient but requires tool support

2. **Git checkpointing**: Should each iteration create a git commit for easier debugging/rollback?
   - Ralph-wiggum relies on git history naturally
   - Explicit checkpoints could help with complex tasks

3. **Cancellation UX**: How should users cancel a running loop?
   - Ctrl+C with graceful shutdown
   - State file with "cancelled" flag
   - Named pipe or socket for control

4. **Multi-agent loops**: Could we run multiple iterative loops in parallel on different tasks?
   - Each with its own state file
   - Shared working directory or separate

5. **Web UI integration**: How would this integrate with the GUI?
   - Progress bar for iterations
   - Real-time output display
   - Cancel button
