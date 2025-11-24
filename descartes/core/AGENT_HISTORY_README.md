# Agent History - Event Sourcing and History Tracking

## Overview

The Agent History module provides comprehensive tracking of agent behavior through event sourcing, combining:

- **Brain State**: Event logs tracking thoughts, actions, tool usage, and state changes
- **Body State**: Git commit references tracking code changes and artifacts

This enables time-travel debugging, audit trails, performance analysis, and recovery/restoration capabilities.

## File Locations

### Core Implementation
- **Module**: `/home/user/descartes/descartes/core/src/agent_history.rs`
- **Migration**: `/home/user/descartes/descartes/core/migrations/005_create_agent_history.sql`
- **Exports**: `/home/user/descartes/descartes/core/src/lib.rs` (lines 4, 143-146)

## Architecture

### Data Models

#### 1. AgentHistoryEvent (Brain State)

Represents a discrete moment in agent execution:

```rust
pub struct AgentHistoryEvent {
    pub event_id: Uuid,
    pub agent_id: String,
    pub timestamp: i64,
    pub event_type: HistoryEventType,
    pub event_data: Value,
    pub git_commit_hash: Option<String>,
    pub session_id: Option<String>,
    pub parent_event_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub metadata: Option<Value>,
}
```

**Key Features**:
- Unique event identification
- Flexible JSON event data
- Git commit linking (body state)
- Session grouping
- Parent-child causality tracking
- Tag-based categorization

#### 2. HistoryEventType

Categorizes different types of agent events:

```rust
pub enum HistoryEventType {
    Thought,        // Cognitive events - agent's internal reasoning
    Action,         // Action events - agent performing operations
    ToolUse,        // Tool usage events - agent using external tools
    StateChange,    // State transitions - changes in agent state machine
    Communication,  // Communication events - messages sent/received
    Decision,       // Decision events - choices made by the agent
    Error,          // Error events - failures and exceptions
    System,         // System events - lifecycle and metadata changes
}
```

#### 3. HistorySnapshot (Combined Brain + Body State)

Point-in-time capture of complete agent state:

```rust
pub struct HistorySnapshot {
    pub snapshot_id: Uuid,
    pub agent_id: String,
    pub timestamp: i64,
    pub events: Vec<AgentHistoryEvent>,
    pub git_commit: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<Value>,
    pub agent_state: Option<Value>,
}
```

**Purpose**:
- Recovery and restoration
- Performance analysis
- State comparison
- Debugging checkpoints

#### 4. HistoryQuery

Flexible query parameters for retrieving events:

```rust
pub struct HistoryQuery {
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub event_type: Option<HistoryEventType>,
    pub tags: Vec<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub ascending: bool,
}
```

#### 5. HistoryStatistics

Aggregate statistics about agent history:

```rust
pub struct HistoryStatistics {
    pub total_events: i64,
    pub events_by_type: HashMap<String, i64>,
    pub total_snapshots: i64,
    pub earliest_event: Option<i64>,
    pub latest_event: Option<i64>,
    pub unique_sessions: i64,
    pub git_commits: Vec<String>,
}
```

## Storage Implementation

### AgentHistoryStore Trait

The core trait for history storage operations:

```rust
#[async_trait]
pub trait AgentHistoryStore: Send + Sync {
    // Initialization
    async fn initialize(&mut self) -> StateStoreResult<()>;

    // Event Recording
    async fn record_event(&self, event: &AgentHistoryEvent) -> StateStoreResult<()>;
    async fn record_events(&self, events: &[AgentHistoryEvent]) -> StateStoreResult<()>;

    // Event Retrieval
    async fn get_events(&self, agent_id: &str, limit: i64) -> StateStoreResult<Vec<AgentHistoryEvent>>;
    async fn query_events(&self, query: &HistoryQuery) -> StateStoreResult<Vec<AgentHistoryEvent>>;
    async fn get_events_by_type(&self, agent_id: &str, event_type: HistoryEventType, limit: i64) -> StateStoreResult<Vec<AgentHistoryEvent>>;
    async fn get_events_by_time_range(&self, agent_id: &str, start_time: i64, end_time: i64) -> StateStoreResult<Vec<AgentHistoryEvent>>;
    async fn get_events_by_session(&self, session_id: &str) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    // Snapshot Management
    async fn create_snapshot(&self, snapshot: &HistorySnapshot) -> StateStoreResult<()>;
    async fn get_snapshot(&self, snapshot_id: &Uuid) -> StateStoreResult<Option<HistorySnapshot>>;
    async fn list_snapshots(&self, agent_id: &str) -> StateStoreResult<Vec<HistorySnapshot>>;

    // Maintenance
    async fn delete_events_before(&self, timestamp: i64) -> StateStoreResult<i64>;

    // Analytics
    async fn get_statistics(&self, agent_id: &str) -> StateStoreResult<HistoryStatistics>;
    async fn get_event_chain(&self, event_id: &Uuid) -> StateStoreResult<Vec<AgentHistoryEvent>>;
}
```

