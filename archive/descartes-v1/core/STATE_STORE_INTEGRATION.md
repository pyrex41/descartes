# StateStore SQLite Backend Integration Guide

## Overview

The `SqliteStateStore` is a complete implementation of the `StateStore` trait that provides persistent agent state management with full transaction support, state snapshots, and state transition history tracking.

## Architecture

### Components

1. **SqliteStateStore** - Main implementation class
   - Connection pooling with async SQLite
   - Schema migrations
   - Transaction support
   - Key prefixing for multi-tenant scenarios

2. **AgentState** - State persistence structure
   - Agent identity and metadata
   - Serialized state data
   - Version tracking
   - Timestamps

3. **StateTransition** - History tracking
   - State before/after
   - Transition reason
   - Timestamp
   - Optional metadata

4. **Migrations** - Schema versioning
   - Automatic migration application
   - Track applied migrations
   - Support for future rollbacks

## Database Schema

### Tables Created

#### `agent_states`
```sql
CREATE TABLE agent_states (
    key TEXT PRIMARY KEY,              -- Full key with optional prefix
    agent_id TEXT NOT NULL UNIQUE,     -- Unique agent identifier
    name TEXT NOT NULL,                -- Agent display name
    status TEXT NOT NULL,              -- Current status (idle, running, paused, etc.)
    metadata TEXT NOT NULL,            -- JSON metadata
    state_data TEXT NOT NULL,          -- Serialized state
    version INTEGER NOT NULL,          -- Version for evolution
    created_at INTEGER NOT NULL,       -- Creation timestamp
    updated_at INTEGER NOT NULL,       -- Last update timestamp
    is_deleted INTEGER NOT NULL        -- Soft delete flag
);
```

#### `state_transitions`
```sql
CREATE TABLE state_transitions (
    id TEXT PRIMARY KEY,               -- Unique transition ID
    agent_id TEXT NOT NULL,            -- Associated agent
    state_before TEXT NOT NULL,        -- Previous state
    state_after TEXT NOT NULL,         -- New state
    reason TEXT,                       -- Reason for change
    timestamp INTEGER NOT NULL,        -- When it happened
    metadata TEXT                      -- Additional info
);
```

#### `state_snapshots`
```sql
CREATE TABLE state_snapshots (
    id TEXT PRIMARY KEY,               -- Snapshot ID
    agent_id TEXT NOT NULL,            -- Associated agent
    state_data TEXT NOT NULL,          -- Snapshotted state
    description TEXT,                  -- Optional description
    created_at INTEGER NOT NULL,       -- Creation time
    expires_at INTEGER                 -- Optional expiration
);
```

#### `events`
```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,               -- Event ID
    event_type TEXT NOT NULL,          -- Type of event
    timestamp INTEGER NOT NULL,        -- When it occurred
    session_id TEXT NOT NULL,          -- Session identifier
    actor_type TEXT NOT NULL,          -- Who caused it
    actor_id TEXT NOT NULL,            -- Which actor
    content TEXT NOT NULL,             -- Event content
    metadata TEXT,                     -- Optional metadata
    git_commit TEXT,                   -- Git commit reference
    created_at INTEGER NOT NULL        -- Creation timestamp
);
```

#### `tasks`
```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,               -- Task ID
    title TEXT NOT NULL,               -- Task title
    description TEXT,                  -- Optional description
    status TEXT NOT NULL,              -- Current status
    assigned_to TEXT,                  -- Assigned agent
    created_at INTEGER NOT NULL,       -- Creation time
    updated_at INTEGER NOT NULL,       -- Last update
    metadata TEXT                      -- Optional metadata
);
```

#### `sessions`
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,               -- Session ID
    agent_id TEXT NOT NULL,            -- Associated agent
    started_at INTEGER NOT NULL,       -- Session start
    ended_at INTEGER,                  -- Session end
    status TEXT NOT NULL,              -- Current status
    metadata TEXT,                     -- Optional metadata
    created_at INTEGER NOT NULL        -- Creation timestamp
);
```

### Indexes

The schema includes comprehensive indexes for:
- Agent ID lookups
- Status filtering
- Timestamp-based queries
- Combined queries (agent + status, agent + timestamp)
- Full-text search support on event content

## Usage Examples

### Initialize State Store

```rust
use descartes_core::SqliteStateStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create state store with SQLite backend
    let mut store = SqliteStateStore::new(
        "data/agent_state.db",
        false  // disable compression for now
    ).await?;

    // Initialize schema and run migrations
    store.initialize().await?;

    Ok(())
}
```

### Save Agent State

```rust
use descartes_core::{SqliteStateStore, AgentState};
use serde_json::json;
use chrono::Utc;

