# Descartes: The Composable AI Orchestration System
## Master Plan & Architecture Document

**Version:** 3.0 (Composable Rust + Iced Architecture)
**Date:** November 19, 2025
**Status:** Active

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

# As powerful as 
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

## 2. System Architecture

### 2.1 High-Level Stack
*   **Language**: 100% Rust (Backend & Frontend).
*   **GUI Framework**: Iced (The Elm Architecture in Rust).
*   **Communication**: Standard Stdin/Stdout/Stderr pipes for agents; internal message bus for system components.
*   **Communication**: Standard Stdin/Stdout/Stderr pipes for agents; internal message bus for system components.
*   **Storage**: Abstracted `StateStore` trait (Default: SQLite; Future: Postgres/S3).

### 2.2 Component Architecture
```
┌─────────────────────────────────────────────────────────┐
│                  Descartes Application                   │
│                     (Single Rust Binary)                 │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Iced UI Layer                       │   │
│  │      (Subscribes to Event Bus / Sends Cmds)      │   │
│  └─────────────────────────────────────────────────┘   │
│                          │                              │
│                  (Event/Command Protocol)               │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐   │
│  │           Core Orchestration Engine              │   │
│  │                                                  │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  │   │
│  │  │ AgentRunner│  │ StateStore │  │ContextSync │  │   │
│  │  │  (Trait)   │  │  (Trait)   │  │  (Trait)   │  │   │
│  │  └────────────┘  └────────────┘  └────────────┘  │   │
│  └─────────────────────────────────────────────────┘   │
│           │               │               │             │
│           ▼               ▼               ▼             │
│    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐      │
│    │Local Process│ │ SQLite DB   │ │ Local FS    │      │
│    │(Impl)       │ │ (Impl)      │ │ (Impl)      │      │
│    └─────────────┘ └─────────────┘ └─────────────┘      │
└─────────────────────────────────────────────────────────┘
```

### 2.3 Core Components

#### 2.3.1 Agent Runner (Abstraction)
*   **Role**: Spawns and supervises agent execution environments.
*   **Trait Definition**: `spawn(config) -> Result<Box<dyn AgentHandle>>`
*   **Implementations**:
    *   **LocalProcessRunner** (Default): Spawns standard OS processes (`std::process`).
    *   **DockerRunner** (Future): Spawns agents in isolated containers.
    *   **RemoteRunner** (Future): Dispatches agents to a remote cluster (e.g., Phoenix/K8s).
*   **Supported Engines**:
    *   **OpenCode**: Connects via HTTP API (`opencode serve`). Uses SSE for events and REST for control/permissions. (Target: V2)
    *   **Claude Code**: Connects via CLI Process (`claude -p`). Uses JSON Streaming (`--output-format=stream-json`) for events. (Target: V1 Reference Impl)
*   **Capabilities**:
    *   **JSON Streaming**: Parses structured log events from both engines to visualize agent thought processes.
    *   **Permission Prompting**: Intercepts permission requests (via API or JSON stream) and routes them to the **Notification Router**.
*   **Isolation**: Enforces resource limits and sandboxing appropriate to the runtime.

#### 2.3.2 Context Streaming Engine
*   **Role**: Efficiently loads, slices, and streams context to agents.
*   **Features**:
    *   **Streaming**: Processes gigabytes of context without loading all into RAM.
    *   **Slicing**: Filters context by semantic relevance, file patterns, or dependency graphs.
    *   **Syncing (Trait)**: `ContextSyncer` abstracts data access, allowing future support for remote worktree synchronization.
    *   **Concurrency Control**:
        *   **File Leases**: Implements a TTL-based locking system for shared files to prevent agent collisions.
        *   **Optimistic Locking**: Rejects writes if the file has changed since the agent last read it.
    *   **Sources**: Git, Filesystem, URLs, Vector DBs.

#### 2.3.3 Contract System
*   **Role**: Enforces strict input/output specifications for tasks.
*   **Features**:
    *   **Schema Validation**: JSON Schema for structured data.
    *   **Constraints**: "Must use crate X", "Max tokens Y".
    *   **Verification**: Runs tests or linters before accepting agent output.

