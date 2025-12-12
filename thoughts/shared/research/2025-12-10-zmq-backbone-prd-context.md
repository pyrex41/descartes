---
date: 2025-12-10T17:14:12Z
researcher: Claude Code
git_commit: 5fc83d472ca4774c018a21e3ba5c7e3d55db31d7
branch: backbone
repository: backbone
topic: "ZMQ Backbone PRD - Current Architecture Context"
tags: [research, codebase, descartes, zmq, rag, architecture, prd]
status: complete
last_updated: 2025-12-10
last_updated_by: Claude Code
---

# Research: ZMQ Backbone PRD - Current Architecture Context

**Date**: 2025-12-10T17:14:12Z
**Researcher**: Claude Code
**Git Commit**: 5fc83d472ca4774c018a21e3ba5c7e3d55db31d7
**Branch**: backbone
**Repository**: backbone

## Research Question

Review the ZMQ Backbone PRD at `.scud/docs/prd/backbone.md` and the codebase to prepare the research context needed to flesh out into a more detailed PRD document.

## Summary

This research documents the current state of the Descartes architecture to provide context for the "ZMQ Backbone" refactoring PRD. The codebase is a comprehensive Rust-based AI coding agent orchestration system with:

- **59 database tables** across semantic analysis, RAG, secrets, leasing, and state management
- **Full ZeroMQ implementation** with REQ/REP/DEALER/ROUTER sockets and MessagePack serialization
- **Comprehensive RAG system** using LanceDB (vectors) + Tantivy (full-text) + Tree-Sitter (AST)
- **Sophisticated IPC** with message bus, dead letter queues, backpressure, and event streaming
- **Time Travel debugging** via "Brain vs Body" model (SQLite events + Git commits)
- **Real-time GUI** with Iced framework and WebSocket event streaming

---

## Detailed Findings

### 1. RAG and Semantic Analysis Components

**Location**: `/descartes/agent-runner/src/`

#### Current Implementation

The codebase contains a **complete RAG system** combining:

1. **Tree-Sitter AST Parsing** (`parser.rs`, `grammar.rs`, `semantic.rs`, `traversal.rs`)
   - Languages: Rust, Python, JavaScript, TypeScript (v0.23 grammars)
   - Semantic node extraction: functions, classes, structs, imports
   - Traversal strategies: BreadthFirst, DepthFirstPreOrder, DepthFirstPostOrder

2. **Vector Search - LanceDB** (`rag.rs:576-720`)
   - Table name: "code_chunks"
   - Configurable dimension (default: 1536 for OpenAI)
   - Methods: `add_chunks()`, `search()`, `delete_chunks()`, `clear()`

3. **Full-Text Search - Tantivy** (`rag.rs:722-874`)
   - Schema: id, content, file_path, language, chunk_type, metadata
   - Methods: `add_chunks()`, `search()`, `clear()`

4. **Hybrid Search** (`rag.rs:1083-1124`)
   - Runs vector + full-text in parallel via `tokio::join!`
   - Weighted scoring: vector_weight=0.7, fulltext_weight=0.3 (configurable)

5. **Embedding Providers** (`rag.rs:164-341`)
   - `OpenAiEmbeddings`: endpoint `https://api.openai.com/v1/embeddings`
   - `AnthropicEmbeddings`: endpoint `https://api.voyageai.com/v1/embeddings`
   - Caching with LRU + blake3 hashing (`EmbeddingCache`)

6. **Knowledge Graph** (`knowledge_graph.rs:702-960`)
   - `KnowledgeNode`: code entities with qualified names, signatures, parameters
   - `KnowledgeEdge`: relationships (Calls, Imports, Inherits, Implements, Uses, Defines)
   - Bidirectional linking with FileTree via `knowledge_links`

#### Dependencies to Remove (per PRD)

```toml
# From agent-runner/Cargo.toml
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
lancedb = "0.22"
tantivy = "0.22"
```

#### Files to Remove (per PRD)

- `agent-runner/src/rag.rs` (1161 lines)
- `agent-runner/src/semantic.rs` (278 lines)
- `agent-runner/src/knowledge_graph.rs` (1005 lines)
- `agent-runner/src/knowledge_graph_overlay.rs` (772 lines)
- `agent-runner/src/parser.rs` (323 lines)
- `agent-runner/src/grammar.rs` (88 lines)
- `agent-runner/src/traversal.rs` (290 lines)
- `agent-runner/src/file_tree_builder.rs` (711 lines)