let store = SqliteStateStore::new("data/agent_state.db", false).await?;

let agent_state = AgentState {
    agent_id: "agent_001".to_string(),
    name: "MyWorkerAgent".to_string(),
    status: "running".to_string(),
    metadata: json!({
        "type": "worker",
        "capacity": 100,
        "tags": ["production"]
    }),
    state_data: json!({
        "tasks_processed": 42,
        "current_task": "task_123",
        "memory_usage": "256MB"
    }).to_string(),
    created_at: Utc::now().timestamp(),
    updated_at: Utc::now().timestamp(),
    version: 1,
};

store.save_agent_state(&agent_state).await?;
```

### Load Agent State

```rust
if let Some(agent_state) = store.load_agent_state("agent_001").await? {
    println!("Agent: {} ({})", agent_state.name, agent_state.status);
    println!("State data: {}", agent_state.state_data);
}
```

### List All Agents

```rust
let agents = store.list_agents().await?;
for agent in agents {
    println!("{}: {}", agent.agent_id, agent.status);
}
```

### Update Agent Status

```rust
store.update_agent_status("agent_001", "paused").await?;
```

### Record State Transitions

```rust
store.record_state_transition(
    "agent_001",
    r#"{"status":"idle","iteration":10}"#,
    r#"{"status":"running","iteration":11}"#,
    Some("User initiated execution".to_string()),
).await?;

// Retrieve transition history
let history = store.get_state_history("agent_001", 50).await?;
for transition in history {
    println!("At {}: {} (reason: {})",
        transition.timestamp,
        transition.state_after,
        transition.reason.unwrap_or_default()
    );
}
```

### Create State Snapshots

```rust
// Create a checkpoint
let snapshot_id = store.create_snapshot(
    "agent_001",
    Some("Stable state after iteration 100".to_string())
).await?;

// List snapshots
let snapshots = store.list_snapshots("agent_001").await?;
for (id, desc, created_at) in snapshots {
    println!("Snapshot: {} - {}", id, desc.unwrap_or_default());
}

// Restore from snapshot
store.restore_snapshot(&snapshot_id).await?;
```

### Event Management

```rust
use descartes_core::{Event, ActorType};
use uuid::Uuid;

let event = Event {
    id: Uuid::new_v4(),
    event_type: "agent_completed_task".to_string(),
    timestamp: Utc::now().timestamp(),
    session_id: "session_001".to_string(),
    actor_type: ActorType::Agent,
    actor_id: "agent_001".to_string(),
    content: "Task completed successfully".to_string(),
    metadata: Some(json!({ "task_id": "task_123" })),
    git_commit: None,
};

store.save_event(&event).await?;

// Query events
let events = store.get_events("session_001").await?;
let by_type = store.get_events_by_type("agent_completed_task").await?;
let search_results = store.search_events("task").await?;
```

### Task Management

```rust
use descartes_core::{Task, TaskStatus};

let task = Task {
    id: Uuid::new_v4(),
    title: "Process batch".to_string(),
    description: Some("Process incoming data batch".to_string()),
    status: TaskStatus::InProgress,
    assigned_to: Some("agent_001".to_string()),
    created_at: Utc::now().timestamp(),
    updated_at: Utc::now().timestamp(),
    metadata: Some(json!({ "batch_size": 1000 })),
};

store.save_task(&task).await?;

if let Some(task) = store.get_task(&task.id).await? {
    println!("Task: {}", task.title);
}

let all_tasks = store.get_tasks().await?;
```

## Advanced Features

### Key Prefixing

Use key prefixes to namespace agent states by type or environment:

```rust
let worker_store = SqliteStateStore::new("data/state.db", false)
    .await?
    .with_prefix("worker".to_string());

let coordinator_store = SqliteStateStore::new("data/state.db", false)
    .await?
    .with_prefix("coordinator".to_string());

// Keys are stored as "worker:agent_001" and "coordinator:agent_001"
```

### Concurrent Access

The connection pool supports concurrent access:

```rust
use std::sync::Arc;

let store = Arc::new(SqliteStateStore::new("data/state.db", false).await?);

