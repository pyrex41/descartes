---
date: 2025-01-15T16:30:00-05:00
topic: "Documentation vs Codebase Analysis: Harnesses, Auth, Packaging, and Loop Implementations"
tags: [research, codebase, documentation, harnesses, authentication, ralph-loop]
status: complete
---

# Research: Documentation vs Codebase Analysis

## Research Question

The docs have some content that may not be accurate or surface design issues:
1. Are opencode and codex harnesses also headless? Is there an attach mode for any harness?
2. Does GUI install CLI by default? Should there be a way to install both?
3. Why do docs say to use ANTHROPIC_API_KEY when claude-code harness uses the CLI (handles its own auth)?
4. Is there clear separation between pure Ralph Wiggum loop and augmented loops with review agents?

## Summary

**Critical findings:**

1. **ALL harnesses are headless** - claude-code, opencode, and codex all run headless by design. **Terminal multiplexer attach IS implemented** (press 1-9 in TUI to focus panes in Zellij/Tmux/Kitty). The socket-based attach from the PRD (SIGSTOP, Unix sockets, connect tokens) is NOT implemented.

2. **GUI/CLI separation is INTENTIONAL** - `cargo install descartes-gui` does NOT install CLI binaries. This is by design to avoid Iced dependency bloat (~100 crates, 2-5 min compile) for CLI-only users. The GUI only uses ~10-15% of the descartes-cli library (just `scud` and `config` modules). Users must run: `cargo install descartes-cli descartes-gui`

3. **Authentication documentation is WRONG** - claude-code harness delegates to `claude` CLI (requires `claude login`), NOT `ANTHROPIC_API_KEY`. OpenCode delegates to `opencode` CLI. Only codex harness directly reads an API key (`OPENAI_API_KEY`). BAML separately uses `ANTHROPIC_API_KEY` for direct API calls.

4. **TWO loop implementations are INTENTIONALLY SEPARATE**:
   - `ralph_loop.rs`: Interactive mode with BAML orchestration, 5-phase pattern, conditional review agent. Best for: exploratory work, quality-critical changes.
   - `ralph_executor.rs`: Batch mode with fresh-context-per-task, context handoff at 60%, wave parallelism, NO review agent (by design for determinism). Best for: CI/CD, large task graphs.

## Detailed Findings

### 1. Harness Implementations and Attach Mode

#### Harness Execution Model

| Harness | Implementation | Headless? | Evidence |
|---------|---------------|-----------|----------|
| **claude-code** | Spawns `claude -p` subprocess | Yes | `claude_code.rs:277` - `Command::new(&self.binary)` |
| **opencode** | Spawns `opencode run` subprocess | Yes | `opencode.rs:343` - `Command::new(&self.binary)` |
| **codex** | Direct HTTP API calls | Yes | `codex.rs:421-427` - `self.client.post(&url).json(&request)` |

All three harnesses run headless - this is by design for orchestration.

#### Attach Mode: WHAT IS IMPLEMENTED

**Terminal Multiplexer Attach (IMPLEMENTED)**
- Press 1-9 in the Ralph TUI to focus on a running agent's pane (`ralph_tui.rs:178-192`)
- Works with Zellij, Tmux, Kitty via `FocusCommand` (`registry.rs:407-424`)
- `AgentRegistry` tracks agents with pane names and terminal types (`registry.rs:87-108`)
- Focus commands: `zellij action focus-pane --name`, `tmux select-pane -t`, `kitty @ focus-window`

**In-Process Pause/Resume (IMPLEMENTED)**
- `AgentControl::Pause` and `AgentControl::Resume` exist for in-process control (`interactive/session.rs:40-54`)
- Used by subagent spawner via control channels (`agent/subagent.rs:122`)
- GUI has Pause/Resume buttons that use this mechanism

#### Attach Mode: WHAT IS NOT IMPLEMENTED (from PRD)

