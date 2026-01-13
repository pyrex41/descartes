---
date: 2026-01-10T03:25:30Z
researcher: reuben
git_commit: 397605a0ff10636ac0f87f334ef3dce27a590c0a
branch: master
repository: descartes
topic: "GUI-to-Daemon Integration Audit: What's Connected vs. Disconnected"
tags: [research, codebase, gui, daemon, integration, cleanup]
status: complete
last_updated: 2026-01-10
last_updated_by: reuben
---

# Research: GUI-to-Daemon Integration Audit

**Date**: 2026-01-10T03:25:30Z
**Researcher**: reuben
**Git Commit**: 397605a0ff10636ac0f87f334ef3dce27a590c0a
**Branch**: master
**Repository**: descartes

## Research Question

The GUI shows many features that don't appear to be connected to the daemon. We need to document:
1. What navigation items exist in the GUI vs. what's actually supported
2. Why the daemon connection is failing
3. What features are orphaned (UI exists but no backend)
4. What "raw flutes" are and how to manage them

## Summary

The Descartes GUI has significant drift between what's shown in the UI and what's actually functional. Key findings:

1. **Port Mismatch Likely**: The GUI expects daemon at `http://127.0.0.1:19280` but daemon default config uses port `7331`. This explains the connection failure.

2. **Navigation Discrepancy**: The screenshot shows 8 nav items (Sessions, Dashboard, Chat, Tasks, Agents, Debugger, Workflows, Files, Graph) but the current code only exposes 5 (Sessions, Dashboard, Chat, Agents, Debugger).

3. **Orphaned Features**: TaskBoard and DagEditor code still exists but is hidden from navigation with comments "not connected to agent system" and "no workflow executor implemented".

4. **"FLUTE" Does Not Exist**: The term "FLUTE" is not present anywhere in the codebase. This may refer to something else (thoughts? sessions? transcripts?).

5. **What IS Connected**: Only Chat (via RPC + ZMQ streaming), Session management, and basic agent control (pause/resume/attach for paused agents).

## Detailed Findings

### 1. GUI Navigation - Current Code vs. Screenshot

#### Screenshot Shows (8 items):
- Sessions
- Dashboard
- Chat
- Tasks
- Agents
- Debugger
- Workflows
- Files
- Graph

#### Current Code Shows (5 items):
**Location**: `gui/src/main.rs:1491-1499`

```rust
let nav_items = vec![
    (ViewMode::Sessions, "\u{25C6}", "Sessions"),    // Active
    (ViewMode::Dashboard, "\u{2302}", "Dashboard"),  // Active
    (ViewMode::Chat, "\u{2709}", "Chat"),            // Active
    // TaskBoard removed - not connected to agent system
    (ViewMode::SwarmMonitor, "\u{25CE}", "Agents"),  // Active
    (ViewMode::Debugger, "\u{23F1}", "Debugger"),    // Active
    // DagEditor removed - no workflow executor implemented
];
```

#### Analysis:
The screenshot appears to be from an older version or a different build. The current codebase has deliberately hidden:
- **TaskBoard** (was "Tasks") - still in ViewMode enum but commented out
- **DagEditor** (was "Workflows") - still in ViewMode enum but commented out
- **Files** - completely removed from codebase (December 2025 cleanup)
- **Graph** - never existed in current code; may be from mockups

### 2. Port Configuration Mismatch

#### GUI Expects:
**Location**: `core/src/daemon_launcher.rs:11-30`
```rust
const DEFAULT_HTTP_PORT: u16 = 19280;
const DEFAULT_WS_PORT: u16 = 19380;

pub fn daemon_http_endpoint() -> String {
    format!("http://127.0.0.1:{}", DEFAULT_HTTP_PORT)
}
```