// Use in multiple tasks
let store_clone = Arc::clone(&store);
tokio::spawn(async move {
    let agents = store_clone.list_agents().await.unwrap();
});
```

### Transaction Pattern

```rust
let result = store.transact(|| async {
    // Execute multiple operations
    // Automatic rollback on error
    Ok("Transaction completed")
}).await?;
```

### Connection Pool Access

For advanced operations, access the underlying pool:

```rust
let pool = store.pool();
// Use sqlx directly for complex queries
```

## Performance Considerations

### Connection Pooling
- Default pool size: 10 connections
- Min connections: 1
- Acquire timeout: 30 seconds
- Configurable via `SqlitePoolOptions`

### Indexes
- Automatic indexes on foreign keys
- Composite indexes for common query patterns
- Partial indexes for filtered queries (e.g., non-deleted records)

### Compression
- Optional state data compression
- Currently disabled by default
- Enable with: `SqliteStateStore::new(..., true)`

### Batch Operations
For bulk inserts, consider batching:

```rust
for agent_state in states {
    store.save_agent_state(&agent_state).await?;
}
// Each operation auto-commits; wrap in transaction for atomicity
```

## Migration Strategy

### Automatic Migrations
Migrations are applied automatically on `initialize()`:

1. v1: Create agent_states table
2. v2: Create state_transitions table
3. v3: Create state_snapshots table
4. v4: Add performance indexes

### Adding New Migrations

Edit `apply_migrations()` in `state_store.rs`:

```rust
let migrations = vec![
    // Existing migrations...
    (5, "add_new_feature", "Add column X to table Y",
        r#"ALTER TABLE agent_states ADD COLUMN new_field TEXT;"#),
];
```

### Checking Migration Status

```rust
let migrations = store.get_migration_history().await?;
for m in migrations {
    println!("Applied: v{} - {}", m.version, m.name);
}
```

## Error Handling

The state store uses `StateStoreError`:

```rust
use descartes_core::StateStoreError;

match store.load_agent_state("agent_001").await {
    Ok(Some(state)) => println!("Found: {:?}", state),
    Ok(None) => println!("Not found"),
    Err(StateStoreError::DatabaseError(e)) => eprintln!("DB error: {}", e),
    Err(StateStoreError::MigrationError(e)) => eprintln!("Migration error: {}", e),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

The implementation includes unit tests:

```bash
# Run tests
cargo test --lib state_store

# With logging
RUST_LOG=debug cargo test --lib state_store -- --nocapture
```

## Integration with Agent Runner

```rust
use descartes_core::{SqliteStateStore, LocalProcessRunner};

let state_store = SqliteStateStore::new("data/state.db", false).await?;
state_store.initialize().await?;

// Use with agent runner
let runner = LocalProcessRunner::new(Default::default())?;

// After spawning agents, save their state
let agents = runner.list_agents().await?;
for agent_info in agents {
    let state = AgentState {
        agent_id: agent_info.id.to_string(),
        name: agent_info.name.clone(),
        status: format!("{:?}", agent_info.status),
        metadata: serde_json::json!({
            "started_at": agent_info.started_at,
            "backend": agent_info.model_backend,
        }),
        state_data: "{}".to_string(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        version: 1,
    };

    state_store.save_agent_state(&state).await?;
}
```

## Best Practices

1. **Initialize Early**: Call `store.initialize()` once at startup
2. **Use Prefixes**: Namespace states by agent type or environment
3. **Record Transitions**: Log important state changes for debugging
4. **Create Snapshots**: Checkpoint stable states for recovery
5. **Clean Up**: Periodically delete old snapshots and transitions
6. **Monitor**: Check migration status on startup
7. **Error Handling**: Use proper error types and propagate with context

## Troubleshooting

### Database Lock Errors
- Increase pool size if many concurrent operations
- Ensure WAL mode is enabled (set by default)
- Check for long-running transactions

### Schema Issues
- Run migrations: `store.initialize().await?`
- Check migration history: `get_migration_history()`
- Review table structure with sqlite3 CLI

### Performance
- Check indexes are created: `store.get_migration_history()`
- Monitor pool statistics
- Use profiling tools to identify bottlenecks

## Future Enhancements

- [ ] Full-text search support (FTS5)
- [ ] Automatic snapshot rotation/cleanup
- [ ] State data compression (snappy/zstd)
- [ ] Backup/restore utilities
- [ ] Replication support
- [ ] State diff/merge operations
- [ ] Metrics and observability hooks
