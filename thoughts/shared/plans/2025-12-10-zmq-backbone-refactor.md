# ZMQ Backbone Refactor Implementation Plan

## Overview

Aggressive simplification of the Descartes architecture to focus on ZeroMQ-driven agent orchestration. We're removing the RAG/semantic analysis stack, simplifying the database to an event-sourcing model, and eliminating complex IPC in favor of ZMQ + simple Tokio channels.

**This is greenfield** - no existing users, so we can do this in one pass.

## Current State Analysis

### Code to Remove
| Component | Location | Lines | Purpose (Removing) |
|-----------|----------|-------|-------------------|
| agent-runner crate | `descartes/agent-runner/` | 6,116 | RAG, semantic analysis, knowledge graphs |
| Complex IPC | `core/src/ipc.rs` | 1,049 | DLQ, backpressure, routing (ZMQ handles this) |
| IPC benchmarks | `core/benches/ipc_*.rs` | ~20,000 | Old IPC performance tests |

### Code to Keep
| Component | Location | Purpose |
|-----------|----------|---------|
| ZMQ implementation | `core/src/zmq_*.rs` | Already production-ready |
| EventBus | `daemon/src/events.rs` | GUI event streaming |
| Time Travel | `core/src/body_restore.rs`, `brain_restore.rs` | Git + event replay |
| Secrets | `core/src/secrets*.rs` | API key management |
| Leases | `core/src/lease*.rs` | Multi-agent coordination |
| State Store | `core/src/state_store.rs` | Simplify, don't delete |

### Key Discoveries
- GUI already has `#[cfg(not(feature = "agent-runner"))]` stubs - removal is seamless
- ZMQ implementation is complete with ROUTER/DEALER, MessagePack, reconnection logic
- EventBus is lightweight (broadcast channel) - keep it
- Secrets (11 tables) and Leases (3 tables) are isolated - keep them

## Desired End State

After this refactor:

1. **Workspace**: 4 crates (core, cli, daemon, gui) - `agent-runner` deleted
2. **Database**: Simplified schema focused on events + agents + snapshots
3. **IPC**: ZMQ for agent communication, EventBus for GUI streaming
4. **Binary size**: Significantly smaller (no lancedb, tantivy, tree-sitter)
5. **Compile time**: Much faster (heavy deps removed)

### Verification
```bash
# Build succeeds
cargo build --workspace

# Tests pass
cargo test --workspace

# Daemon starts and accepts connections
cargo run --bin descartes-daemon &
curl http://localhost:8080/health

# GUI launches (without agent-runner feature)
cargo run --bin descartes-gui

# ZMQ benchmarks run
cargo bench --bench zmq_benchmarks
```

## What We're NOT Doing

- **NOT** touching secrets management (11 tables) - isolated and essential
- **NOT** touching lease management (3 tables) - needed for multi-agent
- **NOT** adding feature flags - greenfield, just delete
- **NOT** preserving IPC benchmarks - replacing with ZMQ benchmarks
- **NOT** changing ZMQ protocol - it's already correct

---

## Phase 1: Delete agent-runner Crate

### Overview
Remove the entire `agent-runner` crate and all references to it.

### Changes Required

#### 1. Delete the crate directory
```bash
rm -rf descartes/agent-runner/
```

Files being deleted:
- `src/lib.rs`, `src/rag.rs`, `src/semantic.rs`, `src/knowledge_graph.rs`
- `src/knowledge_graph_overlay.rs`, `src/parser.rs`, `src/grammar.rs`
- `src/traversal.rs`, `src/file_tree_builder.rs`, `src/db_schema.rs`
- `src/types.rs`, `src/errors.rs`
- `migrations/*.sql` (5 files)
- `tests/*.rs` (5 files)
- `examples/*.rs` (4 files)
- `Cargo.toml`

#### 2. Update workspace Cargo.toml
**File**: `descartes/Cargo.toml`

```toml
[workspace]
members = ["core", "cli", "gui", "daemon"]  # Remove "agent-runner"
resolver = "2"
```