---

### 2. IPC System and Internal Message Bus

**Location**: `/descartes/core/src/ipc.rs` and `/descartes/daemon/src/events.rs`

#### Current Implementation

**Message Bus** (`ipc.rs:727-923`):
- `IpcMessage` structure with UUID, type, sender, recipient, topic, payload, priority, TTL
- Message types: DirectMessage, PublishMessage, Request, Response, Subscribe, Unsubscribe, Ack, Error, Heartbeat, Control
- Statistics tracking: total messages, failed messages, per-topic/sender counts
- History buffer: configurable VecDeque (default 10,000)

**Dead Letter Queue** (`ipc.rs:367-405`):
- `Arc<Mutex<VecDeque<(IpcMessage, String)>>>` stores failed messages with reason
- Max size: 1000 messages (FIFO eviction)
- Reasons: size exceeded, TTL expired, routing failed

**Backpressure Controller** (`ipc.rs:408-477`):
- `max_pending`: 10,000 messages
- `wait_duration`: 100ms when backpressured
- `timeout`: 30s max
- Uses `AtomicU64` for pending count

**Message Router** (`ipc.rs:480-599`):
- Priority-sorted routing rules
- Filters: msg_type, sender, recipient, topic
- Dynamic handler registration via `DashMap`

**Transport Layer** (`ipc.rs:203-361`):
- `UnixSocketTransport`: Unix domain sockets with length-prefixed framing
- `MemoryTransport`: Tokio mpsc unbounded channels (for testing)

**Event Bus** (`events.rs:285-388`):
- `broadcast::Sender<DescartesEvent>` with capacity 1000
- Event types: AgentEvent, TaskEvent, WorkflowEvent, SystemEvent, StateEvent
- Filtering by agent_ids, task_ids, workflow_ids, event_categories

#### PRD Direction

The PRD calls for **simplification**: "If ZMQ handles the transport, the internal bus should just be simple Tokio channels bridging the ZMQ socket to the State Store."

Current complexity to potentially simplify:
- Dead Letter Queue (may not be needed with ZMQ reliability)
- Backpressure Controller (ZMQ has built-in high water marks)
- Message Router with complex filtering (could be event bus only)

---

### 3. Database Schema

**Location**: Multiple locations across `core/` and `agent-runner/`

#### Current Table Count: **59 tables** (+ 2 FTS virtual tables)

**Core State Management (8 tables)**:
- `events`, `tasks`, `sessions`, `agent_states`, `state_transitions`, `state_snapshots`, `task_dependencies`, `migrations`

**Agent History (3 tables)**:
- `agent_history_events`, `history_snapshots`, `snapshot_events`

**Lease Management (3 tables)**:
- `leases`, `lease_history`, `lease_configs`

**Secrets Management (11 tables)**:
- `master_keys`, `secrets`, `secret_tags`, `secret_values`, `access_control`, `audit_logs`, `secret_sessions`, `rotation_policies`, `master_key_rotations`, `encryption_metadata`, `access_attempts`

**Semantic Analysis (13 tables)**:
- `semantic_nodes`, `semantic_node_parameters`, `semantic_node_type_parameters`, `file_dependencies`, `semantic_relationships`, `node_call_graph`, `ast_parsing_sessions`, `file_metadata`, `circular_dependencies`, `semantic_search_cache`, `code_change_tracking`, `rag_metadata`, `semantic_index_stats`

**RAG Layer (13 tables)**:
- `rag_store_state`, `vector_metadata`, `fts_index_metadata`, `sqlite_index_metadata`, `hybrid_search_config`, `search_results_log`, `document_chunks`, `rag_context_windows`, `embedding_cache`, `relevance_feedback`, `sync_audit_trail`, `consistency_checks`, `rag_performance_stats`

**System Support (8 tables)**:
- `summary_statistics`, `query_statistics`, `schema_versions`, `migration_operations`, `configuration`, `rollback_points`, `schema_documentation`, `migrations`

#### PRD Target Schema (3 tables)

Per the PRD:
```sql
-- agents: (UUID, status, ZMQ address/metadata)
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    zmq_address TEXT,
    metadata TEXT
);

-- events: Append-only log (source of truth for GUI)
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    agent_id TEXT,
    content TEXT NOT NULL,
    metadata TEXT
);

-- snapshots: Time Travel (Git Commit Hash + Last Event ID)
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    git_commit_hash TEXT NOT NULL,
    last_event_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
```

