---
date: 2025-12-29T00:00:00-05:00
researcher: Claude
git_commit: 48b9705
branch: claude/review-plugin-patterns-O6wMc
repository: descartes
topic: "Comparison: local-master-backup Loop Implementation vs. New Proposal"
tags: [research, comparison, iterative-loop, ralph-wiggum, scud, flow]
status: complete
last_updated: 2025-12-29
last_updated_by: Claude
---

# Loop Implementation Comparison

## Summary

The `local-master-backup` branch contains a **substantial partial implementation** of Ralph Wiggum-style loops in Rust. This analysis compares what exists vs. what the new integrated proposal adds. **Key finding**: The implementations are **complementary, not competing** — the existing code provides low-level loop machinery, while the proposal adds high-level workflow integration.

---

## What Exists in `local-master-backup`

### 1. Core Rust Library (`descartes/core/src/iterative_loop.rs` - 1312 lines)

**Data Structures**:
```rust
IterativeLoopConfig {
    command: String,              // Any CLI command
    args: Vec<String>,
    prompt: String,
    completion_promise: Option<String>,  // <promise>TEXT</promise>
    max_iterations: Option<u32>,
    working_directory: Option<PathBuf>,
    state_file: Option<PathBuf>,
    include_iteration_context: bool,
    iteration_timeout_secs: Option<u64>,
    backend: LoopBackendConfig,
    git: LoopGitConfig,
}

IterativeLoopState {
    iteration: u32,
    config: IterativeLoopConfig,
    started_at: DateTime<Utc>,
    completed: bool,
    exit_reason: Option<IterativeExitReason>,
    iteration_summaries: Vec<IterationSummary>,
    // ...
}

IterativeExitReason::CompletionPromiseDetected
                   | MaxIterationsReached
                   | UserCancelled
                   | Error { message }
```

**Features**:
| Feature | Status | Notes |
|---------|--------|-------|
| Generic CLI support | ✅ Complete | Any command via `command` + `args` |
| Promise detection | ✅ Complete | `<promise>TEXT</promise>` pattern |
| State persistence | ✅ Complete | JSON state file for resume |
| Max iterations | ✅ Complete | Safety limit |
| Iteration timeout | ✅ Complete | Per-iteration timeout |
| Git auto-commit | ✅ Complete | Commit after each iteration |
| Backend trait | ✅ Complete | Claude, OpenCode, Generic backends |
| Cancellation | ✅ Complete | Ctrl+C handler with graceful shutdown |
| Resume capability | ✅ Complete | `IterativeLoop::resume(state_file)` |

**Backend System**:
```rust
trait LoopBackend {
    fn command(&self) -> &str;
    fn base_args(&self) -> Vec<String>;
    fn format_prompt(&self, prompt: &str, iteration: u32, max: Option<u32>) -> String;
    fn check_completion(&self, output: &str, promise: &str) -> Option<String>;
    fn output_format(&self) -> &str;
}

// Implementations:
- LoopClaudeBackend      // claude -p --output-format stream-json
- LoopOpenCodeBackend    // opencode run --format json
- LoopGenericBackend     // any arbitrary command
```

### 2. CLI Command (`descartes/cli/src/commands/loop_cmd.rs` - 255 lines)

```bash
descartes loop start \
    --command "claude" \
    --prompt "Build a REST API" \
    --completion-promise "COMPLETE" \
    --max-iterations 20 \
    --backend claude \
    --auto-commit

descartes loop resume --state-file .descartes/loop-state.json
descartes loop status --state-file .descartes/loop-state.json
descartes loop cancel --state-file .descartes/loop-state.json
```

### 3. GUI Components

**`loop_state.rs` (103 lines)**:
```rust
pub struct LoopViewState {
    pub active: bool,
    pub current_iteration: u32,
    pub max_iterations: Option<u32>,
    pub progress: f32,              // 0.0 to 1.0
    pub phase: String,              // "executing", "checking", etc.
    pub command: String,
    pub prompt_preview: String,
    pub output_lines: Vec<String>,
    pub exit_reason: Option<IterativeExitReason>,
    pub error: Option<String>,
}

pub enum LoopMessage {
    StartLoop(IterativeLoopConfig),
    CancelLoop,
    RefreshState,
    IterationComplete(u32),
    LoopComplete(IterativeLoopResult),
    OutputLine(String),
    Error(String),
}
```

