---
date: 2025-12-06T00:00:00-08:00
researcher: Claude
git_commit: cdf0687d77c6304e246052112698df964db06f92
branch: master
repository: cap
topic: "Comparative Analysis: Descartes Architecture vs hl_quick Simple Agent"
tags: [research, codebase, architecture, task-management, graph-view, simplification]
status: complete
last_updated: 2025-12-06
last_updated_by: Claude
---

# Research: Comparative Analysis - Descartes vs hl_quick Agent

**Date**: 2025-12-06
**Researcher**: Claude
**Git Commit**: cdf0687d77c6304e246052112698df964db06f92
**Branch**: master
**Repository**: cap

## Research Question

Conduct a deep dive into the Descartes codebase comparing it with the simpler hl_quick agent prototype. Focus on:
1. What's implemented and working vs what's not fully done
2. Where complexity can be simplified, particularly around task management hooks/handoffs
3. How the graph view from hl_quick could be brought to Descartes

## Summary

The Descartes codebase is a comprehensive Rust-based AI agent orchestration system with sophisticated architecture including process management, state persistence, DAG-based task orchestration, and a GUI. The hl_quick agent is a simpler ~20-hour prototype built with TypeScript/SolidJS that demonstrates core agent functionality with less overhead.

**Key Finding**: The user is correct that the granular task-level agent check-in/check-out system in SCUD is overengineered for current needs. A simpler approach keeping the DAG/graph structure for planning while removing hooks and handoffs at the individual task level would reduce complexity significantly.

## Detailed Findings

### 1. Descartes Architecture Overview

**Fully Implemented & Working:**

| Component | Location | Status |
|-----------|----------|--------|
| Local Process Spawning | `core/src/agent_runner.rs:41-347` | Complete |
| Pause/Resume (Cooperative + Forced) | `core/src/agent_runner.rs:522-637` | Complete |
| Session Management | `core/src/session_manager.rs:18-565` | Complete |
| SQLite State Store | `core/src/state_store.rs:14-1062` | Complete |
| State Machine Workflows | `core/src/state_machine.rs:294-542` | Complete |
| DAG Task Graph | `core/src/dag.rs:324-391` | Complete |
| Multi-Provider Support | `core/src/providers.rs:1-400` | Complete (OpenAI, Anthropic, Grok) |
| Minimal Pi-style Tools | `core/src/tools/definitions.rs:1-200` | Complete |
| IPC/ZMQ Communication | `core/src/ipc.rs`, `zmq_agent_runner.rs` | Complete |
| GUI - Session Selector | `gui/src/session_selector.rs` | Complete |
| GUI - DAG Editor | `gui/src/dag_editor.rs` | Complete |
| GUI - Time Travel Debugger | `gui/src/time_travel.rs` | Complete |
| GUI - Chat Interface | `gui/src/chat_view.rs` | Complete |

**Partially Implemented:**

| Component | Location | Status | Notes |
|-----------|----------|--------|-------|
| LLM Streaming | `core/src/providers.rs:125-131` | Scaffold only | Returns `UnsupportedFeature` |
| Transaction Support | `core/src/state_store.rs:1036-1062` | Outlined | Not fully implemented |
| Context Menus | `gui/src/dag_canvas_interactions.rs:256-261` | Defined | No handlers |
| Edge Selection | `gui/src/dag_editor.rs:232` | Message defined | No visual implementation |
| File Tree | `gui/src/main.rs:1621-1697` | Requires feature flag | `agent-runner` feature |
| Knowledge Graph | `gui/src/main.rs:1699-2011` | Requires feature flag | Sample data available |
| Playback Timer | `gui/src/time_travel.rs:827-848` | Message defined | No subscription wired |

### 2. hl_quick Agent Architecture

The hl_quick prototype (`hl_quick.xml`) is a complete but simpler agent built in ~20 hours:

**Key Components:**

| Component | Description |
|-----------|-------------|
| **Server** | Hono + Node.js (should be Bun), SSE streaming, ~2,500 lines |
| **Agent Loop** | Multi-provider with doom loop detection (`agent.ts`) |
| **Subagents** | Parallel execution with role-based configs (simple/complex/researcher) |
| **Tools** | 4 tools: read_file, write_file, edit_file, bash |
| **Graph View** | SVG-based tree visualization with live updates |
| **Slash Commands** | File-based command system (`.agent/commands/`, `.claude/commands/`) |
| **Sessions** | File-based JSON persistence |
| **MCP Integration** | Server configuration and tool bridging |

**What Makes It Simpler:**

1. **No State Machine** - Just message history, no formal workflow states
2. **No Task Tracking** - No SCUD-style claiming/locking
3. **Simple Persistence** - JSON files instead of SQLite
4. **In-Process Everything** - No daemon architecture
5. **Single-File UI** - `App.tsx` at ~1,700 lines with graph view
6. **Direct Tool Execution** - No abstraction layers