### SqliteAgentHistoryStore

SQLite-backed implementation with optimized queries and indexes.

## Database Schema

### Tables

#### agent_history_events
Stores individual events in agent history:

```sql
CREATE TABLE agent_history_events (
    event_id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    git_commit_hash TEXT,
    session_id TEXT,
    parent_event_id TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    metadata TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (parent_event_id) REFERENCES agent_history_events(event_id) ON DELETE SET NULL
);
```

**Indexes**:
- `idx_history_agent_id` - Filter by agent
- `idx_history_timestamp` - Time-based queries
- `idx_history_event_type` - Type-based filtering
- `idx_history_session_id` - Session grouping
- `idx_history_git_commit` - Body state lookups
- `idx_history_agent_timestamp` - Combined agent+time queries
- `idx_history_agent_type` - Combined agent+type queries
- `idx_history_parent` - Event chain traversal

#### history_snapshots
Stores point-in-time snapshots:

```sql
CREATE TABLE history_snapshots (
    snapshot_id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    git_commit TEXT,
    description TEXT,
    metadata TEXT,
    agent_state TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
```

#### snapshot_events
Junction table linking snapshots to events:

```sql
CREATE TABLE snapshot_events (
    snapshot_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, event_id),
    FOREIGN KEY (snapshot_id) REFERENCES history_snapshots(snapshot_id) ON DELETE CASCADE,
    FOREIGN KEY (event_id) REFERENCES agent_history_events(event_id) ON DELETE CASCADE
);
```

### Views

#### v_recent_agent_activity
Shows recent activity by agent and type:

```sql
CREATE VIEW v_recent_agent_activity AS
SELECT agent_id, event_type, COUNT(*) as event_count,
       MAX(timestamp) as last_event_time,
       MIN(timestamp) as first_event_time
FROM agent_history_events
WHERE timestamp > strftime('%s', 'now') - 86400
GROUP BY agent_id, event_type;
```

#### v_agent_event_summary
Aggregate statistics per agent:

```sql
CREATE VIEW v_agent_event_summary AS
SELECT agent_id,
       COUNT(*) as total_events,
       COUNT(DISTINCT session_id) as unique_sessions,
       COUNT(DISTINCT git_commit_hash) as unique_commits,
       MIN(timestamp) as earliest_event,
       MAX(timestamp) as latest_event
FROM agent_history_events
GROUP BY agent_id;
```

#### v_event_chains
Parent-child event relationships:

```sql
CREATE VIEW v_event_chains AS
SELECT e1.event_id as child_event_id,
       e1.event_type as child_event_type,
       e2.event_id as parent_event_id,
       e2.event_type as parent_event_type,
       e1.agent_id
FROM agent_history_events e1
LEFT JOIN agent_history_events e2 ON e1.parent_event_id = e2.event_id
WHERE e1.parent_event_id IS NOT NULL;
```

## Usage Examples

### 1. Recording Events

#### Simple Event
```rust
use descartes_core::{
    SqliteAgentHistoryStore, AgentHistoryEvent, HistoryEventType, AgentHistoryStore
};
use serde_json::json;

// Create store
let mut store = SqliteAgentHistoryStore::new("./agent_history.db").await?;
store.initialize().await?;

// Record a thought event
let event = AgentHistoryEvent::new(
    "agent-123".to_string(),
    HistoryEventType::Thought,
    json!({
        "content": "Analyzing the user's request",
        "confidence": 0.85
    })
);

store.record_event(&event).await?;
```

