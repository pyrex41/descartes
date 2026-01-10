---
date: 2026-01-10T03:32:51Z
researcher: reuben
git_commit: 397605a0ff10636ac0f87f334ef3dce27a590c0a
branch: master
repository: descartes
topic: "Codebase Architecture Audit for Refactor Planning"
tags: [research, codebase, architecture, refactor, harness, gui, ralph-wiggum, agents]
status: complete
last_updated: 2026-01-09
last_updated_by: reuben
---

# Research: Codebase Architecture Audit for Refactor Planning

**Date**: 2026-01-10T03:32:51Z
**Researcher**: reuben
**Git Commit**: 397605a
**Branch**: master
**Repository**: descartes

## Research Question

Map the current codebase architecture against the core vision to identify:
1. What exists and aligns with core goals
2. What exists but represents sprawl or scope creep
3. What's partially implemented or abandoned

### Core Vision (from user)
- **Harness** supporting Claude Code and maybe OpenCode
- **Agent/sub-agent spawning** with visibility
- **Attach to running agents** capability
- **Simple, snappy GUI** for monitoring
- **Configurability** for the system
- **Real Ralph Wiggum implementation** with GUI support
- **Flexible agent topology** with user control
- **Historical and live visibility** into what's happening

---

## Summary

The Descartes codebase has evolved into a comprehensive but sprawling agent orchestration system. The core infrastructure for the vision exists, but it's surrounded by multiple alternative approaches and partially implemented features that create complexity without clear value.

### What Aligns with Core Vision

| Component | Status | Location |
|-----------|--------|----------|
| CLI Harness (spawn/attach/kill/pause/resume/logs) | Complete | `cli/src/commands/` |
| Claude Code Integration | Complete | `daemon/src/claude_code_tui.rs` |
| OpenCode Integration | Complete | `daemon/src/opencode_tui.rs` |
| Agent Runner (local processes) | Complete | `core/src/agent_runner.rs` |
| Session Management | Complete | `core/src/session_manager.rs` |
| RPC Communication (CLI ↔ Daemon) | Complete | `daemon/src/rpc_server.rs` |
| Ralph Wiggum (Iterative Loop) | Complete | `core/src/iterative_loop.rs` |
| SCUD Loop (Task-aware Ralph) | Complete | `core/src/scud_loop.rs` |
| GUI Framework (Iced) | Complete | `gui/src/main.rs` |
| Chat View | Complete | `gui/src/chat_view.rs` |
| Session Selector | Complete | `gui/src/session_selector.rs` |
| Loop View | Complete | `gui/src/loop_view.rs` |
| Swarm Monitor | Complete | `gui/src/swarm_monitor.rs` |
| Event Streaming (ZMQ pub/sub) | Complete | `daemon/src/zmq_publisher.rs` |
| Configuration System | Complete | `core/src/config.rs` |

### What Represents Sprawl

| Component | Status | Relationship to Core | Recommendation |
|-----------|--------|---------------------|----------------|
| Flow Workflow | Complete | Alternative to Ralph Wiggum (PRD→code pipeline) | Evaluate: keep or remove? |
| Swank/LISP Debugger | Complete | Specialized for Lisp development only | Evaluate: is this needed? |
| ZMQ Distributed Infrastructure | Complete (unused) | For remote agent spawning - never integrated | Evaluate: future need? |
| SCUD Task Management | Complete | External CLI dependency | Clarify relationship |
| DAG Editor | UI complete, stub backend | Visual workflow design - orphaned | Remove or complete |
| Task Board | UI complete, hidden | Kanban view - disconnected | Remove or integrate |
| Time Travel Debugger | UI complete, sample data | No real agent history connection | Remove or complete |
| Multiple Provider Backends | Complete | 6 AI providers configured | Simplify if only Claude needed |

### What's Partially Implemented

| Component | State | Missing Pieces |
|-----------|-------|----------------|
| Workflow Execute RPC | Stub | Returns placeholder, doesn't execute |
| Debugger Expression Evaluation | Stub | Returns "not implemented" error |
| Git Stash for Time Travel | Not implemented | Critical for safe restore |
| DeepSeek/Groq Streaming | Stubs | Return "unsupported" errors |
| ZMQ Server Control Commands | Stubs | Pause/Resume/IO/Signal stubbed |

---

## Detailed Findings

### 1. Harness/CLI Layer (Core - Complete)

The CLI harness is **fully implemented** and production-ready.

**Entry Point**: `cli/src/main.rs`