#### 3. Update GUI Cargo.toml
**File**: `descartes/gui/Cargo.toml`

Remove:
```toml
[features]
default = []
agent-runner = ["dep:descartes_agent_runner"]

[dependencies]
descartes_agent_runner = { package = "agent-runner", path = "../agent-runner", optional = true }
```

Replace with:
```toml
[features]
default = []

[dependencies]
# agent-runner removed - using stub types
```

#### 4. Verify GUI stubs are complete
**File**: `descartes/gui/src/knowledge_graph_panel.rs`

The file already has:
```rust
#[cfg(not(feature = "agent-runner"))]
// Stub types...
```

These stubs will now always be used. Verify they compile.

#### 5. Verify GUI file_tree_view stubs
**File**: `descartes/gui/src/file_tree_view.rs`

Same pattern - existing stubs will be used.

### Success Criteria

#### Automated Verification:
- [x] `rm -rf descartes/agent-runner/` completes
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` passes (excluding pre-existing failures in dag_editor_interaction_tests.rs)
- [ ] `cargo run --bin descartes-gui` launches without errors
- [x] No references to `agent-runner` in any `.toml` files

#### Manual Verification:
- [x] GUI opens and displays correctly (file tree shows stub data)
- [x] No error messages in GUI console (warnings about missing feature are expected stub behavior)

---

## Phase 2: Simplify IPC System

### Overview
Delete the complex `ipc.rs` and replace with a thin Tokio channel wrapper. Keep `EventBus` in daemon for GUI streaming.

### Changes Required

#### 1. Delete complex IPC
**File**: `descartes/core/src/ipc.rs` (1,049 lines)

Delete entire file. It contains:
- `MessageBus` (lines 727-923) - replaced by ZMQ
- `DeadLetterQueue` (lines 367-405) - not needed
- `BackpressureController` (lines 408-477) - ZMQ has HWM
- `MessageRouter` (lines 480-599) - over-engineered
- `UnixSocketTransport` / `MemoryTransport` - ZMQ replaces

#### 2. Create minimal channel bridge
**File**: `descartes/core/src/channel_bridge.rs` (NEW)

```rust
//! Minimal Tokio channel bridge for internal communication
//! ZMQ handles all agent communication; this is just for in-process coordination

use tokio::sync::{mpsc, broadcast};

/// Simple message for internal coordination
#[derive(Debug, Clone)]
pub struct InternalMessage {
    pub msg_type: String,
    pub payload: serde_json::Value,
}

/// Bridge between ZMQ and internal components
pub struct ChannelBridge {
    /// Sender for internal messages
    tx: mpsc::UnboundedSender<InternalMessage>,
    /// Receiver for internal messages
    rx: mpsc::UnboundedReceiver<InternalMessage>,
}

impl ChannelBridge {
    pub fn new() -> (mpsc::UnboundedSender<InternalMessage>, mpsc::UnboundedReceiver<InternalMessage>) {
        mpsc::unbounded_channel()
    }
}

impl Default for ChannelBridge {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}
```

#### 3. Update lib.rs exports
**File**: `descartes/core/src/lib.rs`

Remove:
```rust
pub mod ipc;
pub use ipc::*;
```

Add:
```rust
pub mod channel_bridge;
pub use channel_bridge::*;
```

#### 4. Update any ipc.rs consumers
Search and update any files that import from `ipc`:
```bash
grep -r "use.*ipc" --include="*.rs" core/ daemon/ cli/ gui/
```

Replace with either:
- `channel_bridge` for simple internal messaging
- `zmq_*` modules for agent communication
- `events` module for GUI streaming

#### 5. Delete IPC benchmarks
**Files to delete**:
- `descartes/core/benches/ipc_latency.rs`
- `descartes/core/benches/ipc_throughput.rs`

Update `core/Cargo.toml` to remove these bench targets.

### Success Criteria

#### Automated Verification:
- [x] `rm descartes/core/src/ipc.rs` completes
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` passes (excluding pre-existing env-dependent test)
- [x] No compiler errors about missing `ipc` module
- [x] `grep -r "mod ipc\|use.*ipc" --include="*.rs"` returns no results