### 3. SCUD Task Management Complexity Analysis

**Current SCUD Architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     5-Phase Workflow                            │
├─────────────────────────────────────────────────────────────────┤
│ PM Agent → SM Agent → Architect → Dev Agent → Retrospective    │
│ (Ideation)  (Planning)  (Design)  (Implementation) (Review)    │
├─────────────────────────────────────────────────────────────────┤
│                   Phase Gate Validation                         │
│ - Each agent checks current_phase in workflow-state.json        │
│ - Agent blocks if wrong phase                                   │
│ - Explicit handoff updates state                                │
├─────────────────────────────────────────────────────────────────┤
│                   Task-Level Mechanics                          │
│ - scud claim <task-id> --name <agent>                          │
│ - scud set-status <task-id> in-progress/done                    │
│ - scud release <task-id>                                        │
│ - File watching + event emission                                │
│ - WebSocket broadcast for status changes                        │
└─────────────────────────────────────────────────────────────────┘
```

**Complexity Points:**

1. **5 Agent Personas** - Each with phase validation, explicit handoffs
2. **Task Claiming/Locking** - `locked_by`, `locked_at` fields
3. **File Watching** - `ScgTaskEventEmitter` watches `.scud/tasks/tasks.json`
4. **Type Conversions** - Descartes ↔ SCUD mapping in `traits.rs:484-643`
5. **Workflow State** - `.scud/workflow-state.json` tracking current phase
6. **Wave Computation** - Parallel execution wave analysis

**What's Overengineered for Current Use:**

| Feature | Problem |
|---------|---------|
| Task-level claiming | Assumes multiple agents compete for tasks - rare in practice |
| Phase gates | Rigid workflow prevents natural iteration |
| File watching + events | Complex for simple status tracking |
| 5 agent personas | Cognitive overhead vs just having one agent |
| Handoff ceremony | Explicit state updates between phases |

### 4. Simplification Recommendations

Based on the user's direction, here's a proposed simplified task management:

**Keep:**
- DAG/graph structure for task planning (SCUD's `.scg` format is efficient)
- Wave computation for understanding parallelism
- Task dependencies and priority
- Complexity estimation (Fibonacci)

**Remove/Simplify:**
- Task claiming/locking at individual task level
- Explicit handoff ceremony between phases
- File watching for granular events
- 5 separate agent personas (consolidate to 1-2)

**Proposed Model:**

```
┌─────────────────────────────────────────────────────────────────┐
│                   Phase-Level Tracking                          │
├─────────────────────────────────────────────────────────────────┤
│ Phase = { planning, building, reviewing }                       │
│ - No granular task claiming                                     │
│ - Agent references task graph but doesn't lock tasks            │
│ - Phase transitions are lightweight state updates               │
├─────────────────────────────────────────────────────────────────┤
│                   Task Graph (Read-Only Reference)              │
│ - Tasks parsed from PRD into DAG                                │
│ - Dependencies computed, waves calculated                       │
│ - Agent uses graph for context, not as tracking system          │
│ - Completion tracked by code/test status, not claim/release     │
└─────────────────────────────────────────────────────────────────┘
```

### 5. Graph View Comparison

**hl_quick Graph View** (`App.tsx:10900-11824`):

| Feature | Implementation |
|---------|----------------|
| Node Types | user, assistant, tool, subagent-root, subagent-message |
| Layout | Custom tree layout with computed x/y positions |
| Rendering | SVG with `<g>` groups and transforms |
| Edges | Curved paths with Bezier curves |
| Interactions | Click to select, expand/collapse subagents |
| Live Updates | Reactive rebuild on message changes |
| Detail View | Popup showing full content, tool I/O |
| Toggle | Button to switch between list and graph view |

**Key Implementation Details:**

```typescript
// Node structure (hl_quick)
interface GraphNode {
  id: string
  type: GraphNodeType  // user | assistant | tool | subagent-root | subagent-message
  x: number; y: number  // Computed layout position
  label: string         // Truncated display text
  content?: string      // Full content for detail
  children: GraphNode[]
  expanded: boolean
  isLive: boolean       // Currently updating
}

