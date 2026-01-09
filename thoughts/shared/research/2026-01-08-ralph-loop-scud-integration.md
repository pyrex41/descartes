---
date: 2026-01-08T12:00:00-08:00
researcher: Claude Code (Opus 4.5)
git_commit: 44705baf3d355717130c7e1122b554daf7482e25
branch: master
repository: cap
topic: "Ralph-Style Loop Integration with SCUD Flow Implementation"
tags: [research, codebase, ralph-loop, scud, flow, iterative-agents]
status: complete
last_updated: 2026-01-08
last_updated_by: Claude Code
---

# Research: Ralph-Style Loop Integration with SCUD Flow Implementation

**Date**: 2026-01-08T12:00:00-08:00
**Researcher**: Claude Code (Opus 4.5)
**Git Commit**: 44705baf3d355717130c7e1122b554daf7482e25
**Branch**: master
**Repository**: cap

## Research Question

User asked: "We have a rough Ralph Wiggum plugin adapted from clock code. Here's a conversation with Geoff (Geoffrey Huntley) who developed it initially. I'd like to do something more like this in our flow implementation that does the loop with SCUD. Research exactly how we've implemented it and help figure out what's needed."

## Summary

The codebase has **three layers** of Ralph-style loop implementation, each with increasing SCUD awareness:

1. **Generic IterativeLoop** (`iterative_loop.rs`) - Wraps any CLI command, loops until completion promise detected
2. **ScudIterativeLoop** (`scud_loop.rs`) - SCUD-aware loop with wave-based execution and task state tracking
3. **flow:implement slash command** - High-level workflow description for SCUD task execution

**Current State vs Geoff's Approach:**

| Aspect | Current Implementation | Geoff's Approach |
|--------|----------------------|-----------------|
| Completion detection | Promise tags in output | Fresh context per goal (no promise reliance) |
| Context management | Cumulative with iteration context | Fixed ~5k token spec allocation per loop |
| Loop control | Max iterations + promise | External orchestrator (shell script) |
| Compaction | Not explicitly handled | Avoided entirely ("the devil") |
| Goal scope | Can drift across iterations | One goal per fresh context window |

**Key Gap**: The current implementation follows Anthropic's plugin pattern (promise-based completion), which Geoffcriticizes as unreliable. A more "Geoff-style" approach would reset context each iteration with a persistent spec prefix.

## Detailed Findings

### 1. Current Ralph Wiggum Implementation

#### 1.1 Generic Iterative Loop (`descartes/core/src/iterative_loop.rs`)

A 1,313-line Rust implementation providing:

**Core Configuration:**
```rust
pub struct IterativeLoopConfig {
    pub command: String,              // CLI to run (claude, opencode, etc.)
    pub args: Vec<String>,            // Command arguments
    pub prompt: String,               // Task prompt
    pub completion_promise: Option<String>,  // Exit signal text
    pub max_iterations: Option<u32>,  // Safety limit (default: 10)
    pub state_file: Option<PathBuf>,  // Persistence (.descartes/loop-state.json)
    pub include_iteration_context: bool,  // Add iteration info to prompt
    pub backend: LoopBackendConfig,   // Backend-specific settings
    pub git: LoopGitConfig,           // Git auto-commit config
}
```

**Execution Flow:**
1. Load or create state from `.descartes/loop-state.json`
2. Check cancellation flag and iteration limits
3. Build prompt with optional iteration context
4. Spawn CLI process, capture output
5. Check for `<promise>TEXT</promise>` completion signal
6. If not found, increment iteration and repeat
7. Persist state after each iteration

**Backend Support:**
- `LoopClaudeBackend` - Claude Code CLI (`claude -p --output-format stream-json`)
- `LoopOpenCodeBackend` - OpenCode CLI (`opencode run --format json`)
- `LoopGenericBackend` - Any arbitrary command

**Location:** `/Users/reuben/gauntlet/cap/descartes/core/src/iterative_loop.rs`