**Available Commands**:
- `descartes spawn` - Spawn new agents
- `descartes attach` - Attach to running agent
- `descartes ps` - List running agents
- `descartes kill` - Terminate agents
- `descartes pause` / `descartes resume` - Lifecycle control
- `descartes logs` - View agent logs
- `descartes init` - Initialize project
- `descartes doctor` - System diagnostics
- `descartes loop` - Ralph Wiggum iterative loops
- `descartes workflow` - Flow workflow execution

**AI Assistant Integration**:
- `daemon/src/claude_code_tui.rs` - Claude Code TUI handler
- `daemon/src/opencode_tui.rs` - OpenCode TUI handler
- `core/src/attach_protocol.rs` - Attach handshake protocol

**Architecture**:
```
CLI → RPC Client → Daemon RPC Server → Agent Runners → AI Backends
```

### 2. Agent Spawning System (Core - Complete)

**Local Agent Runner**: `core/src/agent_runner.rs`
- `LocalProcessRunner` - Spawns and manages local processes
- `LocalAgentHandle` - Individual agent control
- Full lifecycle: spawn, kill, pause, resume, signal

**Agent Monitoring**: `daemon/src/agent_monitor.rs`
- Real-time status tracking
- Health summaries
- Background monitoring tasks

**Session Management**: `core/src/session_manager.rs` + `session.rs`
- Session state tracking
- Transcript handling

### 3. GUI/Frontend (Mixed - Core Functional, Extras Hidden)

**Technology**: Iced 0.13 (Rust native GUI framework)

**Entry Point**: `gui/src/main.rs` - `DescartesGui` struct

**Active View Modes** (visible in navigation):
- Chat View - Agent conversation interface
- Session Selector - Pick active sessions
- Loop View - Iterative loop visualization
- Swarm Monitor - Live agent swarm status
- Debugger - Basic debugger panel

**Hidden View Modes** (removed from navigation):
- Task Board - Comment: "not connected to agent system"
- DAG Editor - Comment: "no workflow executor implemented"

**Communication**:
- `gui/src/rpc_client.rs` - HTTP RPC to daemon
- `gui/src/rpc_unix_client.rs` - Unix socket RPC
- `gui/src/zmq_subscriber.rs` - ZMQ for streaming logs

### 4. Ralph Wiggum Implementation (Core - Complete)

**Two Implementations Exist**:

1. **Generic Iterative Loop** (`core/src/iterative_loop.rs`):
   - Backend-agnostic (Claude, OpenCode, arbitrary CLI)
   - Configurable completion detection via `<promise>` tags
   - State persistence to `.descartes/loop-state.json`
   - Git integration (auto-commit on completion)
   - Maximum iteration safety limits

2. **SCUD Loop** (`core/src/scud_loop.rs`):
   - Wave-based task execution
   - "Tune the Guitar" automatic prompt refinement
   - Spec building from task + plan + custom files
   - Verification commands after each task
   - Human checkpoint after max auto-tune attempts
   - Integrates with external SCUD CLI

**CLI Commands**: `cli/src/commands/loop_cmd.rs`
- `descartes loop start` - Start iterative loop
- `descartes loop resume` - Resume paused loop
- `descartes loop status` - Check loop status
- `descartes loop cancel` - Cancel active loop
- `descartes loop tune` - Human intervention for prompt tuning

**GUI Support**: `gui/src/loop_view.rs` + `loop_state.rs`
- Wave progress visualization
- Current task display
- Blocked task list
- Completion status

### 5. Visibility/Monitoring (Core - Complete)

**Event Streaming**:
- `daemon/src/events.rs` - Event bus system
- `daemon/src/zmq_publisher.rs` - ZMQ PUB for broadcast
- `gui/src/zmq_subscriber.rs` - ZMQ SUB for receiving

**Agent Monitoring**:
- `daemon/src/agent_monitor.rs` - Status tracking
- `gui/src/swarm_monitor.rs` - Live swarm visualization

**Historical Data**:
- `core/src/agent_history.rs` - Event sourcing for agent history
- `core/src/state_store.rs` - SQLite-backed persistence
- `core/migrations/` - Database schema with history tables

**GUI Visualization**:
- Chat Graph (`gui/src/chat_graph_view.rs`) - Parent-child relationships
- Swarm Monitor - Agent cards with status
- Loop View - Wave progress

### 6. Configuration System (Core - Complete)