**`loop_view.rs` (268 lines)**:
- Progress bar with iteration counter
- Status indicator (running, completed, cancelled, error)
- Command/prompt display
- Real-time output scrollview
- Cancel button
- Refresh button

### 4. Test Coverage

```rust
// Unit tests
- test_config_serialization
- test_state_serialization
- test_state_defaults
- test_config_defaults
- test_backend_config_defaults
- test_git_config_defaults
- test_exit_reason_serialization
- test_iteration_summary
- test_completion_promise_detection

// Integration tests
- test_iterative_loop_with_echo          // Complete in 1 iteration
- test_max_iterations                     // Exit after max reached
- test_state_persistence                  // State saved to file
- test_process_success_without_promise    // Exit code 0 completes
```

---

## What the New Proposal Adds

### 1. SCUD Task Integration

**Current**: Promise-based completion (`<promise>DONE</promise>`)
**Proposed**: SCUD task states as completion detection

```
Loop complete when:
  scud stats --tag <tag> shows 0 pending
  AND make test passes
  AND no blocked tasks
```

**Benefits**:
- Objective, measurable completion (not self-assessed)
- Natural loop boundaries (waves)
- Progress tracking built-in
- Handles partial completion gracefully

### 2. Handoff Documents

**Current**: None — single session until complete
**Proposed**: Structured handoffs between phases

```markdown
## Research Handoff
- Key findings with file:line references
- Context for next phase
- Recommended approach

## Planning Handoff
- Plan path
- SCUD tag and task count
- Wave overview
- Critical context

## Implementation Handoff
- Completion summary
- Learnings captured
- Follow-up items
```

**Benefits**:
- Clean context breaks (prevents overflow)
- Session continuity across breaks
- Audit trail of decisions
- Enables multi-session workflows

### 3. Phase-Based Workflow

**Current**: Single loop with one prompt
**Proposed**: Research → Plan → Implement pipeline

```
/flow:research [question]
      ↓ (handoff)
/flow:plan [research-path]
      ↓ (handoff + SCUD tasks)
/flow:implement [scud-tag]
      ↓ (loop with wave-based completion)
/flow:resume [handoff-path]
```

**Benefits**:
- Specialized agents per phase
- Clean separation of concerns
- Natural checkpoints
- Can resume from any phase

### 4. Wave-Based Implementation Loop

**Current**: Simple iteration counter
**Proposed**: SCUD wave progression

```
Wave 1: Foundation tasks (parallel)
    ↓ commit
Wave 2: Dependent tasks (parallel within wave)
    ↓ commit
Wave N: Final tasks
    ↓ loop complete
```

**Benefits**:
- Logical grouping of related work
- Dependency-aware execution
- Commit per wave (rollback possible)
- Progress visibility

### 5. Sub-Agent Delegation

**Current**: Single agent per iteration
**Proposed**: Spawn sub-agents per task

```
Loop iteration:
  → Get next task from SCUD
  → Spawn implementation sub-agent with:
      - Task description
      - Plan context
      - Patterns from codebase-pattern-finder
  → Verify with tests
  → Mark done or blocked
  → Continue
```

**Benefits**:
- Keeps main loop context lean
- Isolates task failures
- Enables parallelism within waves
- Better error recovery

---

## Architecture Comparison

| Aspect | `local-master-backup` | New Proposal |
|--------|----------------------|--------------|
| **Completion Detection** | `<promise>` tags | SCUD task states |
| **Loop Boundary** | Iteration count | Wave count |
| **State Persistence** | JSON file | JSON + Handoff docs |
| **Context Management** | Single session | Clean context breaks |
| **Task Tracking** | None | SCUD integration |
| **Phase Separation** | None | Research/Plan/Implement |
| **Sub-agents** | None | Per-task spawning |
| **Commit Strategy** | Per-iteration | Per-wave |
| **Resume Capability** | Same session | Cross-session via handoffs |

---

## Integration Strategy

### These Are Complementary, Not Competing

The existing `iterative_loop.rs` provides **low-level loop machinery**:
- Process spawning
- Output capture
- Promise detection
- State persistence
- Backend abstraction

The new proposal adds **high-level workflow integration**:
- SCUD task management
- Phase-based workflow
- Handoff documents
- Sub-agent delegation

