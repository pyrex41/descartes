# Product Requirements Document: Descartes
## A Composable AI Agent Orchestration System
**Version:** 3.0 (Composable Rust + Iced Architecture)
**Date:** November 19, 2025
**Status:** Draft

---

## 1. Executive Summary

### 1.1 Product Overview
Descartes is a composable AI agent orchestration system that brings the Unix philosophy to AI development. Built entirely in **Rust** with a native cross-platform GUI using **Iced**, it treats AI agents as first-class operating system processes that can be composed, piped, and orchestrated like traditional Unix tools.

Unlike monolithic AI IDEs or simple chat interfaces, Descartes provides a set of focused, composable tools that combine to create sophisticated multi-agent systems. Each agent runs as an isolated process, contexts flow as streams, and orchestration happens through an elegant combination of CLI commands and visual tools.

### 1.2 Key Innovation
Descartes solves the fundamental problem of AI development complexity through radical simplicity:

```bash
# As simple as Unix pipes
$ descartes spawn architect < requirements.md | descartes spawn coder > implementation.rs

# As powerful as distributed systems
$ descartes swarm deploy --agents 50 --strategy skill-match --contract strict

# As intuitive as modern GUIs
$ descartes gui  # Launch visual orchestration interface
```

### 1.3 Core Principles
1.  **Composability Over Integration**: Small tools that combine rather than one big tool.
2.  **Processes Over Threads**: True parallelism and isolation through OS processes.
3.  **Streams Over State**: Data flows through transformations, not sitting in memory.
4.  **Contracts Over Conversations**: Explicit specifications with validation.
5.  **Native Over Web**: Fast, efficient, cross-platform native applications (Rust + Iced).

---

## 2. Problem Statement

### 2.1 Current Market Failures
*   **The Monolith Problem**: Tools like Cursor or Windsurf are rigid, forcing users into specific workflows and ecosystems.
*   **The Toy Problem**: Simple chat interfaces lack state, orchestration, and safety for complex engineering.
*   **The Integration Problem**: Existing tools don't play well with standard Unix pipelines, CI/CD, or existing CLI workflows.

### 2.2 User Pain Points
*   **Context Amnesia**: "Every new chat loses all previous understanding."
*   **Orchestration Nightmare**: "I can't coordinate Claude (planning) and DeepSeek (coding) effectively."
*   **Safety Gaps**: "AI randomly deletes files or hallucinates APIs."
*   **Workflow Rigidity**: "I can't customize the pipeline for my specific team needs."

---

## 3. System Architecture

### 3.1 High-Level Stack
*   **Language**: 100% Rust (Backend & Frontend).
*   **GUI Framework**: Iced (The Elm Architecture in Rust).
*   **Communication**: Standard Stdin/Stdout/Stderr pipes for agents; internal message bus for system components.
*   **Storage**: SQLite (State), File System (Contexts), Vector DB (Embeddings).