#### 2.3.4 Session Manager
*   **Role**: Manages persistent groups of agents (Sessions).
*   **Features**:
    *   **Namespaces**: Groups agents under a session ID.
    *   **Persistence**: Saves full state to disk (SQLite + Files).
    *   **State Tuple**: Implements robust resumption by treating state as `(Context Window + Git Commit SHA)`.
    *   **Resumption Logic**:
        1.  **Restore Brain**: Load event history from SQLite.
        2.  **Restore Body**: `git checkout` the specific commit linked to the last event.
        3.  **Resume**: Agent continues exactly where it left off, with matching memory and file system.
    *   **Attach/Detach**: Tmux-like capability to leave and resume sessions.

#### 2.3.5 LSP Interface (Language Server)
*   **Role**: Integrates Descartes directly into editors (VS Code, Neovim) without plugins.
*   **Features**:
    *   **Diagnostics**: Agents publish "errors" or "warnings" directly to the editor gutter (e.g., Security Agent flagging unsafe code).
    *   **Code Actions**: "Lightbulb" actions to trigger agents (e.g., "Refactor this function", "Generate Tests").
    *   **Hover**: Show "Knowledge Graph" history and AI rationale when hovering over code.
    *   **Modes**:
        *   *Passive*: Lightweight monitoring (low cost).
        *   *Active*: Full agent swarms (higher cost, invoked explicitly).

#### 2.3.6 Secrets Vault
*   **Role**: Securely manages API keys, database credentials, and sensitive env vars.
*   **Features**:
    *   **Encryption**: AES-256 encryption for all stored secrets.
    *   **Masking**: Automatically redacts secrets from agent logs and UI outputs.
    *   **Injection**: Securely injects secrets into agent process environments at runtime.

#### 2.3.7 Notification Router
*   **Role**: Abstracted system for alerting users and receiving feedback.
*   **Features**:
    *   **Multi-Channel**: Supports Desktop Notifications, Telegram Bot, and Email (SMTP/SendGrid).
    *   **Bi-Directional**: Allows users to *reply* to notifications (e.g., via Telegram) to approve actions or guide agents.
    *   **Priority**: Routes critical alerts (e.g., "Approval Needed") to high-priority channels.

#### 2.3.8 Global Task Manager
*   **Role**: Centralized source of truth for task state, solving the "Split-Brain" problem of in-repo `tasks.json` across worktrees.
*   **Features**:
    *   **Centralized State**: Task status (Todo, In-Progress, Done) is stored in the `StateStore` (SQLite), not in the git repo.
    *   **Live Updates**: Agents receive task assignments via RPC/Streams, not by reading files.
    *   **Consistency**: Ensures all agents in all worktrees see the same global project state.
    *   **SCUD Integration**:
        *   Descartes exposes a standard SQLite schema for tasks.
        *   `scud` (CLI) is configured via `.descartes/config.toml` to read/write directly to this DB instead of local JSON.
        *   Enables seamless `scud task list` usage from any worktree.

#### 2.3.9 Execution Control (Debugger)
*   **Role**: Provides "GDB-like" control over agent execution.
*   **Communication**:
    *   **Control Plane**: **JSON-RPC 2.0** over Unix Socket (Daemon Mode). Allows CLI (`scud`), GUI (`descartes`), and IDEs to connect.
    *   **Data Plane**: **Shared Memory Ring Buffer** (via `ipmpsc` or `shared_memory`) for zero-copy context transfer.
    *   **Serialization**: **rkyv** (Zero-Copy) instead of JSON for high-throughput internal messages.
    *   **Future Proofing**: **ZeroMQ (ZMQ)** support planned for multi-machine agent swarms (e.g., remote GPU workers).
