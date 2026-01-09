# Iterative Loops

*Autonomous task execution with automatic completion detection*

---

What if your agent could keep working until the job is actually done? Not just run once and hope, but iterate, learn from its progress, and signal when it has truly completed the task.

Descartes' iterative loop system implements the "Ralph Wiggum" pattern: repeatedly execute a command until completion is detected. Named after [Geoffrey Huntley's technique](https://ghuntley.com/ralph/) of having an agent run in a loop, seeing its own previous work, and improving until it signals done.

**Key principles:**
- Same specification fed each iteration (fresh context)
- Agent sees previous work in files and git history
- External orchestration (not model-managed)
- Deterministic failures enable systematic improvement

## Quick Start

Start a loop that runs Claude until it outputs `<promise>COMPLETE</promise>`:

```bash
descartes loop start \
  --command "claude -p" \
  --prompt "Implement authentication. Output <promise>COMPLETE</promise> when done." \
  --max-iterations 20
```

The agent will:
1. Execute the command with your prompt
2. Check output for the completion promise
3. If not found, run again with iteration context
4. Repeat until promise detected or limit reached

---

## CLI Commands

### loop start

Start a new iterative loop:

```bash
descartes loop start \
  --command "claude -p" \
  --prompt "Your task description" \
  --completion-promise "DONE" \
  --max-iterations 10 \
  --backend claude \
  --auto-commit \
  --timeout 300 \
  --working-dir /path/to/project
```

| Flag | Default | Description |
|------|---------|-------------|
| `--command` | (required) | Command to execute (e.g., "claude -p", "opencode run") |
| `--prompt` | (required) | Task prompt for the agent |
| `--completion-promise` | `COMPLETE` | Text that signals task completion |
| `--max-iterations` | `20` | Safety limit on iterations |
| `--backend` | `generic` | Backend type: `claude`, `opencode`, or `generic` |
| `--auto-commit` | `false` | Auto-commit after each iteration |
| `--timeout` | (none) | Timeout per iteration in seconds |
| `--working-dir` | (cwd) | Working directory for execution |

### loop status

Show the current state of a running or completed loop:

```bash
descartes loop status

# Output:
# Loop Status
# ===========
#   State file: .descartes/loop-state.json
#   Iteration: 5
#   Started: 2025-01-15T10:30:00Z
#   Completed: No
#   Exit reason: Running
#
# Config:
#   Command: claude
#   Max iterations: 20
#   Completion promise: "COMPLETE"
```

### loop resume

Resume an interrupted loop from its state file:

```bash
descartes loop resume

# Or specify a custom state file:
descartes loop resume --state-file /path/to/loop-state.json
```

### loop cancel

Cancel a running loop:

```bash
descartes loop cancel
```

This updates the state file to mark the loop as cancelled.

---

## How It Works

### Completion Detection

The loop monitors agent output for a completion signal. Two formats are recognized:

**Tagged format (recommended):**
```
<promise>COMPLETE</promise>
```

**Plain text (case-insensitive):**
```
COMPLETE
```

The tagged format is more reliable as it reduces false positives from the agent discussing completion without actually being done.

### Exit Conditions

| Exit Reason | Description |
|-------------|-------------|
| `CompletionPromiseDetected` | Success - promise text found in output |
| `MaxIterationsReached` | Safety limit hit |
| `UserCancelled` | User interrupted with Ctrl+C or cancel |
| `ProcessSuccess` | Exit code 0 (when no promise configured) |
| `Error` | Command failed to execute |

### Iteration Context

After the first iteration, subsequent runs receive additional context:

```
---
ITERATION CONTEXT:
- This is iteration 3 of 20
- Your previous work persists in files and git history
- Review what you've done and continue improving
- Output <promise>COMPLETE</promise> when the task is completely finished
---
```

This helps the agent understand:
- Which iteration it's on
- That its previous changes are visible
- How to signal completion

### State Persistence

Loop state is persisted to `.descartes/loop-state.json`:

```json
{
  "version": "1.0",
  "iteration": 3,
  "config": {
    "command": "claude",
    "args": ["-p"],
    "prompt": "Implement authentication...",
    "completion_promise": "COMPLETE",
    "max_iterations": 20,
    "include_iteration_context": true,
    "backend": {
      "backend_type": "claude",
      "prompt_mode": "arg",
      "output_format": "stream-json"
    },
    "git": {
      "auto_commit": true,
      "commit_template": "loop: iteration {iteration}"
    }
  },
  "started_at": "2025-01-15T10:30:00Z",
  "last_iteration_at": "2025-01-15T10:35:00Z",
  "completed": false,
  "exit_reason": {"type": "running"},
  "iteration_summaries": [
    {
      "iteration": 0,
      "started_at": "2025-01-15T10:30:00Z",
      "completed_at": "2025-01-15T10:32:00Z",
      "exit_code": 0,
      "output_preview": "I'll start by examining the codebase...",
      "promise_checked": true
    }
  ]
}
```

---

## Backend Configuration

### Claude Backend

Optimized for Claude Code CLI:

```bash
descartes loop start \
  --command "claude" \
  --backend claude \
  --prompt "Your task"
```

The Claude backend:
- Uses `-p` flag for prompt mode
- Sets `--output-format stream-json`
- Formats prompts with iteration context
- Checks for both `<promise>X</promise>` and `<promise>X</promise>` patterns

### OpenCode Backend

Configured for OpenCode CLI:

```bash
descartes loop start \
  --command "opencode" \
  --backend opencode \
  --prompt "Your task"
```

The OpenCode backend:
- Uses `run --format json` arguments
- Supports model selection
- Parses JSON output format

### Generic Backend

For any command-line tool:

```bash
descartes loop start \
  --command "python agent.py" \
  --backend generic \
  --prompt "Your task"
```

The generic backend:
- Passes prompt as command-line argument
- Simple string matching for completion
- No prompt formatting

---

## Git Integration

### Auto-Commit

Enable automatic commits after each iteration:

```bash
descartes loop start \
  --command "claude -p" \
  --prompt "Implement feature X" \
  --auto-commit
```

Commits use the template: `loop: iteration {iteration}`

### Custom Commit Templates

In the state file configuration:

```json
{
  "git": {
    "auto_commit": true,
    "commit_template": "feat: iteration {iteration} progress",
    "create_branch": true,
    "branch_template": "loop/{timestamp}"
  }
}
```

| Option | Default | Description |
|--------|---------|-------------|
| `auto_commit` | `false` | Create commit after each iteration |
| `commit_template` | `loop: iteration {iteration}` | Message template |
| `create_branch` | `false` | Create a new branch for the loop |
| `branch_template` | `loop/{timestamp}` | Branch name template |

---

## SCUD Integration

For SCUD-based task tracking, Descartes provides a specialized iterative loop system that executes tasks wave-by-wave, spawning fresh sub-agents for each task with complete context from your SCUD tasks, implementation plans, and custom spec files.

### Quick Start with SCUD

```bash
descartes loop start \
    --scud-tag my-feature \
    --plan ./thoughts/shared/plans/my-feature.md \
    --spec-file ./ARCHITECTURE.md \
    --verify "cargo check && cargo test"
```

This starts a SCUD-aware loop that:
1. Loads all pending tasks from the SCUD tag
2. Builds task specifications from task details + plan sections + custom files
3. Spawns a fresh Claude sub-agent for each task
4. Runs verification after each task
5. Commits completed work wave-by-wave

### Slash Commands for Claude Code

Use these commands directly in Claude Code:

```
/ralph-wiggum:ralph-loop my-feature --plan thoughts/shared/plans/my-feature.md
/ralph-wiggum:cancel-ralph
/ralph-wiggum:help
```

### Wave-Based Execution

SCUD loops organize tasks into dependency waves:

```
Wave 1: [Task A]           # No dependencies
Wave 2: [Task B, Task C]   # Both depend on A
Wave 3: [Task D]           # Depends on B and C
```

Each wave executes sequentially, with tasks in a wave processed one at a time with fresh context per task.

### Task Status Tracking

SCUD tasks flow through states:

```
pending -> in-progress -> done
                       -> blocked (if verification fails)
```

The loop automatically updates task status in `.scud/tasks/{tag}.json`.

### Configuration

The SCUD loop system accepts CLI flags that map to configuration:

```bash
descartes loop start \
    --scud-tag feature-x \
    --plan ./thoughts/shared/plans/feature-x.md \
    --spec-file ./ARCHITECTURE.md \
    --spec-file ./docs/patterns.md \
    --max-spec-tokens 5000 \
    --verify "cargo check && cargo test"
```

| CLI Flag | Default | Description |
|----------|---------|-------------|
| `--scud-tag` | (required) | SCUD tag for task tracking |
| `--plan` | `None` | Implementation plan for context |
| `--spec-file` | `[]` | Additional spec files (repeatable) |
| `--max-spec-tokens` | `5000` | Token budget warning threshold |
| `--verify` | `cargo check && cargo test` | Run after each task |

The underlying Rust configuration structure:

```rust
ScudLoopConfig {
    tag: "feature-x",
    plan_path: Some("/path/to/plan.md"),
    max_iterations_per_task: 3,
    max_total_iterations: 100,
    use_sub_agents: true,
    verification_command: Some("cargo check && cargo test"),
    auto_commit_waves: true,
    spec: LoopSpecConfig {
        include_task: true,
        include_plan_section: true,
        additional_specs: vec![PathBuf::from("./ARCHITECTURE.md")],
        max_spec_tokens: Some(5000),
        spec_template: None,
    },
}
```

### Spec Building

For each task, the loop builds a comprehensive specification by combining:

1. **Task details from SCUD** - Title, description, test strategy, dependencies
2. **Relevant plan section** - Extracted from the implementation plan document
3. **Additional spec files** - Architecture docs, patterns, examples

The spec is passed to each sub-agent as its complete context. The agent reads code as needed but receives the task specification upfront.

### Wave Commits

After completing all tasks in a wave:

```bash
# Automatic commit message
feat(feature-x): complete wave 2
```

Wave commits are recorded with metadata:

```json
{
  "wave_commits": [
    {
      "wave": 1,
      "commit_hash": "abc123",
      "timestamp": "2025-01-15T10:45:00Z",
      "tasks_completed": [1, 2]
    },
    {
      "wave": 2,
      "commit_hash": "def456",
      "timestamp": "2025-01-15T11:00:00Z",
      "tasks_completed": [3, 4, 5]
    }
  ]
}
```

### Verification

After each task, the verification command runs:

```bash
make check test
```

If verification fails:
1. Task is marked `blocked`
2. Reason is recorded
3. Loop continues to next task

Blocked tasks are tracked:

```json
{
  "blocked_tasks": [
    {
      "task_id": 5,
      "title": "Implement auth middleware",
      "reason": "Verification failed",
      "attempts": 3,
      "blocked_at": "2025-01-15T10:50:00Z"
    }
  ]
}
```

---

## GUI Visualization

The Loop view displays real-time progress:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Iterative Loop: Feature Implementation                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Progress: ████████░░░░░░░░ 45% (9/20 iterations)                   │
│                                                                      │
│  Current Iteration: 9                                                │
│  Status: Running                                                     │
│  Started: 15 minutes ago                                             │
│                                                                      │
│  Iteration History:                                                  │
│  ┌──────┬────────────┬───────────┬─────────────────────────────────┐ │
│  │ #    │ Duration   │ Exit Code │ Preview                         │ │
│  ├──────┼────────────┼───────────┼─────────────────────────────────┤ │
│  │  8   │ 2m 15s     │ 0         │ Added authentication tests...   │ │
│  │  7   │ 1m 45s     │ 0         │ Implemented JWT middleware...   │ │
│  │  6   │ 2m 30s     │ 0         │ Created user model...           │ │
│  │  ...                                                             │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  [Pause]  [Cancel]  [View Logs]                                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### SCUD Wave View

For SCUD loops, wave progress is visualized:

```
┌─────────────────────────────────────────────────────────────────────┐
│  SCUD Loop: feature-x                                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Wave 2 of 4                                                         │
│  Tasks: 3/12 completed                                               │
│                                                                      │
│  Wave 1: ████████████████ Complete (3 tasks)                        │
│  Wave 2: ████████░░░░░░░░ In Progress (2/4 tasks)                   │
│  Wave 3: ░░░░░░░░░░░░░░░░ Pending (3 tasks)                         │
│  Wave 4: ░░░░░░░░░░░░░░░░ Pending (2 tasks)                         │
│                                                                      │
│  Current Task: Implement auth middleware                             │
│  Status: in-progress                                                 │
│  Complexity: 5                                                       │
│                                                                      │
│  Blocked Tasks: 1                                                    │
│    - Task 3: "Setup database" (Verification failed)                 │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Configuration Reference

### IterativeLoopConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | `String` | (required) | Command to execute |
| `args` | `Vec<String>` | `[]` | Additional command arguments |
| `prompt` | `String` | (required) | Task prompt |
| `completion_promise` | `Option<String>` | `None` | Completion signal text |
| `max_iterations` | `Option<u32>` | `10` | Safety iteration limit |
| `working_directory` | `Option<PathBuf>` | `cwd` | Execution directory |
| `state_file` | `Option<PathBuf>` | `.descartes/loop-state.json` | State persistence path |
| `include_iteration_context` | `bool` | `true` | Add context after first iteration |
| `iteration_timeout_secs` | `Option<u64>` | `None` | Timeout per iteration |

### LoopBackendConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend_type` | `String` | `generic` | `claude`, `opencode`, or `generic` |
| `prompt_mode` | `String` | `arg` | How to pass prompt: `arg`, `stdin`, or `env` |
| `environment` | `HashMap` | `{}` | Additional environment variables |
| `output_format` | `String` | `text` | Expected output: `stream-json`, `json`, or `text` |

### LoopGitConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_commit` | `bool` | `false` | Commit after each iteration |
| `commit_template` | `String` | `loop: iteration {iteration}` | Message template |
| `create_branch` | `bool` | `false` | Create new branch for loop |
| `branch_template` | `String` | `loop/{timestamp}` | Branch name template |

### ScudLoopConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tag` | `String` | (required) | SCUD tag for task file |
| `plan_path` | `Option<PathBuf>` | `None` | Implementation plan document |
| `handoff_path` | `Option<PathBuf>` | `None` | Handoff document for resume |
| `max_iterations_per_task` | `u32` | `3` | Retries per task |
| `max_total_iterations` | `u32` | `100` | Global iteration limit |
| `use_sub_agents` | `bool` | `true` | Spawn sub-agents for tasks |
| `verification_command` | `Option<String>` | `make check test` | Post-task verification |
| `auto_commit_waves` | `bool` | `true` | Commit after wave completion |

---

## Best Practices

### 1. Set Appropriate Limits

Always set `max_iterations` to prevent runaway loops:

```bash
descartes loop start \
  --max-iterations 20 \
  ...
```

### 2. Use Tagged Completion Promises

The tagged format reduces false positives:

```bash
--completion-promise "TASK_COMPLETE"
# Agent outputs: <promise>TASK_COMPLETE</promise>
```

### 3. Enable Auto-Commit for Long Tasks

For multi-hour tasks, auto-commit preserves progress:

```bash
--auto-commit
```

### 4. Monitor with Status Command

Check progress in another terminal:

```bash
watch -n 5 descartes loop status
```

### 5. Use Ctrl+C for Graceful Stop

The loop handles SIGINT gracefully, finishing the current iteration before stopping.

---

## Next Steps

- **[Flow Workflow](07-flow-workflow.md)** - PRD to code automation using iterative loops
- **[Sub-Agent Tracking](10-subagent-tracking.md)** - Monitor agents spawned during loops
- **[Advanced Features](11-advanced-features.md)** - Time-travel and state restoration

---

*Keep iterating until it's right.*