### 3.2 Component Architecture
```
┌─────────────────────────────────────────────────────────┐
│                  Descartes Application                   │
│                     (Single Rust Binary)                 │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Iced UI Layer                       │   │
│  │  (View, Update, Message - No JSON/IPC overhead)  │   │
│  └─────────────────────────────────────────────────┘   │
│                          │                              │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐   │
│  │           Core Orchestration Engine              │   │
│  │  (Process Mgr, Context Slicer, Session Mgr)      │   │
│  └─────────────────────────────────────────────────┘   │
│                          │                              │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐   │
│  │            Agent Process Pool                    │   │
│  │         (Spawned External Processes)             │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 3.3 Core Components

#### 3.3.1 Process Manager
*   **Role**: Spawns and supervises agent processes.
*   **Features**: Resource limits (cgroups), signal handling, I/O piping.
*   **Isolation**: Each agent is a separate OS process, ensuring true parallelism and crash resilience.

#### 3.3.2 Context Streaming Engine
*   **Role**: Efficiently loads, slices, and streams context to agents.
*   **Features**:
    *   **Streaming**: Processes gigabytes of context without loading all into RAM.
    *   **Slicing**: Filters context by semantic relevance, file patterns, or dependency graphs.
    *   **Sources**: Git, Filesystem, URLs, Vector DBs.

#### 3.3.3 Contract System
*   **Role**: Enforces strict input/output specifications for tasks.
*   **Features**:
    *   **Schema Validation**: JSON Schema for structured data.
    *   **Constraints**: "Must use crate X", "Max tokens Y".
    *   **Verification**: Runs tests or linters before accepting agent output.

#### 3.3.4 Session Manager
*   **Role**: Manages persistent groups of agents (Sessions).
*   **Features**:
    *   **Namespaces**: Groups agents under a session ID.
    *   **Persistence**: Saves full state to disk (SQLite + Files).
    *   **Attach/Detach**: Tmux-like capability to leave and resume sessions.

---

## 4. User Interface (Iced)

### 4.1 Design Philosophy
*   **Type-Safe**: Leverages Rust's type system for reliable UI state.
*   **Native Performance**: GPU-accelerated rendering, low memory footprint.
*   **Unified Binary**: No separate frontend build step or Electron overhead.

### 4.2 Key Views
1.  **Dashboard**: High-level metrics (Active Agents, Token Usage, Cost).
2.  **Orchestration Board**: Visual DAG editor for workflows, drag-and-drop agent assignment.
3.  **Terminal Matrix**: Grid of terminal emulators attached to running agents.
4.  **Context Browser**: Visual explorer for loaded context and slices.

---

## 5. User Workflows

### 5.1 The "Unix Pipe" Flow
```bash
# Quick refactor using three specialized models
$ cat legacy_code.rs | \
  descartes spawn architect --model claude-3-opus --task "plan refactor" | \
  descartes spawn coder --model deepseek-coder --task "implement" | \
  descartes spawn reviewer --model gpt-4 --task "security check" > new_code.rs
```

### 5.2 The "Swarm" Flow
1.  **Initialize**: `descartes session create feature-x`
2.  **Plan**: Spawn an Architect agent to read docs and generate a `tasks.json`.
3.  **Fan-Out**: `descartes distribute --input tasks.json --workers 10`
4.  **Monitor**: Open `descartes gui` to watch the swarm work in real-time.
5.  **Merge**: Agents submit Pull Requests or patch files directly upon contract validation.

### 5.3 The "Interactive" Flow
1.  User opens the GUI.
2.  Loads a project.
3.  Spawns a "Pair Programmer" agent.
4.  Agent attaches to the IDE terminal.
5.  User and Agent collaborate in a shared context.

---

## 6. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)
*   **Goal**: Working CLI with single-agent spawning.
*   **Deliverables**:
    *   Rust Process Manager.
    *   Basic Context Loading (Files/Git).
    *   CLI: `spawn`, `ps`, `kill`, `logs`.
    *   Simple Stdin/Stdout piping.

### Phase 2: Composition (Weeks 5-8)
*   **Goal**: Multi-agent pipelines and Contracts.
*   **Deliverables**:
    *   Context Slicing & Streaming.
    *   Contract Validator (Schema/Test runner).
    *   Message Bus for inter-agent comms.
    *   Session Persistence (SQLite).

### Phase 3: The Interface (Weeks 9-12)
*   **Goal**: Native GUI with Iced.
*   **Deliverables**:
    *   Iced Application Shell.
    *   Terminal Widget.
    *   Agent Monitoring Dashboard.
    *   Visual Workflow Editor.

### Phase 4: Ecosystem (Months 4-6)
*   **Goal**: Production readiness.
*   **Deliverables**:
    *   Plugin System (WASM-based adapters?).
    *   Cloud Sync (Optional).
    *   Team Collaboration features.

---

## 7. Success Metrics
*   **Performance**: Spawn 100 agents in < 1s.
*   **Efficiency**: < 50MB overhead per agent process.
*   **Reliability**: 99.9% session recovery rate.
*   **Adoption**: 1,000+ GitHub stars in first 3 months.

---

## 8. Security & Governance
*   **Sandboxing**: Agents run with restricted permissions (optional Docker/WASM encapsulation).
*   **Approval Gates**: Configurable policies (e.g., "Require approval for file deletion").
*   **Audit Trail**: Cryptographically signed logs of all agent actions and user approvals.