#### Manual Verification:
- [ ] Daemon starts and accepts ZMQ connections
- [ ] Events flow to GUI via WebSocket

---

## Phase 3: Simplify Database Schema

### Overview
Create a new migration that establishes the simplified 3-table event sourcing schema. Keep secrets and leases tables untouched.

### Changes Required

#### 1. Create new migration
**File**: `descartes/core/migrations/100_zmq_backbone_simplify.sql` (NEW)

```sql
-- ZMQ Backbone Simplification Migration
-- Drops RAG/semantic tables, establishes event sourcing model
-- Does NOT touch: secrets_*, leases_*, lease_*

-- Drop semantic analysis tables (if they exist from agent-runner)
DROP TABLE IF EXISTS semantic_nodes;
DROP TABLE IF EXISTS semantic_node_parameters;
DROP TABLE IF EXISTS semantic_node_type_parameters;
DROP TABLE IF EXISTS file_dependencies;
DROP TABLE IF EXISTS semantic_relationships;
DROP TABLE IF EXISTS node_call_graph;
DROP TABLE IF EXISTS ast_parsing_sessions;
DROP TABLE IF EXISTS file_metadata;
DROP TABLE IF EXISTS circular_dependencies;
DROP TABLE IF EXISTS semantic_search_cache;
DROP TABLE IF EXISTS code_change_tracking;
DROP TABLE IF EXISTS rag_metadata;
DROP TABLE IF EXISTS semantic_index_stats;

-- Drop RAG layer tables
DROP TABLE IF EXISTS rag_store_state;
DROP TABLE IF EXISTS vector_metadata;
DROP TABLE IF EXISTS fts_index_metadata;
DROP TABLE IF EXISTS sqlite_index_metadata;
DROP TABLE IF EXISTS hybrid_search_config;
DROP TABLE IF EXISTS search_results_log;
DROP TABLE IF EXISTS document_chunks;
DROP TABLE IF EXISTS rag_context_windows;
DROP TABLE IF EXISTS embedding_cache;
DROP TABLE IF EXISTS relevance_feedback;
DROP TABLE IF EXISTS sync_audit_trail;
DROP TABLE IF EXISTS consistency_checks;
DROP TABLE IF EXISTS rag_performance_stats;

-- Drop old system tables we're replacing
DROP TABLE IF EXISTS summary_statistics;
DROP TABLE IF EXISTS query_statistics;

-- Core event sourcing tables (simplified)
-- These may already exist, so use IF NOT EXISTS

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'idle',
    zmq_address TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    metadata TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    agent_id TEXT,
    session_id TEXT,
    content TEXT NOT NULL,
    metadata TEXT DEFAULT '{}',
    git_commit TEXT,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT,
    git_commit_hash TEXT NOT NULL,
    last_event_id INTEGER NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    metadata TEXT DEFAULT '{}',
    FOREIGN KEY (agent_id) REFERENCES agents(id),
    FOREIGN KEY (last_event_id) REFERENCES events(id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_events_agent_id ON events(agent_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_git ON events(git_commit);
CREATE INDEX IF NOT EXISTS idx_snapshots_agent ON snapshots(agent_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_git ON snapshots(git_commit_hash);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
```

#### 2. Update state_store.rs to use new schema
**File**: `descartes/core/src/state_store.rs`

Simplify the `apply_migrations()` function to:
1. Keep existing core tables
2. Apply the new simplification migration
3. Remove references to deleted tables

Key changes:
- Remove any queries to semantic/RAG tables
- Simplify event storage to use new `events` table
- Update snapshot queries to use new `snapshots` table

#### 3. Update agent_history.rs
**File**: `descartes/core/src/agent_history.rs`

Ensure it uses the simplified `events` table structure. The existing `AgentHistoryEvent` struct maps well to our new schema.

### Success Criteria