#### 1.2 SCUD-Aware Loop (`descartes/core/src/scud_loop.rs`)

A 934-line extension that adds SCUD task tracking:

**Key Differences from Base Loop:**
- Completion detected via SCUD task states, not promise tags
- Progress tracked by wave (parallel task groups), not iteration count
- Commits after each wave, not each iteration
- Designed for sub-agent spawning per task

**Configuration:**
```rust
pub struct ScudLoopConfig {
    pub tag: String,                    // SCUD tag for task tracking
    pub plan_path: Option<PathBuf>,     // Implementation plan for context
    pub max_iterations_per_task: u32,   // Default: 3
    pub max_total_iterations: u32,      // Default: 100
    pub use_sub_agents: bool,           // Spawn per-task agents
    pub verification_command: Option<String>,  // e.g., "cargo check && cargo test"
    pub auto_commit_waves: bool,        // Git commit after wave completion
}
```

**Execution Flow:**
1. Get SCUD stats and waves for tag
2. Check if all tasks complete (via `scud stats --tag`)
3. Get next pending task from current wave
4. Mark task `in-progress`, execute, verify
5. Mark `done` or `blocked` based on verification
6. After wave complete, commit and advance to next wave
7. Continue until all done or limits reached

**Current Limitation (line 679-698):**
```rust
/// Execute a single task
/// In production, this would spawn a sub-agent
async fn execute_task(&self, task: &LoopTask) -> Result<bool> {
    // For now, this is a placeholder
    // In production, we'd:
    // 1. Read the plan context for this task
    // 2. Spawn a sub-agent with task-implementer prompt
    // 3. Monitor progress
    // 4. Return success/failure

    info!("Executing task {} (placeholder - sub-agent implementation pending)", task.id);
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(true)
}
```

**Location:** `/Users/reuben/gauntlet/cap/descartes/core/src/scud_loop.rs`

#### 1.3 flow:implement Slash Command

A markdown-based workflow description for SCUD task implementation:

**Location:** `/Users/reuben/gauntlet/cap/.claude/commands/flow/implement.md`

**Described Loop Pattern:**
```
LOOP START
│
├─► Check Completion (scud stats --tag)
│   └─► All done? → Exit with success
│
├─► Get Current Wave (scud waves, scud next)
│   └─► No tasks? → Commit wave, advance
│
├─► For Each Pending Task in Wave:
│   ├─► Claim Task (scud set-status in-progress)
│   ├─► Gather Context (read task, find plan, use pattern-finder)
│   ├─► Implement (write code, follow patterns, add tests)
│   ├─► Verify (cargo check && cargo test, up to 3 attempts)
│   └─► Complete (scud set-status done)
│
├─► Wave Complete → git commit
└─► Continue to Next Wave
```

This is a **workflow description**, not executable code. It guides Claude through the manual loop.

### 2. Ralph Wiggum Slash Commands (Status: Not Found)

The skill registry shows:
- `ralph-wiggum:help`
- `ralph-wiggum:cancel-ralph`
- `ralph-wiggum:ralph-loop`