// Layout constants
const GRAPH_LAYOUT = {
  nodeWidth: 200,
  nodeHeight: 60,
  toolNodeHeight: 36,
  verticalGap: 30,
  branchIndent: 60
}
```

**Descartes DAG Editor** (`gui/src/dag_editor.rs:56-935`):

| Feature | Implementation |
|---------|----------------|
| Node Types | DAGNode with task references |
| Layout | Manual positioning with snap-to-grid |
| Rendering | Iced Canvas widget |
| Edges | Multiple types (Dependency, SoftDependency, DataFlow, Trigger) |
| Interactions | Add/edit/delete nodes, edge creation, box selection, undo/redo |
| Tools | Select, AddNode, AddEdge, Delete, Pan |
| Persistence | DAG saved to files |

**Gap Analysis for Bringing hl_quick Graph View to Descartes:**

The Descartes DAG Editor is designed for **editing workflows**, while hl_quick's Graph View is for **observing conversation flow**. These are different use cases:

| Aspect | hl_quick | Descartes DAG Editor |
|--------|----------|---------------------|
| Purpose | Observe agent conversation | Edit task workflow |
| Data Source | Live messages/tool calls | Persisted DAG file |
| Nodes | Messages, tools, subagents | Tasks |
| Real-time | Yes, reactive updates | No, static editing |
| Layout | Automatic tree | Manual positioning |

**Recommendation:**

Add a new **Chat Graph View** to Descartes GUI that mirrors hl_quick's approach:

1. Create `chat_graph_view.rs` for conversation visualization
2. Use Iced Canvas for rendering (consistent with DAG editor)
3. Build nodes from session transcript (messages, tool calls, agent branches)
4. Automatic layout (no manual positioning)
5. Live updates via event subscriptions from daemon
6. Toggle between linear chat and graph view

### 6. What to Implement

**Priority 1: Simplify Task Management**

1. Remove task claiming/locking from SCUD commands
2. Consolidate 5 agent personas into 1-2 (or just reference roles in prompts)
3. Remove phase gate validation (keep as guidance, not enforcement)
4. Keep DAG structure for planning context

**Priority 2: Chat Graph View for Descartes**

1. Create new GUI component based on hl_quick's approach
2. Data model: `ChatGraphNode` with type, content, children, isLive
3. Layout: Simple tree with depth-based indentation
4. Rendering: Iced Canvas with rounded rectangles and curved edges
5. Interactions: Click to expand/view details, toggle view mode
6. Live updates: Subscribe to daemon events for streaming

**Priority 3: Remove Complexity**

1. Remove file watching in `ScgTaskEventEmitter` (or make optional)
2. Simplify `workflow-state.json` to just `{ phase: string }`
3. Remove SCUD type conversions in `traits.rs` (use unified model)

## Code References

**Descartes Core:**
- Agent spawning: `descartes/core/src/agent_runner.rs:41-347`
- Session management: `descartes/core/src/session_manager.rs:18-565`
- DAG structure: `descartes/core/src/dag.rs:324-391`
- SCUD storage: `descartes/core/src/scg_task_storage.rs:1-303`
- Type conversions: `descartes/core/src/traits.rs:484-643`

**Descartes GUI:**
- Main app: `descartes/gui/src/main.rs:72-101` (state struct)
- DAG Editor: `descartes/gui/src/dag_editor.rs:56-935`
- Chat View: `descartes/gui/src/chat_view.rs:13-238`
- Time Travel: `descartes/gui/src/time_travel.rs:27-848`

**Descartes Daemon:**
- Task events: `descartes/daemon/src/scg_task_event_emitter.rs:1-440`

**hl_quick Agent:**
- Graph view: `App.tsx:10900-11930` (in hl_quick.xml)
- Node building: `App.tsx:10950-11086`
- Layout: `App.tsx:11154-11205`
- SVG rendering: `App.tsx:11465-11663`

**SCUD Commands:**
- PM Agent: `.claude/commands/scud/pm.md:1-305`
- SM Agent: `.claude/commands/scud/sm.md:1-332`
- Architect: `.claude/commands/scud/architect.md:1-384`
- Dev Agent: `.claude/commands/scud/dev.md:1-356`

## Architecture Documentation

**Current Descartes Data Flow:**
```
User → GUI → RPC → Daemon → Agent Runner → Claude CLI
                      ↓
                State Store (SQLite)
                      ↓
             SCUD Task Storage (.scg files)
                      ↓
           Event Emitter → WebSocket → GUI
```

**Proposed Simplified Flow:**
```
User → GUI → RPC → Daemon → Agent Runner → Claude CLI
                      ↓
                State Store (SQLite)
                      ↓
             Task Graph (read-only context)
```

## Open Questions

1. Should the DAG editor and new Chat Graph View share canvas rendering code?
2. What's the right granularity for phase-level tracking (none, 2-3 phases, or keep 5)?
3. Should the simplified model still persist to `.scg` files or use a different format?
4. How much of SCUD's wave computation is valuable vs unnecessary overhead?

## Related Research

- Graph View Full Nodes Plan: `thoughts/shared/plans/2025-12-05-graph-view-full-nodes.md`
- Chat Graph View Plan: `thoughts/shared/plans/2025-12-04-chat-graph-view.md`
- Parallel Subagents Plan: `agent/thoughts/plans/parallel-subagents.md`
- SCUD Tool Integration: `agent/thoughts/shared/plans/2025-12-05-scud-tool-integration.md`