#### Event with Git Commit
```rust
let event = AgentHistoryEvent::new(
    "agent-123".to_string(),
    HistoryEventType::Action,
    json!({
        "action": "write_file",
        "file": "src/main.rs",
        "lines_added": 42
    })
)
.with_git_commit("abc123def456".to_string());

store.record_event(&event).await?;
```

#### Event with Causality
```rust
// Parent event
let thought = AgentHistoryEvent::new(
    "agent-123".to_string(),
    HistoryEventType::Thought,
    json!({"content": "I should read the config file"})
);
store.record_event(&thought).await?;

// Child event with parent reference
let action = AgentHistoryEvent::new(
    "agent-123".to_string(),
    HistoryEventType::Action,
    json!({"action": "read_file", "file": "config.toml"})
)
.with_parent(thought.event_id);

store.record_event(&action).await?;
```

#### Batch Recording
```rust
let events = vec![
    AgentHistoryEvent::new(
        "agent-123".to_string(),
        HistoryEventType::Thought,
        json!({"content": "Planning the task"})
    ),
    AgentHistoryEvent::new(
        "agent-123".to_string(),
        HistoryEventType::Action,
        json!({"action": "execute_plan"})
    ),
];

store.record_events(&events).await?;
```

### 2. Querying Events

#### Get Recent Events
```rust
let recent_events = store.get_events("agent-123", 100).await?;

for event in recent_events {
    println!("{}: {} - {:?}",
        event.timestamp,
        event.event_type,
        event.event_data
    );
}
```

#### Query by Type
```rust
let thoughts = store.get_events_by_type(
    "agent-123",
    HistoryEventType::Thought,
    50
).await?;
```

#### Query by Time Range
```rust
use chrono::Utc;

let now = Utc::now().timestamp();
let one_hour_ago = now - 3600;

let events = store.get_events_by_time_range(
    "agent-123",
    one_hour_ago,
    now
).await?;
```

#### Advanced Query
```rust
use descartes_core::HistoryQuery;

let query = HistoryQuery {
    agent_id: Some("agent-123".to_string()),
    event_type: Some(HistoryEventType::Action),
    start_time: Some(one_hour_ago),
    end_time: Some(now),
    limit: Some(100),
    ascending: true,
    ..Default::default()
};

let events = store.query_events(&query).await?;
```

#### Query by Session
```rust
let session_events = store.get_events_by_session("session-456").await?;
```

### 3. Snapshots

#### Create Snapshot
```rust
use descartes_core::HistorySnapshot;

// Get recent events
let events = store.get_events("agent-123", 100).await?;

// Create snapshot
let snapshot = HistorySnapshot::new(
    "agent-123".to_string(),
    events,
    Some("abc123def456".to_string()) // git commit
)
.with_description("Checkpoint before major refactor".to_string())
.with_agent_state(json!({
    "status": "running",
    "memory_usage": 1024000,
    "cpu_usage": 45.2
}));

store.create_snapshot(&snapshot).await?;
```

#### Retrieve Snapshot
```rust
let snapshot = store.get_snapshot(&snapshot_id).await?;

if let Some(snap) = snapshot {
    println!("Snapshot from {}", snap.timestamp);
    println!("Git commit: {:?}", snap.git_commit);
    println!("Events count: {}", snap.events.len());
    println!("Description: {:?}", snap.description);
}
```

#### List Snapshots
```rust
let snapshots = store.list_snapshots("agent-123").await?;

for snapshot in snapshots {
    println!("Snapshot {} at {}: {:?}",
        snapshot.snapshot_id,
        snapshot.timestamp,
        snapshot.description
    );
}
```

### 4. Analytics

#### Get Statistics
```rust
let stats = store.get_statistics("agent-123").await?;

println!("Total events: {}", stats.total_events);
println!("Total snapshots: {}", stats.total_snapshots);
println!("Unique sessions: {}", stats.unique_sessions);

for (event_type, count) in stats.events_by_type {
    println!("{}: {}", event_type, count);
}

println!("Git commits: {:?}", stats.git_commits);
```

