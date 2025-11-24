# Descartes Time Travel Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Architecture](#architecture)
4. [Getting Started](#getting-started)
5. [Basic Usage](#basic-usage)
6. [Advanced Features](#advanced-features)
7. [API Reference](#api-reference)
8. [Best Practices](#best-practices)
9. [Performance Considerations](#performance-considerations)
10. [Troubleshooting](#troubleshooting)
11. [Examples](#examples)

---

## Introduction

Time Travel in Descartes enables you to rewind an agent's execution to any previous point in history, inspect its state, and optionally resume execution from that point. This powerful feature supports:

- **Debugging**: Understand exactly what the agent was thinking at any moment
- **Recovery**: Restore from failures by rewinding to a known-good state
- **Exploration**: Try different paths by rewinding and resuming with different parameters
- **Auditing**: Inspect historical decision-making for compliance and analysis

### What is Time Travel?

Time Travel combines two key concepts:

1. **Brain Restoration**: Replaying event history to rebuild the agent's cognitive state (thoughts, decisions, memory)
2. **Body Restoration**: Checking out git commits to restore the agent's code and artifacts

Together, these provide complete state restoration capabilities.

---

## Core Concepts

### Brain vs Body

**Brain (Event Sourcing)**
- The agent's cognitive state: thoughts, decisions, actions, memories
- Stored as a sequence of immutable events in a database
- Events include: Thoughts, Decisions, Actions, Tool Use, State Changes, Errors

**Body (Git History)**
- The agent's code, configuration files, and generated artifacts
- Stored as git commits in the repository
- Each commit represents a snapshot of the filesystem at a point in time

### Event Types

```rust
pub enum HistoryEventType {
    Thought,        // Cognitive reasoning
    Action,         // Agent actions
    ToolUse,        // External tool invocations
    StateChange,    // State machine transitions
    Communication,  // Messages sent/received
    Decision,       // Decision points
    Error,          // Failures and exceptions
    System,         // Lifecycle events
}
```

### Rewind Points

A rewind point identifies a specific moment in history you can restore to:

```rust
pub struct RewindPoint {
    pub timestamp: i64,              // When the event occurred
    pub event_id: Option<Uuid>,      // Specific event identifier
    pub git_commit: Option<String>,  // Git commit at this point
    pub snapshot_id: Option<Uuid>,   // Pre-made snapshot reference
    pub description: String,         // Human-readable description
    pub event_index: Option<usize>,  // Index in event list (for UI)
}
```

### Snapshots

Snapshots are pre-computed bundles of events and git references that enable fast restoration:

```rust
pub struct HistorySnapshot {
    pub snapshot_id: Uuid,
    pub agent_id: String,
    pub timestamp: i64,
    pub events: Vec<AgentHistoryEvent>,
    pub git_commit: Option<String>,
    pub description: Option<String>,
}
```

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Time Travel System                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐        ┌──────────────┐                  │
│  │    Brain     │        │     Body     │                  │
│  │   Restore    │        │   Restore    │                  │
│  │              │        │              │                  │
│  │ ┌──────────┐ │        │ ┌──────────┐ │                  │
│  │ │  Event   │ │        │ │   Git    │ │                  │
│  │ │ Sourcing │ │        │ │ Checkout │ │                  │
│  │ └──────────┘ │        │ └──────────┘ │                  │
│  │      ↓       │        │      ↓       │                  │
│  │ ┌──────────┐ │        │ ┌──────────┐ │                  │
│  │ │  State   │ │        │ │ Working  │ │                  │
│  │ │ Rebuild  │ │        │ │   Tree   │ │                  │
│  │ └──────────┘ │        │ └──────────┘ │                  │
│  └──────────────┘        └──────────────┘                  │
│         │                       │                           │
│         └───────────┬───────────┘                           │
│                     ↓                                       │
│          ┌────────────────────┐                            │
│          │  Rewind Manager    │                            │
│          │  - Coordination    │                            │
│          │  - Validation      │                            │
│          │  - Undo/Redo       │                            │
│          └────────────────────┘                            │
│                     ↓                                       │
│          ┌────────────────────┐                            │
│          │   Time Travel UI   │                            │
│          │  - Timeline Slider │                            │
│          │  - Event Display   │                            │
│          │  - Playback        │                            │
│          └────────────────────┘                            │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Normal Operation**: Agent records events and creates git commits
2. **Rewind Request**: User selects a point in time via UI or API
3. **Backup Creation**: Current state is saved for undo capability
4. **Brain Restore**: Events are replayed to rebuild cognitive state
5. **Body Restore**: Git checkout restores filesystem state
6. **Validation**: Consistency between brain and body is verified
7. **Resume (Optional)**: Agent continues execution from restored state

---

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git installed and configured
- SQLite (included with Rust)

### Installation

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
descartes-core = { path = "../descartes/core" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Basic Setup

```rust
use descartes_core::{
    agent_history::SqliteAgentHistoryStore,
    time_travel_integration::DefaultRewindManager,
};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create history store
    let mut store = SqliteAgentHistoryStore::new("agent_history.db").await?;
    store.initialize().await?;

    // Create rewind manager
    let repo_path = PathBuf::from(".");
    let manager = DefaultRewindManager::new(
        Arc::new(store),
        repo_path,
        10, // max undo history
    )?;

    println!("Time travel system initialized!");

    Ok(())
}
```

---

## Basic Usage

### Recording Events

```rust
use descartes_core::agent_history::{AgentHistoryEvent, HistoryEventType};
use serde_json::json;

// Record a thought
let thought = AgentHistoryEvent::new(
    "agent-alpha".to_string(),
    HistoryEventType::Thought,
    json!({
        "content": "Analyzing the problem...",
        "confidence": 0.85
    }),
)
.with_session("session-001".to_string())
.with_git_commit("abc123def456...".to_string());

store.record_event(&thought).await?;

// Record a decision
let decision = AgentHistoryEvent::new(
    "agent-alpha".to_string(),
    HistoryEventType::Decision,
    json!({
        "decision_type": "action_selection",
        "options": ["plan_a", "plan_b"],
        "selected": "plan_a",
        "reasoning": "Better success probability"
    }),
)
.with_parent(thought.event_id);

store.record_event(&decision).await?;
```

### Viewing History

```rust
use descartes_core::brain_restore::BrainRestore;

// Get all events
let events = manager.get_rewind_points("agent-alpha").await?;

for point in events {
    println!("{} - {}", point.timestamp, point.description);
}
```

### Simple Rewind

```rust
use descartes_core::time_travel_integration::{RewindConfig, RewindPoint};

// Create a rewind point
let point = RewindPoint {
    timestamp: target_timestamp,
    event_id: Some(event_id),
    git_commit: Some(commit_hash),
    snapshot_id: None,
    description: "Rewind to decision point".to_string(),
    event_index: Some(5),
};

// Configure rewind options
let config = RewindConfig {
    require_confirmation: false,
    auto_backup: true,
    validate_state: true,
    allow_uncommitted_changes: true,
    max_undo_history: 10,
    enable_debugging: false,
};

// Perform rewind
let result = manager.rewind_to(point, config).await?;

if result.success {
    println!("Rewind successful!");
    println!("Events processed: {}",
        result.brain_result.unwrap().events_processed);
} else {
    println!("Rewind failed: {:?}", result.validation.errors);
}
```

### Undo Rewind

```rust
// Each rewind creates a backup with a unique ID
let backup_id = result.backup.backup_id;

// Later, undo the rewind
let undo_result = manager.undo_rewind(backup_id).await?;

if undo_result.success {
    println!("Successfully undone rewind!");
}
```

---

## Advanced Features

### Creating Snapshots

Snapshots enable instant restoration to commonly-used states:

```rust
// Create a snapshot at current state
let snapshot_id = manager.create_snapshot(
    "agent-alpha",
    "Before risky operation".to_string()
).await?;

// Later, rewind to snapshot
let point = RewindPoint::from_snapshot(&snapshot);
manager.rewind_to(point, config).await?;
```

### Resume Execution

After rewinding, you can resume agent execution:

```rust
use descartes_core::time_travel_integration::ResumeContext;

// Rewind first
let rewind_result = manager.rewind_to(point, config).await?;

// Create resume context
let resume_ctx = ResumeContext::from_rewind_result(
    &rewind_result,
    "agent-alpha".to_string()
)?;

// Resume execution (integration with agent runtime required)
manager.resume_from(resume_ctx).await?;
```

### State Validation

Ensure brain and body are consistent:

```rust
let current_commit = body_manager.get_current_commit().await?;
let brain_state = /* ... restored brain state ... */;

let validation = manager.validate_consistency(
    &brain_state,
    &current_commit
).await?;

if !validation.valid {
    println!("Validation errors:");
    for error in &validation.errors {
        println!("  - {}", error);
    }
}

if !validation.warnings.is_empty() {
    println!("Warnings:");
    for warning in &validation.warnings {
        println!("  - {}", warning);
    }
}
```

### Debugging Mode

Enable step-through debugging after rewind:

```rust
let resume_ctx = ResumeContext::from_rewind_result(&result, agent_id)?
    .with_debugging()
    .with_breakpoint(BreakpointLocation::Event(event_id))
    .with_breakpoint(BreakpointLocation::Condition("error_count > 0".to_string()));

manager.resume_from(resume_ctx).await?;
```

---

## API Reference

### RewindManager Trait

Main interface for time travel operations:

```rust
#[async_trait]
pub trait RewindManager: Send + Sync {
    /// Check if rewind is possible
    async fn can_rewind_to(&self, point: &RewindPoint)
        -> StateStoreResult<RewindConfirmation>;

    /// Rewind to a specific point
    async fn rewind_to(
        &self,
        point: RewindPoint,
        config: RewindConfig,
    ) -> StateStoreResult<RewindResult>;

    /// Resume execution from rewound state
    async fn resume_from(&self, context: ResumeContext)
        -> StateStoreResult<()>;

    /// Undo a previous rewind
    async fn undo_rewind(&self, backup_id: Uuid)
        -> StateStoreResult<RewindResult>;

    /// Get available rewind points
    async fn get_rewind_points(&self, agent_id: &str)
        -> StateStoreResult<Vec<RewindPoint>>;

    /// Validate brain-body consistency
    async fn validate_consistency(
        &self,
        brain_state: &BrainState,
        current_commit: &str,
    ) -> StateStoreResult<ValidationResult>;

    /// Create a snapshot for quick restoration
    async fn create_snapshot(&self, agent_id: &str, description: String)
        -> StateStoreResult<Uuid>;
}
```

### RewindConfig

Configuration for rewind operations:

```rust
pub struct RewindConfig {
    /// Whether to require confirmation before rewind
    pub require_confirmation: bool,

    /// Whether to create automatic backups
    pub auto_backup: bool,

    /// Whether to validate state after restore
    pub validate_state: bool,

    /// Whether to allow rewind with uncommitted changes
    pub allow_uncommitted_changes: bool,

    /// Maximum number of undo operations to keep
    pub max_undo_history: usize,

    /// Whether to enable debugging at rewound state
    pub enable_debugging: bool,
}

impl RewindConfig {
    /// Safe default configuration
    pub fn safe() -> Self;

    /// Fast configuration (minimal checks)
    pub fn fast() -> Self;
}
```

### RewindResult

Result of a rewind operation:

```rust
pub struct RewindResult {
    /// Whether the rewind was successful
    pub success: bool,

    /// The point we rewound to
    pub target_point: RewindPoint,

    /// Brain restore result
    pub brain_result: Option<BrainRestoreResult>,

    /// Body restore result
    pub body_result: Option<BodyRestoreResult>,

    /// Backup information for undo
    pub backup: RewindBackup,

    /// State validation results
    pub validation: ValidationResult,

    /// Messages and warnings
    pub messages: Vec<String>,

    /// Time taken (milliseconds)
    pub duration_ms: u64,
}
```

---

## Best Practices

### 1. Always Use Backups

Enable automatic backups for production systems:

```rust
let config = RewindConfig {
    auto_backup: true,
    ..Default::default()
};
```

### 2. Validate After Rewind

Always check validation results:

```rust
let result = manager.rewind_to(point, config).await?;

if !result.validation.valid {
    // Handle validation errors
    eprintln!("State validation failed!");
    for error in &result.validation.errors {
        eprintln!("  {}", error);
    }

    // Consider rolling back
    manager.undo_rewind(result.backup.backup_id).await?;
}
```

### 3. Create Strategic Snapshots

Create snapshots before risky operations:

```rust
// Before risky operation
let snapshot_id = manager.create_snapshot(
    agent_id,
    "Before experimental feature X".to_string()
).await?;

// Try risky operation
match risky_operation().await {
    Ok(_) => println!("Success!"),
    Err(e) => {
        // Rewind to snapshot
        let snapshot = manager.get_snapshot(&snapshot_id).await?;
        let point = RewindPoint::from_snapshot(&snapshot);
        manager.rewind_to(point, RewindConfig::safe()).await?;
    }
}
```

### 4. Use Appropriate Event Types

Record events with correct types for better navigation:

```rust
// Good: Specific event types
AgentHistoryEvent::new(agent_id, HistoryEventType::Decision, data);
AgentHistoryEvent::new(agent_id, HistoryEventType::Error, data);

// Avoid: Overusing generic types
AgentHistoryEvent::new(agent_id, HistoryEventType::System, everything);
```

### 5. Link Events with Git Commits

Always associate important events with git commits:

```rust
let commit = create_git_commit()?;

let event = AgentHistoryEvent::new(agent_id, event_type, data)
    .with_git_commit(commit);

store.record_event(&event).await?;
```

### 6. Handle Uncommitted Changes

Either commit or stash before rewinding:

```rust
// Option 1: Commit changes
git_commit("Work in progress")?;

// Option 2: Allow uncommitted changes (will be lost)
let config = RewindConfig {
    allow_uncommitted_changes: true,
    ..Default::default()
};

// Option 3: Stash changes (future enhancement)
```

### 7. Monitor Performance

For large histories, use pagination and filters:

```rust
// Don't load all events at once
let query = HistoryQuery {
    agent_id: Some(agent_id.to_string()),
    limit: Some(100),
    offset: Some(0),
    ..Default::default()
};

let events = store.query_events(&query).await?;
```

---

## Performance Considerations

### Event Storage

- **SQLite Performance**: ~1000 events/sec insertion rate
- **Query Optimization**: Indexed by agent_id, timestamp, event_type
- **Bulk Operations**: Use `record_events()` for batch inserts

### Rewind Operations

Performance benchmarks (reference hardware: 4-core CPU, SSD):

| Operation | Event Count | Git Commits | Duration |
|-----------|-------------|-------------|----------|
| Get Rewind Points | 1,000 | 10 | ~50ms |
| Brain Restore | 1,000 | N/A | ~200ms |
| Body Restore | N/A | 1 | ~100ms |
| Full Rewind | 1,000 | 10 | ~500ms |
| Snapshot Creation | 500 | 1 | ~100ms |
| Undo Operation | 1,000 | 1 | ~300ms |

### Large History Optimization

For histories with 10,000+ events:

```rust
// Use snapshots at regular intervals
if event_count % 1000 == 0 {
    manager.create_snapshot(
        agent_id,
        format!("Auto-snapshot at event {}", event_count)
    ).await?;
}

// Cleanup old events
let one_month_ago = Utc::now().timestamp() - (30 * 24 * 60 * 60);
store.delete_events_before(one_month_ago).await?;
```

### Memory Usage

Approximate memory requirements:

- **Event**: ~1KB per event (average)
- **Brain State**: ~100KB for 1000 events
- **Undo History**: ~1MB per backup (10 backups = ~10MB)

---

## Troubleshooting

### Common Issues

#### 1. Validation Errors After Rewind

**Problem**: Brain and body states don't match

**Solution**:
```rust
// Check git commit references
if let Some(brain_commit) = brain_state.git_commit {
    let current = body_manager.get_current_commit().await?;
    if brain_commit != current {
        println!("Mismatch: brain={}, body={}", brain_commit, current);
    }
}

// Ensure commits are tagged in events
let event = AgentHistoryEvent::new(...)
    .with_git_commit(current_commit);
```

#### 2. Cannot Rewind with Uncommitted Changes

**Problem**: Rewind blocked by dirty working directory

**Solution**:
```rust
// Option 1: Commit changes
Command::new("git")
    .args(["add", "."])
    .output()?;
Command::new("git")
    .args(["commit", "-m", "WIP"])
    .output()?;

// Option 2: Allow in config
let config = RewindConfig {
    allow_uncommitted_changes: true,
    ..Default::default()
};
```

#### 3. Missing Events in History

**Problem**: Events not appearing in rewind points

**Solution**:
```rust
// Verify events were recorded
let events = store.get_events(agent_id, 100).await?;
println!("Total events: {}", events.len());

// Check event filters
let points = manager.get_rewind_points(agent_id).await?;
// Points only include events with git commits or special types
```

#### 4. Slow Rewind Performance

**Problem**: Rewind takes too long

**Solution**:
```rust
// Create snapshots for frequent rewind points
manager.create_snapshot(agent_id, "checkpoint").await?;

// Use fast config when appropriate
let config = RewindConfig::fast();

// Limit event history
let query = HistoryQuery {
    limit: Some(1000),
    ..Default::default()
};
```

#### 5. Git Errors During Body Restore

**Problem**: Git operations failing

**Solution**:
```rust
// Verify repository state
let repo = gix::open(".")?;
let head = repo.head()?;
println!("Current HEAD: {:?}", head);

// Check for git issues
Command::new("git")
    .args(["fsck"])
    .output()?;
```

### Debug Logging

Enable detailed logging:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

---

## Examples

### Example 1: Simple Rewind

```rust
// See examples/time_travel_example.rs for complete code

// Create history and record events
let event1 = AgentHistoryEvent::new(agent_id, HistoryEventType::Thought, data);
store.record_event(&event1).await?;

let commit1 = create_git_commit()?;

// Rewind to event1
let point = RewindPoint {
    timestamp: event1.timestamp,
    event_id: Some(event1.event_id),
    git_commit: Some(commit1),
    ..Default::default()
};

let result = manager.rewind_to(point, RewindConfig::safe()).await?;
println!("Rewound successfully: {}", result.success);
```

### Example 2: Snapshot-Based Recovery

```rust
// Create checkpoint
let checkpoint = manager.create_snapshot(
    agent_id,
    "Before risky operation".to_string()
).await?;

// Try risky operation
match risky_operation().await {
    Ok(result) => {
        println!("Success: {:?}", result);
    }
    Err(e) => {
        println!("Failed: {}, recovering...", e);

        // Restore from checkpoint
        let snapshot = store.get_snapshot(&checkpoint).await?.unwrap();
        let point = RewindPoint::from_snapshot(&snapshot);
        manager.rewind_to(point, RewindConfig::safe()).await?;

        println!("Recovered to checkpoint");
    }
}
```

### Example 3: Interactive Debugging

```rust
// Rewind to error point
let error_event = find_first_error(&store, agent_id).await?;
let point = RewindPoint::from_event(&error_event, None);

let result = manager.rewind_to(point, RewindConfig::safe()).await?;

// Inspect state
let brain_state = result.brain_result.unwrap().brain_state.unwrap();
println!("Thought history: {:?}", brain_state.thought_history);
println!("Memory: {:?}", brain_state.memory);

// Resume with debugging
let ctx = ResumeContext::from_rewind_result(&result, agent_id)?
    .with_debugging();

manager.resume_from(ctx).await?;
```

### Example 4: Multiple Rewinds

```rust
// Create checkpoints through execution
let checkpoints = vec![];

for phase in phases {
    execute_phase(phase).await?;

    let snapshot_id = manager.create_snapshot(
        agent_id,
        format!("End of {}", phase)
    ).await?;

    checkpoints.push(snapshot_id);
}

// Later, jump to any phase
let phase2_snapshot = store.get_snapshot(&checkpoints[1]).await?.unwrap();
let point = RewindPoint::from_snapshot(&phase2_snapshot);
manager.rewind_to(point, RewindConfig::safe()).await?;
```

---

## Conclusion

Time Travel in Descartes provides powerful debugging and recovery capabilities. By combining event sourcing (brain) with git history (body), you get complete state restoration for your agents.

### Key Takeaways

- ✅ Always create backups before rewinding
- ✅ Validate state after restoration
- ✅ Use snapshots for frequently-accessed points
- ✅ Link events with git commits
- ✅ Monitor performance with large histories

### Next Steps

1. Run the example: `cargo run --example time_travel_example`
2. Read the API documentation: `cargo doc --open`
3. Explore the tests: `cargo test --test time_travel_integration_tests`
4. Try the GUI: Navigate to time travel tab in Descartes UI

---

## Support

For issues, questions, or contributions:

- **GitHub Issues**: [https://github.com/yourusername/descartes/issues](https://github.com/yourusername/descartes/issues)
- **Documentation**: [https://docs.descartes.ai](https://docs.descartes.ai)
- **Examples**: `/examples/time_travel_example.rs`

---

*Last updated: 2025-11-24*
*Version: 1.0.0*
