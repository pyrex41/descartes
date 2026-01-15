---
date: 2026-01-15T19:08:16Z
researcher: reuben
git_commit: dde6b888a727bb402992c013e93dd1ae7119ab11
branch: master
repository: descartes
topic: "Comparison of ralph vs loop/run commands in Descartes CLI"
tags: [research, codebase, cli, ralph, loop, orchestration]
status: complete
last_updated: 2026-01-15
last_updated_by: reuben
---

# Research: Ralph vs Loop/Run Commands - Similarities and Differences

**Date**: 2026-01-15T19:08:16Z
**Researcher**: reuben
**Git Commit**: dde6b888a727bb402992c013e93dd1ae7119ab11
**Branch**: master
**Repository**: descartes

## Research Question

What are the similarities and especially differences between the `ralph` and `loop/run` commands in Descartes?

## Summary

The Descartes CLI provides two distinct orchestration systems that share the same conceptual foundation but differ significantly in implementation:

1. **`loop`/`run`/`plan` commands** - The original BAML-driven orchestration using `ralph_loop.rs`
2. **`ralph` command** - The newer SCUD-integrated executor using `ralph_executor.rs`

Both implement the "Ralph Wiggum pattern" (fresh context per task), but the `ralph` command is the production-ready implementation with PRD parsing, wave computation, backpressure validation, TUI visualization, and context handoff support.

## Detailed Findings

### Command Definitions (main.rs)

All commands are defined in `descartes/src/main.rs`:

| Command | Lines | Description |
|---------|-------|-------------|
| `loop` | 32-41 | Continuous iteration with optional `--plan` flag and `--max` iterations |
| `run` | 43-44 | Single build iteration (alias for `loop --max 1`) |
| `plan` | 46-47 | Single planning iteration |
| `ralph` | 117-184 | Full SCUD-integrated execution with PRD, spec, and validation options |

### Key Architectural Differences

#### 1. Implementation Module

| Aspect | loop/run | ralph |
|--------|----------|-------|
| **Source file** | `ralph_loop.rs` (801 lines) | `ralph_executor.rs` (1337 lines) |
| **Primary struct** | `LoopConfig` | `RalphExecutor` |
| **Entry point** | `ralph_loop::run(loop_config, &config)` | `executor.run(&config)` |

#### 2. SCUD Integration

**loop/run commands (`ralph_loop.rs:203-227`)**:
- Uses basic SCUD module functions: `scud::next()`, `scud::list_tasks()`, `scud::complete()`
- Gets one task at a time via `scud::next()`
- No wave computation - processes tasks sequentially
- No tag-based filtering

**ralph command (`ralph_executor.rs:177-458`)**:
- Direct SCUD storage access via `scud::Storage`
- Full wave computation using Kahn's algorithm (`compute_waves()` at line 728)
- Operates on specific SCUD tags via `--scud-tag`
- Computes parallel execution potential
- Can initialize tasks from PRD documents

#### 3. PRD Initialization

**loop/run**: No PRD support

**ralph** (`main.rs:440-510`):
```
--prd <PATH>         # Parse PRD into SCUD tasks
--num-tasks <N>      # Number of tasks to generate (default: 10)
--tag <NAME>         # Tag name for tasks
--no-expand          # Skip task expansion
--no-check-deps      # Skip dependency validation
```

Runs these SCUD commands automatically:
1. `scud parse <prd> --tag <tag> -n <num>`
2. `scud expand --all --tag <tag>` (unless `--no-expand`)
3. `scud check-deps --fix --tag <tag>` (unless `--no-check-deps`)

#### 4. Spec Configuration

**loop/run**: No spec system - prompts are built inline

**ralph** (`spec.rs:1-635`):
```
--plan <PATH>            # Implementation plan document
--spec-file <PATH>       # Additional spec files (repeatable)
--max-spec-tokens <N>    # Token budget for spec section (default: 5000)
```