#### Automated Verification:
- [x] Migration file created at correct path
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` passes (excluding pre-existing env-dependent test)
- [ ] Fresh database creation works: `rm data/*.db && cargo run --bin descartes-daemon`
- [ ] Database has exactly these core tables: `agents`, `events`, `snapshots`
- [ ] Database still has: `secrets_*`, `leases`, `lease_*` tables

#### Manual Verification:
- [ ] Can spawn an agent and see it in `agents` table
- [ ] Events are logged to `events` table
- [ ] Time travel (rewind) still works with snapshots

---

## Phase 4: Add ZMQ Benchmarks

### Overview
Replace IPC benchmarks with ZMQ-focused benchmarks to verify performance.

### Changes Required

#### 1. Create ZMQ benchmark file
**File**: `descartes/core/benches/zmq_benchmarks.rs` (NEW)

```rust
//! ZMQ Performance Benchmarks
//!
//! Measures latency and throughput for ZMQ-based agent communication

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use descartes_core::zmq_agent_runner::{
    ZmqMessage, SpawnRequest, HealthCheckRequest,
    serialize_zmq_message, deserialize_zmq_message,
};
use descartes_core::traits::AgentConfig;
use std::collections::HashMap;
use uuid::Uuid;

/// Benchmark MessagePack serialization performance
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("zmq_serialization");

    // Small message (health check)
    let health_msg = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
        request_id: Uuid::new_v4().to_string(),
    });

    group.bench_function("health_check_serialize", |b| {
        b.iter(|| serialize_zmq_message(black_box(&health_msg)))
    });

    // Medium message (spawn request)
    let spawn_msg = ZmqMessage::SpawnRequest(SpawnRequest {
        request_id: Uuid::new_v4().to_string(),
        config: AgentConfig {
            name: "test-agent".to_string(),
            model_backend: "claude".to_string(),
            task: "Process data".to_string(),
            context: Some("Test context with some content".to_string()),
            system_prompt: None,
            environment: HashMap::new(),
        },
        timeout_secs: Some(30),
        metadata: Some(serde_json::json!({"key": "value"})),
    });

    group.bench_function("spawn_request_serialize", |b| {
        b.iter(|| serialize_zmq_message(black_box(&spawn_msg)))
    });

    // Deserialize benchmarks
    let health_bytes = serialize_zmq_message(&health_msg).unwrap();
    let spawn_bytes = serialize_zmq_message(&spawn_msg).unwrap();

    group.bench_function("health_check_deserialize", |b| {
        b.iter(|| deserialize_zmq_message(black_box(&health_bytes)))
    });

    group.bench_function("spawn_request_deserialize", |b| {
        b.iter(|| deserialize_zmq_message(black_box(&spawn_bytes)))
    });

    group.finish();
}

/// Benchmark round-trip serialization
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("zmq_roundtrip");

    let msg = ZmqMessage::SpawnRequest(SpawnRequest {
        request_id: Uuid::new_v4().to_string(),
        config: AgentConfig {
            name: "test-agent".to_string(),
            model_backend: "claude".to_string(),
            task: "Process data".to_string(),
            context: Some("Test context".to_string()),
            system_prompt: None,
            environment: HashMap::new(),
        },
        timeout_secs: Some(30),
        metadata: None,
    });

    group.bench_function("serialize_deserialize_roundtrip", |b| {
        b.iter(|| {
            let bytes = serialize_zmq_message(black_box(&msg)).unwrap();
            deserialize_zmq_message(black_box(&bytes)).unwrap()
        })
    });

    group.finish();
}

/// Benchmark message size validation
fn bench_validation(c: &mut Criterion) {
    use descartes_core::zmq_agent_runner::validate_message_size;

    let mut group = c.benchmark_group("zmq_validation");

    // Various message sizes
    for size in [100, 1000, 10000, 100000].iter() {
        let data = vec![0u8; *size];
        group.bench_with_input(BenchmarkId::new("validate_size", size), &data, |b, data| {
            b.iter(|| validate_message_size(black_box(data)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_serialization, bench_roundtrip, bench_validation);
criterion_main!(benches);
```

#### 2. Update core/Cargo.toml
**File**: `descartes/core/Cargo.toml`

Add bench target:
```toml
[[bench]]
name = "zmq_benchmarks"
harness = false
```

Remove old IPC bench targets:
```toml
# DELETE these:
# [[bench]]
# name = "ipc_latency"
# harness = false
#
# [[bench]]
# name = "ipc_throughput"
# harness = false
```

#### 3. Update benches/main.rs
**File**: `descartes/core/benches/main.rs`

Remove IPC benchmark imports, keep other benchmarks.

### Success Criteria

#### Automated Verification:
- [ ] `cargo bench --bench zmq_benchmarks` runs successfully
- [ ] Benchmarks complete in reasonable time (<60s total)
- [ ] No old IPC benchmarks exist
- [ ] Serialization latency <1ms for typical messages
- [ ] Roundtrip latency <2ms for typical messages

#### Manual Verification:
- [ ] Review benchmark output for reasonable performance
- [ ] Compare with expected ZMQ overhead (~microseconds)

---

## Phase 5: Cleanup and Documentation

### Overview
Final cleanup, update documentation, verify everything works end-to-end.

### Changes Required

#### 1. Update root README
**File**: `descartes/README.md`

Update architecture section to reflect:
- 4 crates (core, cli, daemon, gui)
- ZMQ-first agent communication
- Event sourcing database model
- No RAG/semantic analysis

#### 2. Delete obsolete documentation
**Files to delete**:
- Any README files in `agent-runner/` (already deleted with crate)
- `core/benches/ipc_*.md` if they exist

#### 3. Update .gitignore if needed
Ensure no orphaned entries for deleted files.

#### 4. Run full test suite
```bash
cargo test --workspace
cargo clippy --workspace
cargo fmt --all -- --check
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build --workspace --release` succeeds
- [ ] `cargo test --workspace` all pass
- [ ] `cargo clippy --workspace` no warnings
- [ ] `cargo fmt --all -- --check` passes
- [ ] Binary size reduced (check with `ls -lh target/release/descartes*`)

#### Manual Verification:
- [ ] Full workflow test: spawn agent, execute task, view in GUI
- [ ] Time travel works: can rewind to previous state
- [ ] ZMQ remote spawn works (if testing distributed)
- [ ] Documentation reflects current architecture

---

## Testing Strategy

### Unit Tests
- ZMQ message serialization/deserialization
- Channel bridge message passing
- State store CRUD operations with new schema
- Snapshot creation and retrieval

### Integration Tests
- Daemon accepts ZMQ connections
- Events flow to SQLite database
- WebSocket streams events to GUI
- Time travel (rewind/restore) works

### Manual Testing Steps
1. Start daemon: `cargo run --bin descartes-daemon`
2. Start GUI: `cargo run --bin descartes-gui`
3. Verify connection indicator shows "Connected"
4. Trigger an agent spawn (if CLI available)
5. Verify events appear in GUI
6. Test time travel slider (if implemented)

---

## Performance Considerations

### Expected Improvements
- **Compile time**: ~70% faster (no lancedb, tantivy, tree-sitter)
- **Binary size**: ~50% smaller
- **Memory usage**: Significantly lower (no vector DB, no AST cache)
- **Startup time**: Faster (fewer dependencies to initialize)

### Metrics to Track
```bash
# Before refactor
time cargo build --release
ls -lh target/release/descartes-daemon

# After refactor
time cargo build --release
ls -lh target/release/descartes-daemon
```

---

## Migration Notes

This is **greenfield** - no data migration needed. Fresh databases will be created with the new schema.

If you ever need to migrate from old schema:
1. Export events from old `agent_history_events` table
2. Import into new `events` table
3. Recreate snapshots linking to new event IDs

---

## References

- Original PRD: `.scud/docs/prd/backbone.md`
- Research document: `thoughts/shared/research/2025-12-10-zmq-backbone-prd-context.md`
- ZMQ implementation: `descartes/core/src/zmq_*.rs`
- EventBus: `descartes/daemon/src/events.rs`