**Main Config**: `core/src/config.rs`
- `DescaratesConfig` - Root configuration
- Provider configs (Anthropic, OpenAI, Ollama, etc.)
- Agent behavior, storage, security, features, logging

**Loading**: `core/src/config_loader.rs`
**Watching**: `core/src/config_watcher.rs` (hot reload)
**Migration**: `core/src/config_migration.rs`

**File Locations**:
- `.descartes/config.toml` - Project-local config
- `~/.descartes/config.toml` - User config

---

## Sprawl Analysis

### A. Flow Workflow (Alternative Orchestration)

**Location**: `core/src/flow_executor.rs`, `agents/flow-*.md`

**What It Is**: A six-phase PRD-to-code pipeline:
1. Ingest - Parse PRD into tasks
2. Review Graph - Optimize dependencies
3. Plan Tasks - Generate implementation plans
4. Implement - Execute with concurrent QA
5. QA - Final quality analysis
6. Summarize - Documentation

**Relationship to Ralph Wiggum**:
- Both achieve iterative agent execution
- Flow uses long-lived sessions with persistent context
- Ralph uses fresh sub-agents per task with rebuilt specs
- Flow has orchestrator-guided error recovery
- Both documented in `docs/blog/14-choosing-your-workflow.md`

**Assessment**: This is a **parallel system**, not an extension. Having both creates cognitive overhead and maintenance burden.

### B. Swank/LISP Debugger

**Location**: `core/src/swank/`, `gui/src/lisp_debugger.rs`

**What It Is**: Complete integration with SBCL Common Lisp's Swank protocol for live Lisp development and debugging.

**Features**:
- Eval, compile, inspect Lisp code
- Interactive debugger with restarts
- GUI panel for debugger events
- Agent definition: `agents/lisp-developer.md`

**Assessment**: This is a **specialized feature** for a specific use case. The integration is complete and functional, but unclear if it's part of the core vision.

### C. ZMQ Distributed Infrastructure

**Location**: `core/src/zmq_*.rs` (4 files, ~4000 lines)

**What It Is**: Complete distributed agent orchestration over ZeroMQ:
- Message schemas with MessagePack serialization
- Server for spawning agents on remote machines
- Client for controlling remote agents
- PUB/SUB for real-time log streaming
- Reconnection, batching, health checks

**Current State**: **Complete but never integrated**
- No CLI commands use it
- No GUI connects to it
- Examples exist (`zmq_deployment_poc.rs`)
- Tests pass

**Assessment**: This is **infrastructure for a future need** that may never materialize. ~4000 lines of working code with no users.

### D. SCUD Task Management

**Location**: `core/src/scud_plugin.rs`, `core/src/scg_task_storage.rs`

**What It Is**: Integration with external SCUD CLI for task management:
- SCG file format for tasks
- Wave-based organization
- Dual workspace support (Descartes + SCUD)

**Relationship**:
- SCUD Loop depends on SCUD CLI being installed
- Creates `.scud/` directory parallel to `.descartes/`
- Task definitions in SCG format

**Assessment**: This creates an **external dependency** on another tool. Need to clarify if SCUD CLI is part of the system or a separate project.

### E. DAG Editor (Hidden, Incomplete)

**Location**: `gui/src/dag_editor.rs` (~1400 lines), `gui/src/dag_canvas_interactions.rs`

**What Exists**:
- Complete canvas-based graph editor
- Pan/zoom, node/edge creation
- Undo/redo history
- Extensive documentation (546 lines)

**What's Missing**:
- Daemon RPC `workflow.execute` returns stub response
- No actual workflow execution from DAG

**Assessment**: **Remove or complete**. The UI is polished but useless without execution.

### F. Task Board (Hidden, Disconnected)

**Location**: `gui/src/task_board.rs` (~150 lines visible)

**What Exists**:
- Complete Kanban board UI
- Task filtering and sorting
- Real-time update infrastructure

**What's Missing**:
- Hidden from navigation with comment "not connected to agent system"
- Backend exists (`core/src/task_queries.rs`) but not wired up

**Assessment**: **Remove or integrate**. The pieces exist but aren't connected.

### G. Time Travel Debugger (Sample Data Only)

**Location**: `gui/src/time_travel.rs`, `core/src/debugger.rs`

**What Exists**:
- Timeline slider, playback controls
- Event visualization with markers
- Git commit integration points

**What's Missing**:
- Only loads sample data
- No connection to real agent execution history
- Expression evaluation returns "not implemented"

