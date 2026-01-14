---
date: 2026-01-14T15:00:00-08:00
topic: "Descartes & SCUD Deep Dive: GUI/TUI Status and Grounding Issues"
tags: [research, codebase, descartes, scud, tui, gui, architecture]
status: complete
---

# Research: Descartes & SCUD Deep Dive

## Research Question

Deep dive into Descartes (and its dependency SCUD), figure out how to get the GUI up and working. The core is snappy and written in Rust, but has remnants of abandoned features and isn't grounded right.

## Summary

Descartes and SCUD have **two separate TUI implementations** that are both functional but serve different purposes:

1. **SCUD's TUI** (`scud spawn --monitor`) - A sophisticated ratatui-based three-panel monitor for parallel Claude Code agents in tmux
2. **Descartes' TUI** (`ralph_tui.rs`) - A simpler crossterm-based progress display for the Ralph executor

The "not grounded right" observation is accurate - there are several disconnected components:
- Dead workflow module (never used)
- workflow-state.json config file with no code references
- CLEANUP_PLAN.md documenting known issues
- Incomplete OpenCode harness
- Agent control messages defined but not consumed

## What "GUI" Exists

### 0. Archived Iced GUI (descartes-gui v1)

**Location:** Deleted from working tree, preserved in git at commit `fbc532f`

**Recovery Command:**
```bash
# View the GUI files
git show fbc532f:archive/descartes-v1/gui/src/main.rs

# Restore the entire GUI directory
git checkout fbc532f -- archive/descartes-v1/gui/
```

**Technology:** Iced 0.13 + ZeroMQ + descartes-daemon RPC

**Original Features:**
- **SwarmMonitor** - Live multi-agent status dashboard
- **Chat View** - Streaming chat with graph visualization
- **Time Travel Debugger** - Execution replay system
- **History Graph** - Agent conversation tree visualization
- **Lisp Debugger** - Swank protocol integration
- **Session Selector** - Workspace management

**Architecture (v1):**
```
descartes-gui ←→ descartes-daemon ←→ ZeroMQ backbone
                       ↓
                 descartes-core (agents, state)
```

**Why Archived:** v2 rewrote everything with the "Ralph Wiggum loop" pattern, removing:
- ZMQ distributed execution
- Background daemon
- gRPC API
- Complex multi-crate workspace

**Status:** The GUI was sophisticated but tightly coupled to v1's architecture (daemon, ZMQ, state store). It would need significant adaptation to work with v2's simpler design.

**Files (in git):**
| File | Purpose |
|------|---------|
| `gui/src/main.rs` | Iced app entry, ViewMode switching |
| `gui/src/swarm_monitor.rs` | Live agent dashboard |
| `gui/src/chat_view.rs` | Streaming chat interface |
| `gui/src/chat_graph_view.rs` | Conversation tree canvas |
| `gui/src/time_travel.rs` | Execution replay |
| `gui/src/history_graph_view.rs` | Agent history visualization |
| `gui/src/lisp_debugger.rs` | Swank protocol UI |
| `gui/src/rpc_client.rs` | HTTP RPC to daemon |
| `gui/src/zmq_subscriber.rs` | ZMQ SUB for streaming |

---

### 1. SCUD Spawn Monitor TUI

**Location:** `/Users/reuben/projects/harnesses/scud/scud-cli/src/commands/spawn/tui/`

**Technology:** ratatui v0.29 + crossterm v0.28

**Launch:** `scud spawn --monitor` or `scud spawn -m`

**Features:**
- Three-panel layout: Waves | Agents | Terminal Output
- Real-time agent status tracking via tmux integration
- Task selection and spawning with Space/Enter keys
- Live terminal output capture via `tmux capture-pane`
- Ralph mode for autonomous task completion
- Keyboard-driven (Tab to switch panels, j/k navigation)

**Panels:**
- **Waves Panel:** Shows task dependency waves, allows selection for spawning
- **Agents Panel:** Lists running/completed agents with status indicators
- **Output Panel:** Live scrollable terminal output from selected agent