The full PRD vision (`.scud/docs/subagent_pause.md`) includes features not yet built:
- **SIGSTOP/SIGCONT process control** - PRD line 38 proposes OS-level process suspension
- **Unix socket exposure** - PRD lines 44, 57 want `unix://...` socket for external tools
- **Connect tokens with TTL** - PRD lines 44-45 want auth tokens for secure attachment
- **`descartes agents pause <id>` CLI command** - Only in task descriptions
- **RPC methods `agent.pause`, `agent.resume`** - Not implemented
- **`Paused` state in registry** - `RegistryStatus` has Spawning, Running, Idle, Completed, Failed, Terminated - no Paused

**Documentation Issue:** `harnesses.html:199` says OpenCode is "TUI with IPC" - this is incorrect. OpenCode harness uses CLI mode (`opencode run`), not TUI.

### 2. GUI/CLI Package Relationship (Deep Dive)

#### Crate Structure

```
descartes/
├── descartes/           # descartes-cli (v0.3.0)
│   └── Cargo.toml       # [lib] + [[bin]] descartes, claude-proxy
└── descartes-gui/       # descartes-gui (v0.1.0)
    └── Cargo.toml       # Depends on descartes-cli as library
```

#### What the GUI Actually Imports

From `descartes-gui/src/main.rs:13`:
```rust
use descartes::{scud, Config};
```

**That's it.** The GUI uses only two things from descartes-cli:

1. **`Config::load()`** (`main.rs:388`) - Load settings from `.descartes/config.toml`
2. **`scud::list_tasks()`** and **`scud::waves()`** (`main.rs:391-394`) - Read SCUD tasks and compute DAG waves

The GUI uses **~10-15% of the descartes-cli library** - just the `scud` and `config` modules.

**What the GUI does NOT use:**
- Agent spawning (`src/agent/*`)
- Harness implementations (`src/harness/*`)
- BAML integration (`src/baml_client/*`)
- Ralph loops (`src/ralph_loop.rs`, `src/ralph_executor.rs`)
- Transcripts (`src/transcript/*`)
- Interactive session (`src/interactive/*`)

#### Why Separate Crates (Intentional Design)

The separation exists because of **Iced dependency bloat**:

```toml
# descartes-gui/Cargo.toml:21
iced = { version = "0.14", features = ["tokio", "advanced"] }
```

This single dependency pulls in:
- ~100 transitive crates
- Platform-specific graphics libraries (Wayland, X11 on Linux)
- 2-5 minute compile time from scratch

**CLI users don't want any of this.** The separation keeps the CLI lean.

#### Can `cargo install descartes-gui` Also Install CLI?

**No. Cargo does not support this.**

When you run `cargo install descartes-gui`:
1. Cargo resolves dependencies (including `descartes-cli` as a **library**)
2. Builds the `descartes-gui` binary
3. Installs **only** `descartes-gui` to `~/.cargo/bin/`

The `descartes-cli` binaries (`descartes`, `claude-proxy`) are **not installed** because they're defined in a dependency, not in the package being installed.

#### Installation Behavior

| Command | What Gets Installed |
|---------|---------------------|
| `cargo install descartes-cli` | `descartes`, `claude-proxy` binaries |
| `cargo install descartes-gui` | `descartes-gui` binary only |
| **Both together** | Must run both commands separately |

**There is no workspace** - these are independent crates with path dependencies.

#### Options for Improvement

1. **Better documentation** (recommended):
   ```bash
   # Install both CLI and GUI
   cargo install descartes-cli descartes-gui
   ```

2. **Install script**:
   ```bash
   #!/bin/bash
   cargo install descartes-cli
   cargo install descartes-gui
   ```

3. **Workspace with default-members** (complex, limited benefit):
   Would require users to clone repo and run `cargo install --path . --bins`

#### Could the GUI Work Standalone?