**Assessment**: **Remove or complete**. Currently provides no value.

---

## Architecture Visualization

```
                    ┌─────────────────────────────────────────────────┐
                    │                 Descartes CLI                   │
                    │  spawn | attach | kill | pause | resume | logs  │
                    │                 loop | workflow                 │
                    └──────────────────────┬──────────────────────────┘
                                           │ RPC (Unix Socket)
                    ┌──────────────────────▼──────────────────────────┐
                    │                  Daemon                          │
                    │  ┌───────────┐  ┌───────────┐  ┌─────────────┐  │
                    │  │RPC Server │  │Agent      │  │Event Stream │  │
                    │  │           │  │Monitor    │  │(ZMQ PUB)    │  │
                    │  └───────────┘  └───────────┘  └─────────────┘  │
                    │  ┌───────────────────────────────────────────┐  │
                    │  │          TUI Handlers                      │  │
                    │  │  Claude Code TUI  |  OpenCode TUI          │  │
                    │  └───────────────────────────────────────────┘  │
                    └──────────────────────┬──────────────────────────┘
                                           │
                    ┌──────────────────────▼──────────────────────────┐
                    │                   Core                           │
                    │  ┌─────────────┐ ┌─────────────┐ ┌───────────┐  │
                    │  │Agent Runner │ │Iterative   │ │SCUD Loop  │  │
                    │  │(Local)      │ │Loop (RW)   │ │           │  │
                    │  └─────────────┘ └─────────────┘ └───────────┘  │
                    │  ┌─────────────┐ ┌─────────────┐ ┌───────────┐  │
                    │  │Session      │ │State Store │ │Config     │  │
                    │  │Manager      │ │(SQLite)    │ │           │  │
                    │  └─────────────┘ └─────────────┘ └───────────┘  │
                    │  ╔═════════════════════════════════════════════╗ │
                    │  ║ Sprawl / Potentially Remove:                ║ │
                    │  ║  Flow Executor | ZMQ Distributed | Swank    ║ │
                    │  ╚═════════════════════════════════════════════╝ │
                    └─────────────────────────────────────────────────┘

                    ┌─────────────────────────────────────────────────┐
                    │                   GUI (Iced)                     │
                    │  ┌───────────┐ ┌───────────┐ ┌───────────────┐  │
                    │  │Chat View  │ │Session    │ │Swarm Monitor  │  │
                    │  │           │ │Selector   │ │               │  │
                    │  └───────────┘ └───────────┘ └───────────────┘  │
                    │  ┌───────────────────────────────────────────┐  │
                    │  │ Loop View (Ralph Wiggum visualization)    │  │
                    │  └───────────────────────────────────────────┘  │
                    │  ╔═════════════════════════════════════════════╗ │
                    │  ║ Hidden / Incomplete:                        ║ │
                    │  ║  DAG Editor | Task Board | Time Travel      ║ │
                    │  ╚═════════════════════════════════════════════╝ │
                    └─────────────────────────────────────────────────┘
```

---

## Code Statistics

| Crate | Est. Lines | Core Vision | Sprawl | Incomplete |
|-------|------------|-------------|--------|------------|
| cli | ~3,000 | 90% | 0% | 10% |
| daemon | ~5,000 | 70% | 20% | 10% |
| core | ~25,000 | 40% | 40% | 20% |
| gui | ~8,000 | 50% | 30% | 20% |

**Sprawl Estimate**: ~12,000 lines of code (30% of codebase)

---

## Open Questions for Refactor Planning

1. **Flow vs Ralph Wiggum**: Keep both, merge concepts, or remove Flow?
2. **SCUD Integration**: Is SCUD CLI a dependency or should tasks be native?
3. **Swank/LISP**: Is Lisp development a target use case?
4. **ZMQ Distributed**: Is remote agent spawning a future requirement?
5. **Task Board/DAG Editor**: Complete integration or remove entirely?
6. **Time Travel**: Valuable feature or unnecessary complexity?
7. **Provider Count**: Simplify to Anthropic-only or keep multi-provider?

---

## Related Research

- `thoughts/shared/research/2025-12-12-half-baked-features-analysis.md` - Previous feature cleanup
- `thoughts/shared/research/2025-12-28-iterative-agent-loop-ralph-style.md` - Ralph Wiggum design
- `thoughts/shared/research/2025-12-26-flow-orchestration-architecture.md` - Flow design
- `docs/blog/14-choosing-your-workflow.md` - Flow vs RW comparison