#### Tables to Drop (per PRD "Delete" list)

All 13 semantic analysis tables:
- `semantic_nodes`, `semantic_node_parameters`, `semantic_node_type_parameters`
- `file_dependencies`, `semantic_relationships`, `node_call_graph`
- `ast_parsing_sessions`, `file_metadata`, `circular_dependencies`
- `semantic_search_cache`, `code_change_tracking`, `rag_metadata`, `semantic_index_stats`

All 13 RAG layer tables.

---

### 4. ZeroMQ Implementation

**Location**: `/descartes/core/src/zmq_*.rs`

#### Current Implementation (Already ZMQ-Ready)

**Protocol** (`zmq_agent_runner.rs:40-47`):
- Version: "1.0.0"
- Max message size: 10 MB
- Default timeout: 30 seconds
- Serialization: MessagePack (rmp-serde)

**Socket Types** (`zmq_communication.rs:103-113`):
- `Req`: Client-side synchronous (connect)
- `Rep`: Server-side synchronous (bind)
- `Dealer`: Client-side asynchronous (connect)
- `Router`: Server-side asynchronous (bind)

**Message Types** (`zmq_agent_runner.rs:536-580`):
```rust
pub enum ZmqMessage {
    SpawnRequest(SpawnRequest),
    SpawnResponse(SpawnResponse),
    ControlCommand(ControlCommand),
    CommandResponse(CommandResponse),
    StatusUpdate(StatusUpdate),
    ListAgentsRequest(ListAgentsRequest),
    ListAgentsResponse(ListAgentsResponse),
    HealthCheckRequest(HealthCheckRequest),
    HealthCheckResponse(HealthCheckResponse),
    CustomActionRequest(CustomActionRequest),
    BatchControlCommand(BatchControlCommand),
    BatchControlResponse(BatchControlResponse),
    OutputQueryRequest(OutputQueryRequest),
    OutputQueryResponse(OutputQueryResponse),
}
```

**Connection Management** (`zmq_communication.rs:243-550`):
- State machine: Disconnected → Connecting → Connected → Reconnecting → Failed
- Exponential backoff: 100ms initial, 30s max
- Statistics tracking: messages, bytes, errors, reconnections

**Server** (`zmq_server.rs:125-400`):
- Default endpoint: `tcp://0.0.0.0:5555`
- `agents: DashMap<Uuid, Arc<ManagedAgent>>` registry
- Background tasks: status updates, agent monitoring
- Graceful shutdown with SIGTERM → wait 2s → SIGKILL

**Client** (`zmq_client.rs:61-850`):
- Command queueing for offline operation
- Status update subscriptions
- Batch operations support

#### PRD Alignment

The current ZMQ implementation **already matches** the PRD direction:
- Control Plane: Daemon binds ROUTER socket ✓
- Data Plane: Agents connect via DEALER sockets ✓
- Flow: Agent output → ZMQ Message → Daemon → SQLite → GUI ✓

**No major changes needed** - just use it as the primary interface.

---

### 5. Git-Based Time Travel Features

**Location**: `/descartes/core/src/body_restore.rs`, `brain_restore.rs`, `time_travel_integration.rs`

#### Current Implementation

**"Brain vs Body" Model**:

1. **Brain State** (Event Sourcing - `brain_restore.rs`)
   - `BrainState`: thought_history, decision_tree, memory, conversation_state
   - `AgentHistoryEvent`: event_id, timestamp, event_type, event_data, git_commit_hash
   - Replay via `replay_events()` with causality sorting
   - Validation: orphan detection, consistency checks

2. **Body State** (Git - `body_restore.rs`)
   - Uses **gitoxide** (`gix` crate) for pure Rust git operations
   - `CommitInfo`: hash, message, author, timestamp, parents
   - `RepositoryBackup`: head_commit, branch_name, stash_ref
   - Checkout with automatic rollback on failure

3. **Coordination** (`time_travel_integration.rs`)
   - `RewindPoint`: timestamp, event_id, git_commit, snapshot_id
   - `RewindResult`: brain_result, body_result, validation
   - Validates brain-body consistency via git_commit_hash matching