**Status:** Fully implemented and working with tmux. Only supports tmux as the terminal backend for the TUI monitoring features.

### 2. Descartes Ralph TUI

**Location:** `/Users/reuben/projects/harnesses/descartes/descartes/src/ralph_tui.rs`

**Technology:** crossterm (no ratatui, direct terminal control)

**Launch:** Used internally by `descartes ralph --scud-tag <tag>`

**Features:**
- Wave progress bar (e.g., "Wave 2/4 | Tasks: 3/8")
- Agent status list with color-coded status
- Keyboard controls: 1-9 to attach to agent, v to validate, q to quit

**Status:** Functional but simpler than SCUD's TUI. Displays progress during Ralph executor runs.

### 3. Descartes Interactive Mode

**Location:** `/Users/reuben/projects/harnesses/descartes/descartes/src/interactive/`

**Launch:** `descartes interactive` or `descartes i`

**Features:**
- REPL-style interface with slash commands
- Skills system for prompt templates
- Signal handling (Ctrl+C to pause/cancel)
- State machine: Idle → AgentRunning → AgentPaused → AtGate

**Status:** Structurally complete but agent pause/resume/cancel not fully wired. The control channel messages are sent but `spawn_subagent()` doesn't check them.

## Abandoned/Dead Code

### 1. Workflow Module (REMOVED)

The CLEANUP_PLAN.md indicates `src/workflow/` was marked for removal. Based on my search, **this module has already been removed** - it doesn't exist in the current codebase.

### 2. workflow-state.json (Orphaned Config)

**Location:** `/Users/reuben/projects/harnesses/descartes/.scud/workflow-state.json`

**Content:** Defines workflow phases (retrospective, ideation, planning, architecture, implementation) with agent assignments.

**Status:** No code references this file. Appears to be orphaned configuration from an abandoned workflow orchestration system.

### 3. Archive Directories

**Locations:**
- `/Users/reuben/projects/harnesses/descartes/working_docs/archive/phase3/` - 42 historical reports
- `/Users/reuben/projects/harnesses/descartes/.scud/archive/` - 6 archived task files
- `/Users/reuben/projects/harnesses/descartes/working_docs/planning/legacy/` - 6 legacy PRDs

**Status:** Historical records, safe to keep or delete

### 4. claude-proxy Binary

**Location:** `/Users/reuben/projects/harnesses/descartes/descartes/src/bin/claude-proxy.rs`

**Purpose:** OpenAI-compatible HTTP proxy wrapping `claude -p`

**Status:** Builds but no documentation. Experimental/unused.

### 5. Incomplete Code

**proxy.rs:44** - `todo!("Need to clone harness or use Arc")` in `child_proxy()`
**proxy.rs:164-165** - Token metrics tracking (hardcoded to 0)
**agent/subagent.rs:160-161** - Token metrics from harness (hardcoded to 0)

## What's Working Well

### SCUD Core
- DAG-based task management with Kahn's algorithm for wave computation
- SCG format (token-efficient text format for AI context windows)
- File locking for concurrent access
- Cross-tag dependencies via namespaced IDs (`tag:id`)
- Status tracking: Pending → InProgress → Done/Blocked/Failed
- Claude Code hooks for auto-completion

### Descartes Core
- Ralph Wiggum loop pattern (fresh context per task)
- BAML integration (13 typed LLM functions)
- ClaudeCode harness (streaming JSON parsing, session management)
- Spec system (PRD + plan + custom specs)
- Wave-based execution with backpressure validation

### Integration
- Descartes uses SCUD as Rust library dependency (no IPC)
- Type-safe integration at compile time
- Storage, Phase, Task, TaskStatus all shared

## What's Not Grounded

### 1. Two Separate TUI Implementations

SCUD and Descartes each have their own TUI:
- SCUD's is more sophisticated (ratatui, three panels)
- Descartes' is simpler (crossterm only)