Features:
- `SpecConfig` struct for managing spec sources
- `build_task_spec()` combines task + plan + custom files
- `extract_plan_section()` finds relevant plan section by task ID
- Token budget warning when spec exceeds limit
- Template system with placeholders: `{task}`, `{plan}`, `{custom}`, `{verification}`

#### 5. Backpressure Validation

**loop/run** (`ralph_loop.rs:463-470`):
- Simple validator subagent via `run_validator()`
- Runs test command and returns boolean result
- No failure handling beyond logging

**ralph** (`ralph_executor.rs:360-436`):
```
--verify <COMMAND>       # Custom verification command
--no-validate            # Skip validation between waves
```

Features:
- Loads `BackpressureConfig` from `.scud/backpressure.toml`
- Multiple validation commands with failure details
- On validation failure: marks all completed tasks in round as Failed
- Integration with `scud::backpressure::run_validation()`
- Per-wave validation with round-level granularity

#### 6. Context Handoff

**loop/run**: No context monitoring or handoff

**ralph** (`ralph_executor.rs:509-640`):
```rust
context_window: 200_000,    // Default: 200K tokens (Claude Opus 4.5)
handoff_threshold: 0.6,     // Trigger at 60% usage
enable_handoff: true,       // Can be disabled
```

Features:
- `ContextMonitor` tracks token usage during execution
- When threshold reached, generates summary via `summarize_agent_progress()`
- Creates `HandoffContext` with prior work summary
- Spawns fresh agent to continue task
- Tracks handoff count in logs

#### 7. TUI Visualization

**loop/run**: No TUI - console logging only

**ralph** (`ralph_tui.rs:1-520`):
- Real-time terminal UI via crossterm
- Wave progress visualization with progress bar
- Agent status tracking (Spawning, Running, Completed, Failed)
- Keyboard controls: `[1-9]` attach, `[v]` validate, `[q]` quit
- `AgentRegistry` for managing spawned agents

#### 8. Agent Selection Strategy

**loop/run** (`ralph_loop.rs:368-413`):
1. Parse `TaskOverrides` from task body (YAML frontmatter or `// override:` comments)
2. Use BAML `SelectSubagent` for category recommendation
3. Fall back to config heuristic (`use_fast_first`)
4. Runs parallel searchers via `run_parallel_searches_baml()`
5. Conditional review phase for fast-builder output

**ralph** (`ralph_executor.rs:480-658`):
- Single harness per execution (claude-code, opencode, codex)
- No BAML orchestration - direct execution
- Model selection from `--model` or harness-specific config
- Session created per task with standard tool set

#### 9. Harness Configuration

**loop/run**:
- Uses global harness from `Config`
- Category-specific harness overrides in `CategoryConfig`
- Supports mixed harness execution (fast tasks → opencode, smart tasks → claude-code)

**ralph**:
```
--harness <KIND>        # claude-code, opencode, codex (default: claude-code)
--model <MODEL>         # Model override
--working-dir <PATH>    # Working directory
```
- Single harness for entire execution
- Model falls back to harness-specific config defaults

#### 10. Task Blocking Protocol

**loop/run**: No explicit blocking mechanism

**ralph** (`ralph_executor.rs:660-683`, `spec.rs:330-372`):
- Agent can output `TASK_BLOCKED: <reason>` to signal inability to complete
- `parse_task_result()` scans response for pattern
- Blocked tasks marked with `TaskStatus::Blocked` in SCUD
- Instructions in prompt document valid/invalid blocking reasons

#### 11. Git Integration

**loop/run** (`ralph_loop.rs:660-764`):
- `git_commit_baml()` uses BAML `GenerateCommitMessage` for conventional commits
- Supports auto-commit and auto-push via `LoopConfig`
- Stages all changes with `git add -A`

**ralph**: No built-in git operations - leaves commit management to user

### Similarities