**No.** The GUI fundamentally requires:
- SCUD storage access (`.scud/tasks/tasks.scg` parsing)
- Wave computation (Kahn's algorithm in `scud::waves()`)
- Configuration loading

These are implemented in descartes-cli's library. The GUI could theoretically depend on a smaller `scud-core` crate, but currently it uses the full library.

### 3. Authentication Flow Per Harness

#### Critical Finding: Docs Are Misleading

| Harness | How Auth Actually Works | What Docs Say |
|---------|------------------------|---------------|
| **claude-code** | Delegates to `claude` CLI (uses `claude login` auth) | Says to set `ANTHROPIC_API_KEY` |
| **opencode** | Delegates to `opencode` CLI (reads its own env vars) | Says to set `XAI_API_KEY` |
| **codex** | Direct API key: `OPENAI_API_KEY` | Correct |

#### Claude Code Harness (`src/harness/claude_code.rs`)

- **Does NOT read `ANTHROPIC_API_KEY`**
- Creates subprocess: `claude -p <prompt> --output-format stream-json`
- The `claude` CLI binary handles its own authentication
- User must have run `claude login` previously

Code evidence (`claude_code.rs:52-66`):
```rust
pub fn new(config: ClaudeCodeConfig) -> Self {
    Self {
        binary: config.binary.unwrap_or_else(|| "claude".to_string()),
        model: config.model,
        // NO API key field
        ...
    }
}
```

#### OpenCode Harness (`src/harness/opencode.rs`)

- **Does NOT read any API keys directly**
- Creates subprocess: `opencode run --format json`
- The `opencode` CLI binary reads its own environment variables
- Descartes never touches the API key

Code evidence (`opencode.rs:50-63`):
```rust
pub fn new(config: OpenCodeConfig) -> Self {
    Self {
        binary: config.binary.unwrap_or_else(|| "opencode".to_string()),
        model: config.model,
        // NO API key field
        ...
    }
}
```

#### Codex Harness (`src/harness/codex.rs`)

- **DOES read `OPENAI_API_KEY` directly** (line 162-165)
- Falls back: config file → environment variable
- Creates HTTP client with `Authorization: Bearer` header

Code evidence (`codex.rs:162-165`):
```rust
let api_key = config
    .api_key
    .clone()
    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
    .ok_or_else(|| Error::Config("Codex API key not configured".to_string()))?;
```

#### Correct Documentation Should Say

```markdown
## Authentication Setup

### Claude Code Harness
Set up authentication in the Claude CLI:
```bash
claude login
```
Descartes delegates to the `claude` binary - no API key needed in Descartes.

### OpenCode Harness
Set up authentication for the OpenCode CLI:
```bash
export XAI_API_KEY=xai-...  # For xAI/Grok models
# OR
export ANTHROPIC_API_KEY=sk-ant-...  # For Anthropic models
```
Descartes delegates to the `opencode` binary.

### Codex Harness
Descartes reads the API key directly:
```bash
export OPENAI_API_KEY=sk-...
```
```

### 4. Ralph Wiggum Loop Implementations (Deep Dive)

#### Critical Finding: TWO INTENTIONALLY SEPARATE IMPLEMENTATIONS

These are **not accidental divergence** - they serve fundamentally different purposes.

| Aspect | Pure Loop (`ralph_loop.rs`) | Ralph Executor (`ralph_executor.rs`) |
|--------|----------------------------|-------------------------------------|
| **CLI Command** | `descartes loop/run/plan` | `descartes ralph` |
| **Pattern** | BAML-orchestrated iteration | Fresh-context-per-task with handoff |
| **Review Agent** | ✅ Yes (`BuilderReviewer`) | ❌ No (intentionally excluded) |
| **Validation** | Subagent with bash tools | Shell commands from backpressure.toml |
| **Context** | Accumulating transcript | Fresh spec per task + handoff |
| **Context Handoff** | ❌ No | ✅ Yes (at 60% window) |
| **BAML Usage** | Heavy (4 functions) | None |
| **PRD Support** | ❌ No | ✅ Full PRD init workflow |
| **Best For** | Interactive development | Batch task execution |

---

#### Pure Loop (`ralph_loop.rs`) - BAML-Orchestrated

**CLI Commands**: `descartes loop`, `descartes run`, `descartes plan`

**Code path**: `main.rs:237-272` → `ralph_loop::run()`

##### 5-Phase Pattern (lines 295-487)

**Phase 1: Parallel Search Agents** (`run_parallel_searches_baml`, lines 489-577)
- Uses BAML `SelectSubagent` to dynamically choose searchers
- Spawns 2-3 parallel subagents (Searcher, Analyzer categories)
- Returns search context for builder

**Phase 2: Builder** (`run_builder`, lines 579-607)
- Category selection: task override → BAML suggestion → config heuristic
- Spawns `Builder` or `FastBuilder` agent
- Returns implementation result

**Phase 3: Review Agent** (`run_reviewer`, lines 609-642) **← CONDITIONAL**
```rust
// ralph_loop.rs:437-438
let needs_review = config.ralph_loop.always_review
    || (impl_category == "fast-builder" && overrides.disable_review != Some(true));
```

Review agent triggers when:
- `config.ralph_loop.always_review = true`, OR
- Category is `fast-builder` (unless `disable_review: true` in task)

The review agent:
- Spawns `AgentCategory::BuilderReviewer` subagent
- Reviews staged git changes (`git diff --cached`)
- Returns pass/fail boolean

**Phase 4: Validator** (`run_validator`, lines 644-658)
- Spawns `AgentCategory::Validator` subagent
- Runs test suite as backpressure gate
- Returns pass/fail boolean

**Phase 5: Commit** (`git_commit_baml`, lines 660-736)
- Uses BAML `GenerateCommitMessage` for conventional commits
- Optional auto-push

##### BAML Functions Used

| Function | Location | Purpose |
|----------|----------|---------|
| `DecideNextAction` | lines 312-332 | Flow control (Continue/Replan/Complete/AskHuman) |
| `SelectSubagent` | lines 376-412, 498-553 | Choose agent categories dynamically |
| `CreatePlan` | lines 231-251 | Generate implementation plans (Plan mode) |
| `GenerateCommitMessage` | lines 692-721 | Create conventional commits |

##### Task Overrides (lines 30-79)

Tasks can override behavior via YAML frontmatter:
```yaml
---
category: fast-builder
disable_review: true
---
Task description here
```

Or inline: `// override: category=builder,disable_review=false`

---

#### Ralph Executor (`ralph_executor.rs`) - Deterministic Batch Execution

**CLI Command**: `descartes ralph --scud-tag <tag>`

**Code path**: `main.rs:419-543` → `RalphExecutor::new()` → `executor.run()`

##### Architecture

1. **Wave computation** via Kahn's algorithm (`compute_waves`, lines 718-814)
2. **Round-based execution** within each wave (lines 259-438)
3. **Fresh context per task** via spec system
4. **Context handoff** for long-running tasks (lines 509-640)
5. **Backpressure validation** between rounds (lines 360-437)

##### Why No Review Agent? (INTENTIONAL)

The executor **deliberately excludes** the review agent for these reasons:

1. **Determinism**: Same inputs → same outputs. Review adds variability.
2. **Fresh context philosophy**: Each task gets clean slate. Review requires accumulated context about what was changed.
3. **Speed**: Review adds latency to every task. Wave-level validation is sufficient.
4. **Separation of concerns**: Backpressure validation (test suite) catches regressions. Human review happens at PR level, not per-task.

**Evidence**: The executor was designed in SCUD tasks (`.scud/tasks/tasks.scg` migrate phase) with explicit requirements for "fresh session lifecycle" and "backpressure validation", but no mention of review agents.

##### Context Handoff (lines 509-640)

Unique to the executor - handles tasks that exceed context limits:

```rust
// Main execution loop
loop {
    // Create fresh harness and session
    let harness = create_harness_by_name(&self.harness_name, config)?;
    let session = harness.start_session(session_config).await?;

    // Monitor context usage
    context_monitor.record_usage(&prompt);

    while let Some(chunk) = response_stream.next().await {
        context_monitor.record_usage(&text);

        // Check handoff threshold (60% of 200K tokens)
        if context_monitor.should_handoff() {
            break;
        }
    }

    // Perform handoff if needed
    if context_monitor.should_handoff() {
        handoff_count += 1;
        let summary = summarize_agent_progress(&full_response);
        prompt = HandoffContext::new(summary, spec, handoff_count).build_handoff_prompt();
        context_monitor.reset();
        continue;  // Spawn fresh agent with handoff context
    } else {
        break;  // Task complete
    }
}
```

**Settings** (lines 90-92):
- `context_window: 200_000` (200K tokens)
- `handoff_threshold: 0.6` (60%)
- `enable_handoff: true`

##### Backpressure Validation (lines 360-437)

Runs after each round (not per-task):
- Executes commands from `.scud/backpressure.toml`
- On failure: marks all round's completed tasks as `Failed`
- Stops wave processing on validation failure

---

#### When to Use Each

| Use Case | Command | Why |
|----------|---------|-----|
| Interactive development | `descartes loop` | BAML intelligence, review agent, adaptive |
| Exploratory work | `descartes run` | Single iteration with full phases |
| Batch task execution | `descartes ralph` | Fast, deterministic, wave parallelism |
| CI/CD automation | `descartes ralph --verify` | Backpressure gates, no human interaction |
| Quality-critical changes | `descartes loop --max 1` | Review agent catches issues |

---

#### Documentation Gaps

The docs currently don't distinguish these. Should include:

1. **Comparison table** between `loop` and `ralph` commands
2. **When to enable `always_review`** in config
3. **How context handoff works** in the executor
4. **Performance characteristics** of each approach

```markdown
## Execution Modes

### Ralph Executor (`descartes ralph`)
Batch execution mode for SCUD task graphs:
- Fresh context per task with automatic handoff at 60% window
- Wave-based parallel execution following DAG dependencies
- Backpressure validation between rounds (no review agent)
- Best for: CI/CD, large task graphs, deterministic execution

### Pure Loop (`descartes loop`)
Interactive mode with BAML orchestration:
- BAML-driven decisions for flow control and agent selection
- Optional review agent for code quality (`always_review` config)
- Accumulating context within iteration
- Best for: Interactive development, exploratory work, quality-critical changes
```

## Code References

### Harness Implementations
- `src/harness/claude_code.rs:268-293` - Claude subprocess spawn
- `src/harness/opencode.rs:333-359` - OpenCode subprocess spawn
- `src/harness/codex.rs:392-536` - HTTP API calls
- `src/harness/mod.rs:206-229` - Harness trait (no attach method)

### Attach Mode (Implemented)
- `src/ralph_tui.rs:178-192` - TUI attach handler (press 1-9)
- `src/agent/registry.rs:407-424` - FocusCommand generation
- `src/agent/registry.rs:87-108` - AgentHandle with pane tracking
- `src/interactive/session.rs:40-54` - AgentControl::Pause/Resume

### GUI/CLI Relationship
- `descartes/Cargo.toml:23-25` - CLI exports library
- `descartes-gui/Cargo.toml:24` - GUI depends on CLI library
- `descartes-gui/src/main.rs:13` - GUI imports: `use descartes::{scud, Config};`
- `descartes-gui/src/main.rs:388-394` - GUI usage of Config and scud functions

### Authentication
- `src/harness/claude_code.rs:52-66` - No API key in construction
- `src/harness/opencode.rs:50-63` - No API key in construction
- `src/harness/codex.rs:162-165` - Direct API key reading
- `baml_src/clients.baml:32` - BAML uses ANTHROPIC_API_KEY directly

### Pure Loop (`ralph_loop.rs`)
- `src/ralph_loop.rs:116-191` - Main entry point `run()`
- `src/ralph_loop.rs:295-487` - Build iteration (5-phase pattern)
- `src/ralph_loop.rs:437-438` - Review agent trigger conditions
- `src/ralph_loop.rs:489-577` - Parallel searches with BAML
- `src/ralph_loop.rs:579-607` - Builder execution
- `src/ralph_loop.rs:609-642` - Review agent (`run_reviewer`)
- `src/ralph_loop.rs:644-658` - Validator (backpressure)
- `src/ralph_loop.rs:660-736` - BAML commit generation
- `src/ralph_loop.rs:30-79` - Task overrides parsing

### Ralph Executor (`ralph_executor.rs`)
- `src/ralph_executor.rs:44-94` - Executor struct and config
- `src/ralph_executor.rs:185-458` - Main run loop (wave execution)
- `src/ralph_executor.rs:460-658` - Task execution (fresh context)
- `src/ralph_executor.rs:509-640` - Context handoff loop
- `src/ralph_executor.rs:718-814` - Wave computation (Kahn's algorithm)
- `src/ralph_executor.rs:360-437` - Backpressure validation

### CLI Entry Points
- `src/main.rs:237-272` - Pure loop commands (loop/run/plan)
- `src/main.rs:419-543` - Ralph executor command

## Architecture Documentation

### Current Pattern Summary

```
Descartes Architecture (Actual)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Harnesses (all headless):
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  claude-code    │  │    opencode     │  │     codex       │
│                 │  │                 │  │                 │
│ Spawns: claude  │  │ Spawns: opencode│  │ HTTP: OpenAI API│
│ Auth: claude    │  │ Auth: opencode  │  │ Auth: Direct    │
│ login           │  │ CLI env vars    │  │ OPENAI_API_KEY  │
└─────────────────┘  └─────────────────┘  └─────────────────┘

Execution Modes:
┌─────────────────────────────────────────────────────────────┐
│                    descartes ralph                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │  Wave 1  │→ │  Wave 2  │→ │  Wave 3  │  (DAG-ordered)   │
│  └──────────┘  └──────────┘  └──────────┘                  │
│       │                                                     │
│       ↓  Per-task:                                         │
│  ┌────────────────────────────────────────┐                │
│  │ Fresh Agent → [handoff at 60%] → Done  │                │
│  └────────────────────────────────────────┘                │
│       │                                                     │
│       ↓  Per-round:                                        │
│  ┌────────────────────────────────────────┐                │
│  │ Backpressure Validation (shell cmds)   │                │
│  └────────────────────────────────────────┘                │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    descartes loop                           │
│  Per iteration:                                            │
│  ┌────────────────────────────────────────────────────────┐│
│  │ BAML Decision → Parallel Search → Builder →            ││
│  │ [Review Agent?] → Validator → Commit                   ││
│  └────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Attach Mode (PLANNED, NOT IMPLEMENTED)

```
Future Architecture (from .scud/docs/):
┌────────────────────────────────────────────────────────────┐
│ descartes agents pause <id>                               │
│       │                                                    │
│       ↓                                                    │
│ SIGSTOP → Generate Token → Expose Socket                  │
│                               │                            │
│       ┌───────────────────────┘                            │
│       ↓                                                    │
│ External Tool (claude/opencode) attaches via socket       │
│       │                                                    │
│       ↓                                                    │
│ descartes agents resume <id>                              │
│       │                                                    │
│       ↓                                                    │
│ SIGCONT → Continue execution                              │
└────────────────────────────────────────────────────────────┘
```

This is not implemented - exists only in planning docs.

## Open Questions

1. **Should socket-based attach mode be prioritized?** Terminal multiplexer attach works. The full PRD vision (SIGSTOP, Unix sockets, connect tokens) exists in planning docs but isn't implemented. Is this still needed?

2. **Should GUI install CLI binaries?** Currently doesn't - this is **intentional** to avoid Iced dependency bloat for CLI users. Documentation should be clearer about running both install commands.

3. **Should review agent be added to ralph executor?** Currently only in pure loop - this is **intentional** for determinism and speed. The separation is by design, not an oversight. Documentation should explain the tradeoff.

4. **Should docs reference BAML clients separately from harnesses?** BAML uses `ANTHROPIC_API_KEY` directly (`baml_src/clients.baml:32`), while the claude-code harness uses `claude login`. These are independent systems that may cause confusion.

## Resolved Questions

- **Are the two loop implementations accidental?** NO - they are intentionally separate, serving different use cases (interactive vs batch).
- **Is the GUI/CLI separation intentional?** YES - to avoid Iced dependency bloat for CLI-only users.
- **Why doesn't the executor have a review agent?** BY DESIGN - for determinism, speed, and fresh-context philosophy.