**However, no actual command files exist at:**
- `/Users/reuben/gauntlet/cap/.claude/commands/ralph-wiggum/` (directory doesn't exist)

The skills are **registered but not implemented** as slash commands. The functionality exists in the Rust code (`iterative_loop.rs`), but there's no Claude Code slash command wrapper to invoke it.

### 3. Geoff's Key Insights from ralph_convo.md

#### 3.1 Context Engineering (Critical Difference)

Geofftreats the context window as a **fixed-size array** (like C/C++ memory):
- ~5,000 tokens dedicated to **core specs/application details** that persist across loops
- To-do list or implementation plan focusing on **one high-priority task**
- Reset goals per loop for determinism
- Leave headroom for human intervention

**Current gap:** Our implementation appends iteration context, potentially diluting focus:
```rust
// iterative_loop.rs:566-594
fn build_iteration_prompt(&self) -> String {
    let mut prompt = self.state.config.prompt.clone();
    if self.state.config.include_iteration_context && self.state.iteration > 0 {
        let iteration_info = format!(
            "\n\n---\nITERATION CONTEXT:\n- This is iteration {} of {}\n...",
            // adds context each time
        );
        prompt.push_str(&iteration_info);
    }
    prompt
}
```

#### 3.2 Completion Promise Criticism

Geoffdislikes Anthropic's promise-based completion:
> "The plugin depends on the model emitting a specific 'completion promise' in its final message to signal task done. If absent, it injects the prompt again to loop. Geoffsees this as unreliable, introducing non-determinism because the model might forget or mishandle the promise."

**Current implementation uses this exact pattern:**
```rust
// iterative_loop.rs:434-451
if let Some(ref promise) = self.state.config.completion_promise {
    if let Some(found_text) = self.check_completion_promise(&output, promise) {
        // completion detected
    }
}
```

#### 3.3 Auto-Compaction ("The Devil")

Geoffon context compaction:
> "The plugin uses automatic context compaction (summarization) when the window fills, which is lossy – it replaces full details with model-generated summaries, potentially removing critical elements like specs, tasks, or objectives."

**Geoff's alternative:** One goal per fresh context window. No compaction. Specs persist via the prompt structure, not cumulative history.

#### 3.4 External Orchestration

Geoffprefers shell scripts or external orchestrators for loop control, not model-managed looping:
> "Custom setups use external orchestrators (e.g., shell scripts) for control, pushes, resets, and evaluations."

The core Ralph pattern:
```bash
while :; do cat PROMPT.md | claude-code --continue; done
```

### 4. Gap Analysis: Current vs Jeff-Style Implementation

| Feature | Current Status | Needed for Jeff-Style |
|---------|---------------|----------------------|
| External loop control | Rust executor | Shell script or `descartes loop` CLI |
| Fresh context per iteration | Cumulative | Reset context, keep fixed spec prefix |
| Fixed spec allocation | Variable prompt | ~5k token spec block at start |
| Promise-free completion | Promise-based | Objective completion via SCUD stats |
| Wave-based execution | Implemented in scud_loop.rs | Already present |
| Sub-agent spawning | Placeholder | Needs implementation |
| CLI command | `descartes loop start/resume/status/cancel` | Already implemented |

### 5. What Already Works

1. **CLI Infrastructure** (`descartes/cli/src/commands/loop_cmd.rs`):
   - `descartes loop start --prompt "..." --command claude`
   - `descartes loop resume`
   - `descartes loop status`
   - `descartes loop cancel`

2. **SCUD Integration** (`scud_loop.rs`):
   - Wave-based task execution
   - Task state tracking via SCUD stats
   - Auto-commit after waves
   - Blocked task handling

3. **Backend Flexibility** (`iterative_loop.rs`):
   - Claude, OpenCode, or generic CLI support
   - Configurable prompt modes (arg, env, stdin)

### 6. What's Missing for Jeff-Style SCUD Loop

1. **Slash Command Wrappers**
   - No `/ralph-loop` command files exist
   - Need to create `.claude/commands/ralph-wiggum/*.md`

2. **Fresh Context Mode**
   - Add config option to NOT append iteration context
   - Instead, use fixed spec prefix + current task only

3. **SCUD-Based Completion (not promise)**
   - `scud_loop.rs` already does this
   - Just needs to be wired to CLI/slash commands

4. **Sub-Agent Task Execution**
   - Placeholder exists at `scud_loop.rs:679-698`
   - Needs actual implementation to spawn Claude per task

5. **Spec Persistence Pattern**
   - Define a "spec block" format (~5k tokens)
   - Prefix every iteration with this block
   - Clear previous work context, keep spec

## Code References

| File | Lines | Description |
|------|-------|-------------|
| `descartes/core/src/iterative_loop.rs` | 1-1313 | Generic iterative loop executor |
| `descartes/core/src/scud_loop.rs` | 1-934 | SCUD-aware loop with wave execution |
| `descartes/cli/src/commands/loop_cmd.rs` | - | CLI commands for loop control |
| `.claude/commands/flow/implement.md` | 1-306 | Workflow description for SCUD implementation |
| `descartes/core/src/lib.rs` | - | Exports loop types |

## Architecture Documentation

**Existing Patterns:**
- **Iterator Pattern**: Loop executor iterates until condition met
- **State Machine**: Task status transitions (pending → in-progress → done/blocked)
- **Factory Pattern**: Backend creation from config
- **Observer Pattern**: Cancellation via AtomicBool flag

**Integration Points:**
- SCUD CLI (`scud stats`, `scud waves`, `scud set-status`)
- Git operations (auto-commit per wave)
- Claude/OpenCode CLI invocation

## Recommendations (for implementation, not research)

If you want to implement a more Geoff-style loop:

1. **Create slash commands** in `.claude/commands/ralph-wiggum/`:
   - `ralph-loop.md` - Start loop with SCUD tag
   - `cancel-ralph.md` - Stop active loop
   - `help.md` - Explain the technique

2. **Add "fresh context" mode** to `IterativeLoopConfig`:
   ```rust
   pub fresh_context_mode: bool,  // Reset context each iteration
   pub spec_prefix: Option<String>,  // Fixed spec to prepend
   ```

3. **Wire ScudIterativeLoop to CLI**:
   - The executor exists but isn't exposed via `descartes loop`
   - Add `--scud-tag` option to use SCUD-based completion

4. **Implement sub-agent spawning** at `scud_loop.rs:679`:
   - Spawn Claude with task-specific prompt
   - Use `task-implementer` agent definition

## Related Research

- `descartes/thoughts/shared/research/2025-12-28-iterative-agent-loop-ralph-style.md` - Original implementation research
- `descartes/thoughts/shared/plans/2025-12-28-iterative-agent-loop-implementation.md` - Implementation plan
- `thoughts/shared/research/2025-12-29-loop-implementation-comparison.md` - Loop comparison analysis

## SCUD as Fixed Spec Provider

**Key insight:** SCUD tasks + planning process already provide the "fixed spec" that Geoff emphasizes. Each task contains:

| SCUD Element | Maps to Geoff's Spec Concept |
|--------------|------------------------------|
| Task title + description | Core objective |
| Implementation details from plan | Detailed spec |
| Test strategy | Success criteria |
| Dependencies | Prerequisite context |
| Wave membership | Scope boundary |

This means the ~5k token spec allocation is **already structured** via SCUD. The loop just needs to:

1. Load current task from SCUD
2. Load relevant plan section
3. Optionally load custom spec extensions
4. Execute with fresh context (no cumulative history)

### Extending Spec Flexibility

For additional control, consider a `spec_sources` config:

```rust
pub struct LoopSpecConfig {
    /// Always include: SCUD task details
    pub include_task: bool,  // default: true

    /// Include the relevant plan section
    pub include_plan_section: bool,  // default: true

    /// Additional spec files to concatenate
    pub additional_specs: Vec<PathBuf>,

    /// Max tokens for spec (truncate if exceeded)
    pub max_spec_tokens: Option<usize>,

    /// Custom spec template (use {task}, {plan}, {custom} placeholders)
    pub spec_template: Option<String>,
}
```

This would allow:
- `--spec-file ./ARCHITECTURE.md` for project-wide context
- `--spec-file ./API_CONTRACTS.md` for interface definitions
- `--spec-template "Task: {task}\n\nConstraints: {custom}\n\nPlan: {plan}"`

## Open Questions

1. **Shell vs Rust executor**: Should the "Geoff-style" loop be a simple shell script (`while :; do ... done`) or the Rust executor?

2. **Context reset granularity**: Reset per iteration, per task, or per wave?

3. **Spec extension format**: How should additional spec files be structured for optimal token efficiency?

4. **Integration path**: Add to existing `descartes loop` or create separate `descartes ralph` command?