**Key Methods**:
- `rewind_to(point)`: Restores both brain and body to point
- `resume_from(point)`: Sets up debugging from point
- `undo_rewind(backup_id)`: Rolls back to backup

#### PRD Alignment

PRD says: "Brain: Replaying the `events` table. Confirm that we do *not* need to restore complex AST states, only the event log history."

Current implementation **supports this** - the `brain_restore.rs` replays events from SQLite without requiring semantic analysis. The git-based body restore (`body_restore.rs`) remains needed.

---

### 6. Agent Architecture and Runner

**Location**: `/descartes/core/src/agent_runner.rs`, `/descartes/daemon/src/`

#### Current Implementation

**LocalProcessRunner** (`agent_runner.rs:37-274`):
- Spawns CLI processes (claude, opencode) as child processes
- `DashMap<Uuid, Arc<RwLock<LocalAgentHandle>>>` for concurrent registry
- Piped stdin/stdout/stderr with broadcast channels for TUI attachment
- Background tasks: health checker (30s), exit observer (250ms poll)

**Agent Lifecycle**:
- Status: Idle → Initializing → Running/Thinking → Paused → Completed/Failed/Terminated
- Pause modes: Cooperative (stdin message) vs Forced (SIGSTOP)
- Attach system: token-based TUI attachment with 5-minute TTL

**Daemon Architecture** (`daemon/src/main.rs`):
- Single global daemon per system
- HTTP (8080) + WebSocket (8081) endpoints
- JSON-RPC methods: spawn, list_tasks, approve, get_state, pause, resume, attach.*

#### PRD Alignment

Current architecture is **well-positioned**:
- Agents already connect as clients (local or remote)
- ZMQ can replace internal IPC for uniform local/remote handling
- Event sourcing to GUI via WebSocket already works

---

### 7. GUI and WebSocket Observability

**Location**: `/descartes/gui/src/`, `/descartes/daemon/src/events.rs`, `event_stream.rs`

#### Current Implementation

**Event Bus** (`events.rs`):
- Broadcast channel capacity: 1000 events
- Event filtering by agent/task/workflow ID and category
- Statistics tracking

**WebSocket Server** (`event_stream.rs`):
- Protocol: ServerMessage (Event, SubscriptionConfirmed, Ping) / ClientMessage (Subscribe, UpdateFilter, Pong)
- Heartbeat: 30 seconds
- Per-connection subscription management

**GUI Framework** (`gui/src/main.rs`):
- Iced 0.13 with tokio, canvas, advanced features
- Views: Sessions, Dashboard, Chat, TaskBoard, SwarmMonitor, Debugger, DagEditor, FileBrowser, KnowledgeGraph
- Real-time updates via `EventHandler` subscription

**Swarm Monitor** (`swarm_monitor.rs`):
- 60 FPS animation for "thinking" agents
- Live controls: pause/resume/attach buttons
- Performance tracking: FPS, frame times

#### PRD Alignment

The GUI observability layer is **already optimized** for the event sourcing model:
- Events flow: Agent → ZMQ → Daemon → SQLite → WebSocket → GUI
- No dependency on semantic analysis for GUI operation

---

### 8. Project Structure and Dependencies

**Workspace Layout**:
```
descartes/
├── core/           # 77 dependencies, traits/providers/tools/state/zmq
├── cli/            # 8 dependencies, command-line interface
├── daemon/         # 23 dependencies, HTTP/WS/RPC server
├── gui/            # 5 dependencies, Iced native GUI
└── agent-runner/   # 22 dependencies, semantic parsing + RAG
```

**Heavy Dependencies (Candidates for Removal)**:
| Dependency | Version | Crate | Impact |
|------------|---------|-------|--------|
| `lancedb` | 0.22 | agent-runner | Vector database |
| `tantivy` | 0.22 | agent-runner | Full-text search |
| `tree-sitter` | 0.24 | agent-runner | AST parsing |
| `tree-sitter-*` | 0.23 | agent-runner | Language grammars |
| `ndarray` | 0.15 | agent-runner | Embedding operations |

**Dependencies to Keep**:
| Dependency | Version | Crate | Purpose |
|------------|---------|-------|---------|
| `zeromq` | 0.4 | core | Agent communication |
| `sqlx` | 0.7 | core | SQLite persistence |
| `gix` | 0.68 | core | Git operations |
| `tokio` | 1.48 | all | Async runtime |
| `iced` | 0.13 | gui | GUI framework |

---