#### Get Event Chain
```rust
// Follow parent references to get full causality chain
let chain = store.get_event_chain(&event_id).await?;

println!("Event chain:");
for (i, event) in chain.iter().enumerate() {
    println!("  {}: {} - {:?}", i, event.event_type, event.event_data);
}
```

### 5. Maintenance

#### Delete Old Events
```rust
use chrono::Utc;

// Delete events older than 90 days
let ninety_days_ago = Utc::now().timestamp() - (90 * 24 * 60 * 60);
let deleted_count = store.delete_events_before(ninety_days_ago).await?;

println!("Deleted {} old events", deleted_count);
```

## Testing

Comprehensive tests are included in `/home/user/descartes/descartes/core/src/agent_history.rs`:

```bash
# Run all agent history tests
cd /home/user/descartes/descartes/core
cargo test agent_history

# Run specific tests
cargo test test_record_and_retrieve_event
cargo test test_create_and_retrieve_snapshot
cargo test test_event_chain
```

### Test Coverage

- ✅ Event recording and retrieval
- ✅ Batch event recording
- ✅ Query by type
- ✅ Snapshot creation and retrieval
- ✅ Statistics generation
- ✅ Event chain traversal
- ✅ Time range queries

## Integration Points

### With State Machine
```rust
// Record state transitions as events
let transition = AgentHistoryEvent::new(
    agent_id.clone(),
    HistoryEventType::StateChange,
    json!({
        "from_state": "idle",
        "to_state": "running",
        "event": "start"
    })
)
.with_git_commit(current_commit)
.with_session(session_id);

history_store.record_event(&transition).await?;
```

### With Thoughts System
```rust
// Record thought creation as history event
let thought_event = AgentHistoryEvent::new(
    agent_id.clone(),
    HistoryEventType::Thought,
    json!({
        "thought_id": thought.id,
        "title": thought.title,
        "content": thought.content
    })
);

history_store.record_event(&thought_event).await?;
```

### With Tool Usage
```rust
// Record tool usage
let tool_event = AgentHistoryEvent::new(
    agent_id.clone(),
    HistoryEventType::ToolUse,
    json!({
        "tool": "grep",
        "arguments": ["pattern", "file.rs"],
        "result": "found 3 matches"
    })
)
.with_tags(vec!["search".to_string(), "code".to_string()]);

history_store.record_event(&tool_event).await?;
```

## Performance Considerations

### Indexes
The schema includes comprehensive indexes for common query patterns:
- Agent-based queries
- Time-based queries
- Type-based filtering
- Session grouping
- Git commit lookups

### Batch Operations
Use `record_events()` for batch inserts to improve performance:
```rust
// Efficient batch insert
store.record_events(&events).await?;

// Less efficient
for event in events {
    store.record_event(&event).await?;
}
```

### Query Limits
Always use limits when querying large history:
```rust
// Good: limited query
let events = store.get_events("agent-123", 1000).await?;

// Potentially slow: unlimited query with manual limit
let all_events = store.get_events("agent-123", i64::MAX).await?;
```

### Cleanup
Regularly delete old events to maintain performance:
```rust
// Run periodically (e.g., daily)
let retention_period = 90 * 24 * 60 * 60; // 90 days
let cutoff = Utc::now().timestamp() - retention_period;
store.delete_events_before(cutoff).await?;
```

## Future Enhancements

Potential improvements for future phases:

1. **Event Streaming**: Real-time event streaming for monitoring
2. **Compression**: Event data compression for long-term storage
3. **Partitioning**: Time-based partitioning for very large histories
4. **Aggregation**: Pre-computed aggregations for faster analytics
5. **Replay**: Event replay functionality for debugging
6. **Export**: Export history to various formats (JSON, Parquet, etc.)
7. **Search**: Full-text search on event data
8. **Visualization**: Timeline and graph visualizations

## See Also

- State Machine: `/home/user/descartes/descartes/core/STATE_MACHINE_GUIDE.md`
- State Store: `/home/user/descartes/descartes/core/STATE_STORE_README.md`
- Thoughts System: `/home/user/descartes/descartes/core/src/thoughts.rs`