**Recommendation:** Consider whether Descartes should reuse SCUD's TUI infrastructure or if they serve different enough purposes to remain separate.

### 2. Agent Control Not Wired

`src/interactive/session.rs` defines `AgentControl` enum (Pause, Resume, Cancel, Interrupt) and sends messages through a channel, but `spawn_subagent()` never checks the receiver.

**Impact:** `/pause`, `/resume`, `/cancel` commands in interactive mode don't actually affect running agents.

### 3. OpenCode Harness Incomplete

`src/harness/opencode.rs` mirrors ClaudeCode structure but:
- Session ID population logic unclear
- No documentation on actual OpenCode JSON format
- May not match OpenCode's actual output format

### 4. Orphaned Configuration

`workflow-state.json` defines agent assignments that nothing reads. Either the code that reads this was deleted, or it was never implemented.

### 5. Token Metrics Always Zero

Both `proxy.rs` and `subagent.rs` have TODOs for token tracking but hardcode metrics to 0. No harness actually reports token usage.

## Getting the GUI Working

### Option A: Use SCUD's TUI

The most complete TUI is SCUD's spawn monitor:

```bash
# Initialize SCUD if needed
scud init

# Parse a PRD into tasks
scud parse ./docs/prd.md --tag my-feature -n 10

# Expand complex tasks
scud expand --all --tag my-feature

# Launch with TUI monitor
scud spawn --tag my-feature --monitor
```

**Pros:** Sophisticated three-panel UI, already working
**Cons:** Only supports tmux, separate from Descartes orchestration

### Option B: Use Descartes Ralph

Descartes' Ralph executor with TUI:

```bash
# From PRD
descartes ralph --prd ./docs/prd.md --tag my-feature

# Or existing tag
descartes ralph --scud-tag my-feature
```

**Pros:** Integrated with BAML, handles validation, wave-based execution
**Cons:** Simpler TUI, less interactive control

### Option C: Combine Both

1. Use Descartes for PRD parsing and task generation
2. Use SCUD's spawn monitor for parallel execution
3. Use Descartes' Ralph executor for validation and orchestration

### Getting Started Commands

```bash
# Build both projects
cd /Users/reuben/projects/harnesses/scud/scud-cli && cargo build --release
cd /Users/reuben/projects/harnesses/descartes/descartes && cargo build --release

# Test SCUD TUI
cd /your/project
scud init
scud parse ./prd.md --tag demo -n 5
scud spawn --tag demo --monitor  # Opens TUI

# Test Descartes
descartes ralph --prd ./prd.md --tag demo2 --dry-run  # Preview
descartes ralph --prd ./prd.md --tag demo2            # Execute
```

## Recommendations for Grounding

### Immediate Cleanup

1. **Delete orphaned workflow-state.json**
   ```bash
   rm /Users/reuben/projects/harnesses/descartes/.scud/workflow-state.json
   ```

2. **Wire agent control in interactive mode** (or remove pause/resume/cancel commands)

3. **Complete or remove OpenCode harness**

### Structural Decisions

1. **Unify TUI strategy:** Should Descartes use SCUD's TUI, or maintain separate simpler TUI?

2. **Token tracking:** Either implement properly across harnesses or remove the pretense

3. **claude-proxy binary:** Document its purpose or remove from build

### Documentation

1. Add getting-started guide for TUI usage
2. Document which terminal multiplexers are supported (tmux only for full features)
3. Clarify relationship between `descartes ralph` and `scud spawn`

## Code References

| Component | File | Lines |
|-----------|------|-------|
| SCUD TUI Main | `scud-cli/src/commands/spawn/tui/mod.rs` | 225 |
| SCUD TUI App | `scud-cli/src/commands/spawn/tui/app.rs` | 1,300 |
| SCUD TUI UI | `scud-cli/src/commands/spawn/tui/ui.rs` | 740 |
| Descartes TUI | `descartes/src/ralph_tui.rs` | 520 |
| Descartes Interactive | `descartes/src/interactive/session.rs` | 743 |
| Descartes Harnesses | `descartes/src/harness/*.rs` | ~1,500 |
| SCUD Integration | `descartes/src/scud/mod.rs` | 253 |