## Architecture Documentation

### Current Data Flow

```
User Input → CLI/GUI
    ↓
Model Provider (Anthropic/OpenAI/Ollama)
    ↓
Agent Runner (Local Process or ZMQ Remote)
    ↓
Tool Execution (read/write/edit/bash)
    ↓
State Store (SQLite - 59 tables)
    ↓
Event Bus → WebSocket → GUI
```

### PRD Target Data Flow

```
User Input → CLI/GUI
    ↓
ZMQ Control Plane (ROUTER socket)
    ↓
Agent (DEALER socket - local or remote)
    ↓
Tool Execution (read/write/edit/bash)
    ↓
ZMQ Message → Daemon
    ↓
SQLite Event Log (3 tables: agents, events, snapshots)
    ↓
WebSocket → GUI
```

---

## Code References

### Files to Delete
- `descartes/agent-runner/src/rag.rs:1-1161` - Full RAG system
- `descartes/agent-runner/src/semantic.rs:1-278` - Semantic extraction
- `descartes/agent-runner/src/knowledge_graph.rs:1-1005` - Knowledge graph
- `descartes/agent-runner/src/knowledge_graph_overlay.rs:1-772` - Graph overlay
- `descartes/agent-runner/src/parser.rs:1-323` - Tree-sitter parser
- `descartes/agent-runner/src/grammar.rs:1-88` - Grammar loading
- `descartes/agent-runner/src/traversal.rs:1-290` - AST traversal
- `descartes/agent-runner/src/file_tree_builder.rs:1-711` - File tree builder

### Files to Simplify
- `descartes/core/src/ipc.rs:1-923` - Simplify to basic Tokio channels
- `descartes/core/src/state_store.rs:1-827` - Reduce to 3-table schema
- `descartes/agent-runner/migrations/*.sql` - Drop semantic/RAG tables

### Files to Keep (Core ZMQ)
- `descartes/core/src/zmq_agent_runner.rs:1-900` - Message protocol
- `descartes/core/src/zmq_communication.rs:1-652` - Connection management
- `descartes/core/src/zmq_server.rs:1-777` - Server implementation
- `descartes/core/src/zmq_client.rs:1-866` - Client implementation

### Files to Keep (Time Travel)
- `descartes/core/src/body_restore.rs:1-973` - Git restoration
- `descartes/core/src/brain_restore.rs:1-1193` - Event replay
- `descartes/core/src/agent_history.rs:1-1306` - Event sourcing

### Files to Keep (GUI)
- `descartes/daemon/src/events.rs:1-489` - Event bus
- `descartes/daemon/src/event_stream.rs:1-257` - WebSocket streaming
- `descartes/gui/src/` - All GUI components

---

## Related Research

- `.scud/docs/prd/backbone.md` - Original PRD document

---

## Open Questions

1. **Agent-Runner Crate Fate**: Should the entire `agent-runner` crate be removed, or should it be gutted and repurposed for minimal functionality?

2. **IPC Simplification Scope**: How much of the current IPC system should be retained? The event bus for GUI updates seems valuable even with ZMQ.

3. **Secrets Management**: The secrets subsystem (11 tables) is not mentioned in the PRD. Should it be preserved?

4. **Lease Management**: The lease system (3 tables) for file locking - is this needed for multi-agent coordination?

5. **Migration Strategy**: Should the schema change be a single migration or phased approach?

6. **Plugin System**: The WASM plugin system (`plugins/`) is not mentioned. Retain or remove?

---

## Quantitative Summary

| Category | Current | PRD Target | Reduction |
|----------|---------|------------|-----------|
| Database Tables | 59 | 3 | 95% |
| RAG/Semantic Files | 8 | 0 | 100% |
| Heavy Dependencies | 5 (lancedb, tantivy, tree-sitter, ndarray, 4 grammars) | 0 | 100% |
| IPC Complexity | DLQ, Backpressure, Router, 2 Transports | Tokio channels | ~80% |

---

## Recommendations for PRD Detail

The PRD should specify:

1. **Exact file list** for deletion with line counts
2. **Migration SQL** for the 3-table schema
3. **Dependency removal** from Cargo.toml files
4. **IPC replacement** code for Tokio channel bridge
5. **Feature flag strategy** for gradual rollout
6. **Test coverage** requirements for refactored components
7. **Performance benchmarks** before/after (compile time, binary size, memory)