*   **Memory Management**:
    *   **Allocator**: **mimalloc** (Microsoft's allocator) for superior performance in highly threaded workloads.e.
*   **Features**:
    *   **Step-by-Step**: Pause execution before every LLM call.
    *   **Rewind**: Replay previous states from the `StateStore` (Time Travel).
    *   **Inspect**: View full prompt, context, and tool calls before they are sent.
    *   **Modify**: Edit the prompt or tool outputs on the fly during a pause.

#### 2.3.10 Swarm Protocol Engine
*   **Role**: Implements consensus and coordination strategies for multi-agent swarms.
*   **Features**:
    *   **Voting**: Agents can vote on proposals (e.g., `hybrid_tournament_reviewer`).
    *   **Debate**: Structured multi-turn debate protocols.
    *   **Map-Reduce**: Scatter-gather patterns for large tasks.
    *   **Composable**: Protocols are composable traits, not hardcoded logic.

#### 2.3.11 Approval Manager (Human-in-the-Loop)
*   **Role**: Centralized queue for sensitive actions requiring user permission.
*   **Features**:
    *   **Queue**: FIFO queue of pending approvals (Tool calls, File writes).
    *   **Policy**: Configurable auto-approve/auto-deny rules (e.g., "Auto-approve reads, ask for writes").
    *   **Timeout**: Auto-deny requests if user doesn't respond in N seconds.

#### 2.3.12 "Thoughts" System (Personal Context)
*   **Role**: Manages user's personal notes and scratchpads, separate from project code.
*   **Features**:
    *   **Global Storage**: `~/.descartes/thoughts` for cross-project notes.
    *   **Symlinks**: Link relevant thoughts into the current project context.
    *   **Searchable**: Indexed by Tantivy for agents to "remember" user preferences/ideas.

#### 2.3.13 Declarative Control Plane (Verifiable Workflows)
*   **Role**: Enables formally verifiable, deterministic agent orchestration.
*   **Features**:
    *   **Swarm.toml**: Define agent states and transitions declaratively (State Charts).
    *   **Verification**: Compile-time check for deadlocks and unreachable states.
    *   **Engine**: Powered by **Rust State Machines** (e.g., `rust-fsm` or `GraphFlow`).
    *   **Visualization**: Auto-generate Mermaid diagrams from the TOML definition.

#### 2.3.14 Unified LLM Provider Abstraction
*   **Role**: Decouples agent logic from specific models, enabling "Mix & Match" swarms.
*   **Trait**: `ModelBackend` (Async, Streaming, Token-Aware).
*   **Providers**:
    *   **API**: Direct HTTP clients for OpenAI, Anthropic, DeepSeek, Groq.
    *   **Headless CLI**: Wrappers for `claude` (Claude Code), `opencode`, or `gh` (GitHub CLI) to use them as "Agents".
        *   *Mechanism*: Spawns process, pipes `stdin`/`stdout`, parses ANSI/Text output into structured events.
    *   **Local**: Connects to `ollama` or `llama.cpp` server for offline/private inference.
*   **Router**: Smart routing based on task complexity (e.g., "Use Haiku for diffs, Opus for architecture").

---

## 3. User Interface (Iced)

### 3.1 Design Philosophy
*   **Type-Safe**: Leverages Rust's type system for reliable UI state.
*   **Native Performance**: GPU-accelerated rendering, low memory footprint.
*   **Unified Binary**: No separate frontend build step or Electron overhead.

### 3.2 Key Views
1.  **Dashboard**: High-level metrics (Active Agents, Token Usage, Cost).
2.  **Orchestration Board**: Visual DAG editor for workflows, drag-and-drop agent assignment.
3.  **Terminal Matrix**: Grid of terminal emulators attached to running agents.
4.  **Context Browser**: Visual explorer for loaded context and slices.
5.  **Debugger View**: Step-through interface for inspecting agent thought processes.

---

## 4. User Workflows

### 4.1 The "Unix Pipe" Flow
```bash
# Quick refactor using three specialized models
$ cat legacy_code.rs | \
  descartes spawn architect --model claude-3-opus --task "plan refactor" | \
  descartes spawn coder --model deepseek-coder --task "implement" | \
  descartes spawn reviewer --model gpt-4 --task "security check" > new_code.rs

# Debug a complex refactor
$ descartes spawn architect --task "refactor auth" --debug | descartes gui --attach
```

### 4.2 The "Swarm" Flow
1.  **Initialize**: `descartes session create feature-x`
2.  **Plan**: Spawn an Architect agent to read docs and generate a `tasks.json`.
3.  **Fan-Out**: `descartes distribute --input tasks.json --workers 10`
4.  **Monitor**: Open `descartes gui` to watch the swarm work in real-time.
5.  **Merge**: Agents submit Pull Requests or patch files directly upon contract validation.

### 4.3 The "Interactive" Flow
1.  User opens the GUI.
2.  Loads a project.
3.  Spawns a "Pair Programmer" agent.
4.  Agent attaches to the IDE terminal.
5.  User and Agent collaborate in a shared context.

---

## 5. Implementation Roadmap

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
    *   **LSP Server Implementation**:
        *   Integrate `tower-lsp` crate.
        *   Implement `textDocument/publishDiagnostics` for agent alerts.
        *   Implement `textDocument/codeAction` for quick triggers.
        *   Implement `textDocument/hover` for Knowledge Graph context.
    *   **Secrets Management**:
        *   Implement local encrypted vault.
        *   Add log masking middleware.
    *   **Notification System**:
        *   Create `NotificationRouter` trait.
        *   Implement Telegram Bot adapter with reply handling.

### Phase 3: The Interface (Weeks 9-12)
*   **Goal**: Native GUI with Iced (Phased Rollout).
*   **Deliverables**:
    *   **V1 (Read-Only Dashboard)**:
        *   Iced Application Shell.
        *   Live Task Board (Global Task Manager view).
        *   Swarm State Monitor (Active Agents/Worktrees).
        *   Simple "Open TUI" button for interaction.
    *   **V2 (Interactive)**:
        *   Visual DAG Editor (The Elm Architecture).
        *   Interactive Context Browser.

### Phase 4: Ecosystem (Months 4-6)
*   **Goal**: Production readiness.
*   **Deliverables**:
    *   Plugin System (WASM-based adapters?).
    *   Cloud Sync (Optional).
    *   Team Collaboration features.

---

## 6. The Knowledge Graph & Memory Layer

### 6.1 Overview
The Descartes Knowledge Graph is a **persistent, searchable memory system** that captures every decision, conversation, and code change in the AI development process. It transforms Descartes from a stateless orchestrator into an intelligent system that learns from its history and provides deep insights into how code evolved.

### 6.2 Core Value Propositions
1. **Total Recall**: Every AI interaction, decision, and output is preserved and searchable
2. **Time Machine**: Navigate to any point in development history and see the full context
3. **Git Archaeology**: Understand not just what changed, but why the AI made those changes
4. **Learning System**: Extract patterns from successful completions to improve future performance
5. **Parallel Universes**: Isolate experiments in git worktrees while maintaining global awareness

### 6.3 System Architecture

```
┌─────────────────────────────────────────────────┐
│                 Descartes Core                   │
│         (Orchestration, UI, Agents)             │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│            High-Performance RAG Layer            │
├─────────────────────────────────────────────────┤
│  ┌───────────────┐  ┌───────────────┐         │
│  │   LanceDB     │  │   Tantivy     │         │
│  │ (Vector Store)│  │ (Text Search) │         │
│  └───────┬───────┘  └───────┬───────┘         │
│          │                  │                   │
│          ▼                  ▼                   │
│  ┌─────────────────────────────────┐          │
│  │     SQLite (Graph/Metadata)     │          │
│  │                                  │          │
│  │  Events │ Nodes │ Edges │ Git   │          │
│  └─────────────────────────────────┘          │
│          │                                     │
│          ▼                                     │
│  ┌───────────────┐  ┌───────────────┐        │
│  │  Tree-sitter  │  │   Redb        │        │
│  │  (AST Parser) │  │  (Hot Cache)  │        │
│  └───────────────┘  └───────────────┘        │
└─────────────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│            Git Integration Layer                 │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌───────────────┐  ┌───────────────┐         │
│  │   Worktree    │  │   Gitoxide    │         │
│  │   Manager     │  │     (gix)     │         │
│  └───────────────┘  └───────────────┘         │
│                                                 │
│  Main Repo │ Feature Worktrees │ Meta Repo    │
└─────────────────────────────────────────────────┘
```

### 6.4 Data Model

#### Core Schema
```sql
-- Primary event table: The source of truth
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL, -- 'session_start', 'agent_output', 'code_generated', etc.
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    session_id TEXT NOT NULL,
    actor_type TEXT CHECK(actor_type IN ('user', 'agent', 'system')),
    actor_id TEXT,
    content TEXT NOT NULL,
    metadata JSON,
    git_commit TEXT,
    tokens_used INTEGER,
    cost_cents INTEGER
);

-- Full-text search index
CREATE VIRTUAL TABLE events_fts USING fts5(
    event_type, actor_id, content, metadata,
    content=events, content_rowid=id
);
```

#### Git Integration Schema
```sql
-- Git commits linked to AI events
CREATE TABLE git_checkpoints (
    commit_sha TEXT PRIMARY KEY,
    branch TEXT NOT NULL,
    worktree TEXT,
    commit_message TEXT,
    ai_rationale TEXT, -- Why the AI made these changes
    triggering_event_id INTEGER REFERENCES events(id),
    session_id TEXT
);
```

### 6.5 Core Features

### 6.5 The "Tri-Store" Architecture
To achieve "snappy" performance, we use specialized engines for each data type:
1.  **LanceDB (Vector Store)**: Serverless, Rust-native storage for embeddings. Handles disk-based vectors 100x faster than Parquet.
2.  **Tantivy (Text Search)**: High-performance FTS engine (Lucene alternative) for instant keyword search.
3.  **SQLite (Graph/Relation)**: Stores structural relationships (AST parent/child, file dependencies) and metadata.

### 6.5 The "Tri-Store" Architecture
To achieve "snappy" performance, we use specialized engines for each data type:
1.  **LanceDB (Vector Store)**: Serverless, Rust-native storage for embeddings. Handles disk-based vectors 100x faster than Parquet.
2.  **Tantivy (Text Search)**: High-performance FTS engine (Lucene alternative) for instant keyword search.
3.  **SQLite (Graph/Relation)**: Stores structural relationships (AST parent/child, file dependencies) and metadata.

#### Ingestion Pipeline (AST-Based)
*   **Tree-sitter**: Parses Code, Markdown, and Git Commits into ASTs.
*   **Semantic Chunking**: Chunks based on AST nodes (Functions, Classes, Headers) rather than arbitrary text splitting.
*   **Parallel Indexing**: Uses `rayon` to parallelize parsing and embedding across all cores.

#### Context Synchronization & Caching
*   **Prompt Caching**: Explicitly structure prompts to maximize cache hits (Static Prefix -> Tools -> Context -> User Query).
*   **Zero-Copy Transfer**: Use `rkyv` + Shared Memory to move large context chunks from `ContextSyncer` to `AgentRunner` without serialization overhead.

#### Time Machine Navigation
*   **Engine**: **Gitoxide (gix)**. Pure Rust, multithreaded Git implementation.
*   **Performance**: ~4x faster tree diffing than libgit2.
*   **Travel**: Reconstruct context and active agents at any timestamp.
*   **Explain**: Use `gix blame` (streaming) + event history to explain code origins.

#### Git Worktree Management
*   **Isolation**: Create separate worktrees for each swarm (e.g., `feature-auth`, `feature-payment`).
*   **Meta-Repo**: Track global orchestration state in a `.descartes/` meta-repository.
*   **Checkpointing**: Automatically commit changes with AI rationale and link to knowledge graph.

### 6.6 Search & Analytics

#### Multi-Modal Search
*   **Full-Text**: "Find mentions of 'authentication'".
*   **Semantic**: "Find code similar to this logic".
*   **Code Pattern**: "Find functions calling `authenticate`".
*   **Time-Based**: "Show events leading to commit `abc123`".

#### Pattern Learning
*   **Extraction**: Identify successful task approaches from history.
*   **Suggestion**: Recommend patterns for new tasks based on similarity.

---

## 7. Success Metrics
*   **Performance**: Spawn 100 agents in < 1s.
*   **Efficiency**: < 50MB overhead per agent process.
*   **Reliability**: 99.9% session recovery rate.
*   **Adoption**: 1,000+ GitHub stars in first 3 months.
*   **Knowledge Retention**: 100% of AI interactions searchable and linked to code.

---

## 8. Security & Governance
*   **Sandboxing**: Agents run with restricted permissions (optional Docker/WASM encapsulation).
*   **Approval Gates**: Configurable policies (e.g., "Require approval for file deletion").
*   **Audit Trail**: Cryptographically signed logs of all agent actions and user approvals.