### Recommended Approach

**Layer 1: Keep existing `IterativeLoop`** (from `local-master-backup`)
- This is solid, well-tested infrastructure
- Supports any CLI command
- Has GUI integration

**Layer 2: Add `ScudIterativeLoop` wrapper**
```rust
pub struct ScudIterativeLoop {
    /// Underlying loop executor
    inner: IterativeLoop,

    /// SCUD tag for task tracking
    tag: String,

    /// Current wave being processed
    current_wave: u32,

    /// Sub-agent spawner
    sub_agent_spawner: Box<dyn SubAgentSpawner>,
}

impl ScudIterativeLoop {
    /// Override completion check to use SCUD stats
    fn check_completion(&self) -> Result<bool> {
        let stats = run_scud_stats(&self.tag)?;
        Ok(stats.pending == 0 && stats.blocked == 0)
    }

    /// Execute with wave-based progression
    pub async fn execute(&mut self) -> Result<IterativeLoopResult> {
        loop {
            // Check SCUD completion
            if self.check_completion()? {
                return Ok(IterativeLoopResult {
                    exit_reason: IterativeExitReason::CompletionPromiseDetected,
                    ..
                });
            }

            // Get current wave
            let wave = self.get_current_wave()?;

            // Execute all tasks in wave (via sub-agents)
            for task in wave.tasks {
                self.execute_task_with_subagent(&task).await?;
            }

            // Commit wave
            self.commit_wave(wave.number)?;

            // Update state
            self.current_wave += 1;
            self.save_state()?;
        }
    }
}
```

**Layer 3: Add `/flow:*` commands**
- `/flow:research` → Enhanced research with handoff
- `/flow:plan` → Planning with SCUD task generation
- `/flow:implement` → Uses `ScudIterativeLoop`
- `/flow:resume` → Reads handoff, bootstraps appropriate phase

**Layer 4: Handoff infrastructure**
- Handoff document generator
- Handoff parser for resume
- Storage in `thoughts/shared/handoffs/`

---

## Files to Merge from `local-master-backup`

The following files should be cherry-picked or merged:

### Core (keep as-is)
```
descartes/core/src/iterative_loop.rs          # 1312 lines - complete
descartes/core/src/lib.rs                     # Add module export
```

### CLI (keep as-is)
```
descartes/cli/src/commands/loop_cmd.rs        # 255 lines - complete
descartes/cli/src/commands/mod.rs             # Add module
descartes/cli/src/main.rs                     # Add command
```

### GUI (keep as-is)
```
descartes/gui/src/loop_state.rs               # 103 lines - complete
descartes/gui/src/loop_view.rs                # 268 lines - complete
descartes/gui/src/lib.rs                      # Add exports
```

### Documentation (keep research, discard conflicting plans)
```
descartes/thoughts/shared/research/2025-12-28-iterative-agent-loop-ralph-style.md
```

### Files NOT to merge (superseded by new blog docs)
```
# These were deleted - blog docs are more comprehensive
descartes/docs/blog/*                         # Keep current branch version
```

---

## Implementation Phases

### Phase 1: Merge Existing Implementation
1. Cherry-pick `be26678` (the loop commit) to current branch
2. Resolve conflicts (mainly keeping blog docs)
3. Verify tests pass

### Phase 2: Add SCUD Integration
1. Create `ScudIterativeLoop` wrapper
2. Override completion detection to use `scud stats`
3. Add wave-based execution
4. Update GUI to show wave progress

### Phase 3: Add Handoff System
1. Create handoff document format/generator
2. Create `/flow:resume` command
3. Update `/flow:research`, `/flow:plan` to generate handoffs

### Phase 4: Create `/flow:implement`
1. Integrate `ScudIterativeLoop` with handoff context
2. Add sub-agent spawning per task
3. Connect to existing pattern-finder agents

---

## Conclusion

The `local-master-backup` implementation is **production-quality infrastructure** that should be preserved. The new proposal **extends it** with SCUD integration and workflow management. The recommended path is:

1. **Merge** the existing loop implementation
2. **Layer** SCUD integration on top
3. **Add** the `/flow:*` command system
4. **Connect** to existing sub-agent infrastructure

This gives you the best of both worlds:
- Generic loop capability for any CLI
- SCUD-aware loops for structured development
- Clean phase-based workflow with handoffs