1. **Core Pattern**: Both implement the "Ralph Wiggum" fresh-context-per-task pattern
2. **Harness Abstraction**: Both use the `Harness` trait for LLM interaction
3. **Session Lifecycle**: Both create → send → close sessions per task
4. **Config System**: Both use the shared `Config` struct from `config.rs`
5. **SCUD Foundation**: Both rely on SCUD for task definitions and status tracking
6. **Naming**: Both reference "Ralph Wiggum" and the fresh-context philosophy

### Mode Support Comparison

| Mode | loop/run | ralph |
|------|----------|-------|
| Build | Yes (`run`, `loop`) | Yes (default) |
| Plan | Yes (`plan`, `loop --plan`) | No (PRD parsing only) |
| Continuous | Yes (`loop --max 0`) | No (processes all tasks then exits) |
| Single iteration | Yes (`run`, `plan`) | No (always processes full tag) |
| Dry run | No | Yes (`--dry-run`) |

### Configuration Structures

**LoopConfig** (`ralph_loop.rs:92-113`):
```rust
pub struct LoopConfig {
    pub mode: LoopMode,           // Plan or Build
    pub max_iterations: Option<usize>,
    pub auto_commit: bool,
    pub auto_push: bool,
}
```

**RalphExecutor** (`ralph_executor.rs:44-67`):
```rust
pub struct RalphExecutor {
    pub scud_tag: String,
    pub spec_config: SpecConfig,
    pub verify_command: Option<String>,
    pub harness_name: String,
    pub model: Option<String>,
    pub round_size: usize,
    pub validate: bool,
    pub working_dir: PathBuf,
    pub context_window: usize,
    pub handoff_threshold: f64,
    pub enable_handoff: bool,
}
```

## Code References

### loop/run Command Handler
- `descartes/src/main.rs:236-272` - Command dispatch
- `descartes/src/ralph_loop.rs:116-191` - Main `run()` function
- `descartes/src/ralph_loop.rs:295-487` - `build_iteration()`

### ralph Command Handler
- `descartes/src/main.rs:419-543` - Command dispatch with PRD handling
- `descartes/src/ralph_executor.rs:177-458` - Main `run()` method
- `descartes/src/ralph_executor.rs:460-658` - `execute_task()`

### Shared Components
- `descartes/src/config.rs:1-579` - Configuration structs
- `descartes/src/harness/mod.rs` - Harness trait and implementations
- `descartes/src/scud.rs` - SCUD integration module

## Architecture Documentation

### Execution Flow Comparison

**loop/run**:
```
1. Load config
2. Create harness
3. Initialize BAML runtime
4. Loop:
   a. Check iteration limit
   b. Create transcript
   c. Run plan_iteration() or build_iteration()
   d. Save transcript
   e. Handle result (Completed, NoTasksReady, ValidationFailed)
5. Exit when max iterations or no tasks ready
```

**ralph**:
```
1. Handle PRD initialization (if --prd)
2. Build SpecConfig from CLI options
3. Create RalphExecutor
4. If --dry-run: show execution plan and exit
5. Else run():
   a. Load backpressure config
   b. Load tasks from SCUD storage
   c. Compute waves via Kahn's algorithm
   d. Initialize TUI
   e. For each wave:
      - For each round (up to round_size tasks):
        - Execute task with fresh session
        - Monitor context for handoff
        - Update task status
        - Update TUI
      - Run backpressure validation
      - Mark failed tasks if validation fails
   f. Print summary
```

## Related Research

- Documentation: `descartes/docs/ralph-loop.md`
- SCUD CLI reference: `scud/scud-cli/README.md`

## Open Questions

1. Will the `loop`/`run` commands be deprecated in favor of `ralph`?
2. Should BAML orchestration from `ralph_loop.rs` be integrated into `ralph_executor.rs`?
3. Should `ralph` gain plan mode for task breakdown before execution?