## Build Status

Both projects currently compile successfully:
- `scud-cli`: Clean build
- `descartes-cli`: Builds with 29 warnings (unused code, dead_code lint)

## Reviving the GUI: Options

### Option 1: Adapt v1 GUI to v2 Architecture

**Effort:** High (2-4 weeks)

The v1 GUI was built around:
1. **Daemon RPC** - HTTP/Unix socket to `descartes-daemon`
2. **ZMQ streaming** - Real-time chat output via ZeroMQ PUB/SUB
3. **State machine** - Complex state management in `descartes-core`

v2 has:
1. **No daemon** - Direct CLI execution
2. **Process spawning** - Runs `claude` CLI directly
3. **SCUD storage** - File-based task state

**Required Changes:**
- Replace RPC client with direct SCUD/harness calls
- Replace ZMQ with process stdout capture (like ralph_tui does)
- Simplify state to match Ralph executor's model
- Remove Lisp debugger (unless Swank is still wanted)

**Reusable Components:**
- Theme/styling (`theme.rs`)
- History graph visualization (adapt for Ralph loop iterations)
- Swarm monitor UI components (adapt for agent registry)

### Option 2: Build New Minimal GUI

**Effort:** Medium (1-2 weeks)

Build a new Iced or egui GUI that wraps the existing v2 components:

```
New GUI → descartes lib (RalphExecutor, Harness) → SCUD
                    ↓
            ralph_tui (reuse for progress display)
```

**Features to include:**
1. Task waves visualization (from SCUD)
2. Agent status (from AgentRegistry)
3. Live output streaming (capture harness output)
4. PRD loading and task generation

**MVP Architecture:**
```rust
// Thin GUI wrapper
struct DescartesGui {
    executor: Option<RalphExecutor>,
    waves: Vec<Vec<Task>>,
    agents: AgentRegistry,
    output_buffer: String,
}
```

### Option 3: Enhance SCUD's TUI

**Effort:** Low (1 week)

SCUD's spawn TUI (`scud spawn --monitor`) is already sophisticated. Options:
1. Add Descartes integration as new panel
2. Add BAML-driven decisions display
3. Add backpressure validation status

**Pros:** Already working, ratatui-based (consistent), tmux integration
**Cons:** Terminal-only, requires tmux

### Option 4: Web-Based Dashboard

**Effort:** Medium-High (2-3 weeks)

Build a web UI that connects to a minimal API server:

1. Add HTTP server to Descartes (hyper is already a dependency)
2. Serve static files + JSON API
3. React/Svelte frontend

**Pros:** Cross-platform, no terminal requirements
**Cons:** More moving parts, authentication considerations

## Recommended Path

**Short-term:** Use SCUD's TUI (`scud spawn --monitor`) for agent monitoring

**Medium-term:** If a desktop GUI is needed:
1. Extract reusable components from v1 GUI
2. Build minimal Iced app wrapping v2's `RalphExecutor`
3. Focus on: waves view, agent status, live output
4. Skip: Lisp debugger, time-travel, chat graph (unless specifically needed)

**Key insight:** The v1 GUI's _visual designs_ (swarm monitor, history graph) are valuable, but its _architecture_ (daemon/ZMQ) is obsolete. Extracting the UI logic and wiring it to v2's simpler model is the most efficient path.

## Open Questions

1. Should SCUD's TUI be extracted as a reusable library for Descartes?
2. Is the interactive mode's pause/resume feature still desired, or should those commands be removed?
3. What's the intended use case for claude-proxy?
4. Should token metrics tracking be properly implemented or removed?
5. **Is a desktop GUI actually needed, or is the terminal TUI sufficient?**
6. **Which v1 GUI features are must-haves for v2?**