#### Daemon Default Config:
**Location**: `daemon/src/config.rs:45-47` (based on agent analysis)
- HTTP: port `7331` (daemon's default)
- ZMQ PUB: port `5555` (daemon's default)

#### BUT Daemon Main Allows Override:
**Location**: `daemon/src/main.rs:71-120`
The daemon accepts `--http-port` and `--ws-port` CLI args. When the GUI starts the daemon via `ensure_daemon_running()`, it should pass `--http-port 19280`.

#### Root Cause:
If the daemon is started separately (not via GUI), it uses default port 7331. The GUI expects 19280. This mismatch causes "Connection refused" errors.

### 3. Feature Connection Status

#### FULLY CONNECTED (Works End-to-End):

**Chat Interface**
- **Location**: `gui/src/chat_view.rs`, `gui/src/chat_state.rs`
- **RPC Methods Used**:
  - `chat.create` - Creates session, returns `session_id` and `pub_endpoint`
  - `chat.prompt` - Sends user input to Claude CLI
  - `chat.upgrade_to_agent` - Upgrades to agent mode
- **Streaming**: ZMQ SUB on `tcp://{pub_endpoint}` for real-time output
- **Status**: Working when daemon is running

**Session Management**
- **Location**: `gui/src/session_selector.rs`, `gui/src/session_state.rs`
- **Functionality**: Local file-based session discovery, no RPC needed
- **Status**: Works independently of daemon

**Agent Control (Paused Agents Only)**
- **Location**: `gui/src/rpc_client.rs:62-125`
- **RPC Methods Used**:
  - `agent.pause` / `agent.resume`
  - `agent.attach.request`
  - `swank.restart` (for Lisp debugger)
- **Status**: Working when daemon is running

#### PARTIALLY CONNECTED (UI + Some Backend):

**Agents Monitor (SwarmMonitor)**
- **Location**: `gui/src/main.rs:1823-1959` (view_swarm_monitor)
- **Shows**: Active session info, daemon connection status
- **Missing**: No real-time agent list, no swarm visualization
- **RPC Methods Available but Unused**:
  - `agent.list`, `agent.spawn`, `agent.kill`, `agent.logs`
  - `list_agents`, `get_agent_status`, `get_agent_statistics`

**Debugger (Time Travel)**
- **Location**: `gui/src/time_travel.rs`, `gui/src/main.rs:1962-1994`
- **Shows**: Time travel debugger UI for stepping through history
- **Missing**: No connection to real agent history
- **Data**: Only shows sample data loaded via "Load Sample History" button

#### NOT CONNECTED (UI Only, No Backend):

**Task Board**
- **Location**: `gui/src/task_board.rs`
- **Implementation**: Full Kanban board with todo/in-progress/done/blocked columns
- **Missing**: No connection to SCUD tasks or any backend
- **Comment**: "not connected to agent system"
- **Data**: Only shows sample data via "Load Sample Tasks" button

**DAG/Workflow Editor**
- **Location**: `gui/src/dag_editor.rs`, `gui/src/dag_canvas_interactions.rs`
- **Implementation**: Full DAG editor (~1,400 lines), node creation, edge connections
- **Missing**: No workflow executor exists
- **Comment**: "no workflow executor implemented"
- **Data**: Only shows sample workflows via "Load Sample Workflow" button
- **RPC Available**: `workflow.execute` exists in daemon but returns stub response

#### COMPLETELY REMOVED:

**File Browser**
- **Status**: Deleted in December 2025 cleanup
- **Former Location**: `gui/src/file_tree_view.rs`, `gui/src/code_preview_panel.rs`
- **Reason**: "UI complete, no backend" - ~1,700 lines removed
- **Documentation**: `thoughts/shared/plans/2025-12-12-codebase-cleanup-scud-update.md`

### 4. "FLUTE" Research

**Search Results**: Zero matches for "FLUTE" or "flute" anywhere in the codebase.

**Possibly Meant**:
1. **"Thoughts"** - Global thought storage system at `core/src/thoughts.rs`
2. **"Sessions"** - Workspace management at `core/src/session.rs`
3. **"Transcripts"** - Session transcripts at `core/src/session_transcript.rs`
4. **External term** - May be terminology from another project

If "raw flutes" refers to raw session transcripts or thought logs, those are managed by:
- `core/src/thoughts.rs` - Thought storage in `~/.descartes/thoughts/`
- `core/src/session_transcript.rs` - Session transcripts

### 5. Daemon RPC Methods - Full Inventory

#### Available Methods (21 total):

**Agent Lifecycle (4)**:
- `agent.spawn` - Create and start agent
- `agent.list` - List all agents
- `agent.kill` - Terminate agent
- `agent.logs` - Get agent logs

**Agent Control (2)**:
- `agent.pause` - Pause running agent
- `agent.resume` - Resume paused agent

**Attach Sessions (3)**:
- `agent.attach.request` - Get attach credentials
- `agent.attach.validate` - Validate token
- `agent.attach.revoke` - Revoke token

**Lisp/Swank (1)**:
- `swank.restart` - Invoke debugger restart

**Chat Sessions (6)**:
- `chat.create` - Create session (no CLI start)
- `chat.start` - Create and start CLI immediately
- `chat.prompt` - Send prompt to session
- `chat.stop` - Stop session
- `chat.list` - List sessions
- `chat.upgrade_to_agent` - Upgrade to agent mode

**Workflows (1)**:
- `workflow.execute` - Execute workflow (stub)

**State Queries (1)**:
- `state.query` - Query agent state

**System Monitoring (2)**:
- `system.health` - Health check
- `system.metrics` - System metrics

**Extended Monitoring (7)**:
- `list_agents`, `get_agent_status`, `get_agent_statistics`
- `get_monitoring_health`, `get_monitor_stats`
- `push_agent_update`, `register_agent`, `remove_agent`

## Code References

### GUI Main Structure
- `gui/src/main.rs:106-115` - ViewMode enum definition
- `gui/src/main.rs:163-198` - Startup and daemon connection
- `gui/src/main.rs:1491-1499` - Navigation items (with comments showing removed items)

### GUI-Daemon Connection
- `gui/src/rpc_client.rs:18-44` - GuiRpcClient wrapper
- `core/src/daemon_launcher.rs:11-30` - Port configuration
- `daemon/src/server.rs:78-110` - HTTP server binding

### Orphaned Features
- `gui/src/task_board.rs` - TaskBoard implementation (hidden)
- `gui/src/dag_editor.rs` - DagEditor implementation (hidden)
- `gui/src/main.rs:1787-1820` - view_task_board() (functional but hidden)
- `gui/src/main.rs:1998-2026` - view_dag_editor() (functional but hidden)

### Daemon RPC
- `daemon/src/rpc.rs:69-89` - Method routing
- `daemon/src/handlers.rs` - Method implementations
- `daemon/src/rpc_server.rs` - Extended methods (pause/resume/attach)

## Architecture Documentation

### Current Working Architecture

```
GUI (Iced/Rust)
    |
    |-- Local Session Management (file-based, no RPC)
    |
    +-- HTTP POST to http://127.0.0.1:19280
            |
            v
    Daemon (JSON-RPC 2.0 Server)
            |
            +-- chat.* methods --> ChatManager --> Claude CLI
            |                           |
            |                           v
            +-- ZMQ PUB socket -----> GUI subscribes for streaming
            |
            +-- agent.* methods --> AgentRunner --> Spawned processes
            |
            +-- swank.* methods --> SwankClient --> SBCL/Lisp
```

### Disconnected Components

```
GUI Components with No Backend:
    TaskBoard --------> (nothing - data is sample only)
    DagEditor --------> (nothing - workflow.execute is stub)
    Debugger ---------> (nothing - loads sample history only)
    SwarmMonitor -----> (partial - shows session but no agent list)
```

## Open Questions

1. **Port Configuration**: Should the daemon default config be updated to use 19280, or should the GUI be more flexible about which port to connect to?

2. **Screenshot Source**: Where did the screenshot come from? Is there an older build running, or is this from design mockups?

3. **FLUTE Clarification**: What does "raw flutes" refer to? Need user clarification.

4. **Feature Roadmap**: Which hidden features should be:
   - **Deleted**: Remove code entirely (like Files was)
   - **Completed**: Connect to real backend
   - **Kept hidden**: Leave for future development

5. **Agent Monitoring**: The daemon has extensive agent monitoring RPC methods (`list_agents`, `get_agent_status`, etc.) that the GUI doesn't use. Should SwarmMonitor use these?

## Recommendations for Next Steps (To Be Planned)

The following areas need attention for the GUI re-baseline:

1. **Fix Port Configuration** - Ensure daemon and GUI agree on ports
2. **Remove Dead Navigation Items** - If Files/Graph show in any version, remove them
3. **Either Connect or Remove**:
   - TaskBoard: Connect to SCUD tasks OR delete
   - DagEditor: Implement workflow executor OR delete
   - Debugger: Connect to real agent history OR simplify
4. **Enhance SwarmMonitor** - Use existing agent RPC methods
5. **Clarify FLUTE** - Determine what user means and add management if needed
