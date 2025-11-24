# Phase 3:7.2 Quick Reference - Restore Brain Functionality

**Status**: ✅ COMPLETE | **Date**: 2025-11-24

## File Locations

| Component | Path | Purpose |
|-----------|------|---------|
| **Brain Restore** | `/home/user/descartes/descartes/core/src/brain_restore.rs` | Complete implementation (1,050 lines) |
| **Exports** | `/home/user/descartes/descartes/core/src/lib.rs` | Module exports (updated) |
| **Full Report** | `/home/user/descartes/PHASE3_7_2_IMPLEMENTATION_REPORT.md` | Detailed documentation |

---

## Quick Start

### 1. Initialize Restore System

```rust
use descartes_core::{
    SqliteAgentHistoryStore, DefaultBrainRestore, BrainRestore,
    BrainRestoreOptions,
};

// Create store and restore
let store = SqliteAgentHistoryStore::new("./history.db").await?;
let restore = DefaultBrainRestore::new(store);
```

### 2. Load Events

```rust
// Up to a timestamp
let events = restore.load_events_until("agent-123", timestamp).await?;

// Time range
let events = restore.load_events_range("agent-123", start, end).await?;

// By type
let events = restore.filter_by_event_type(
    "agent-123",
    vec![HistoryEventType::Thought, HistoryEventType::Decision]
).await?;
```

### 3. Replay Events

```rust
// Basic replay
let result = restore
    .replay_events(events, BrainRestoreOptions::default())
    .await?;

if result.success {
    let state = result.brain_state.unwrap();
    println!("Thoughts: {}", state.thought_history.len());
    println!("Decisions: {}", state.decision_tree.len());
    println!("Messages: {}", state.conversation_state.messages.len());
}
```

### 4. Restore from Snapshot

```rust
let result = restore
    .restore_brain_state(&snapshot_id, BrainRestoreOptions::default())
    .await?;
```

---

## Core Types

### BrainState

```rust
pub struct BrainState {
    pub agent_id: String,
    pub timestamp: i64,
    pub thought_history: Vec<ThoughtEntry>,      // Thoughts
    pub decision_tree: Vec<DecisionNode>,        // Decisions
    pub memory: HashMap<String, Value>,          // Memory
    pub conversation_state: ConversationState,   // Conversation
    pub session_id: Option<String>,
    pub metadata: HashMap<String, Value>,
    pub git_commit: Option<String>,
}
```

### RestoreOptions Presets

```rust
// Full validation
BrainRestoreOptions::with_validation()

// Skip validation
BrainRestoreOptions::without_validation()

// Lenient (skip errors)
BrainRestoreOptions::lenient()

// Custom filters
BrainRestoreOptions::default()
    .with_event_filters(vec![HistoryEventType::Thought])
```

### RestoreResult

```rust
pub struct RestoreResult {
    pub success: bool,
    pub brain_state: Option<BrainState>,
    pub events_processed: usize,
    pub events_skipped: usize,
    pub validation_errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
}
```

---

## Key Features

### Event Loading
- Load events up to timestamp
- Load events in time range
- Filter by event type
- Efficient database queries

### State Reconstruction
- Replay events to rebuild state
- Restore from snapshots
- Apply single events
- Causality-aware sorting

### Brain Components
- Thought history
- Decision tree
- Memory/context
- Conversation state

### Validation
- Orphaned reference detection
- State consistency checks
- Configurable validation levels
- Detailed error reporting

### Error Handling
- Graceful degradation
- Skip-on-error mode
- Detailed warnings
- Partial success support

---

## Common Patterns

### Time-Travel Debugging

```rust
// Restore to breakpoint
let events = restore.load_events_until(agent_id, breakpoint_time).await?;
let result = restore.replay_events(events, options).await?;
let state = result.brain_state.unwrap();

// Inspect state
println!("Last thought: {:?}", state.thought_history.last());
println!("Last decision: {:?}", state.decision_tree.last());
```

### State Comparison

```rust
use descartes_core::compare_states;

let state1 = restore.replay_events(events1, options).await?.brain_state.unwrap();
let state2 = restore.replay_events(events2, options).await?.brain_state.unwrap();

let differences = compare_states(&state1, &state2);
for diff in differences {
    println!("{}", diff);
}
```

### Event Filtering

```rust
let options = BrainRestoreOptions::default()
    .with_event_filters(vec![
        HistoryEventType::Thought,
        HistoryEventType::Decision,
    ]);

let result = restore.replay_events(events, options).await?;
```

### Validation

```rust
let errors = restore.validate_state(&state)?;
if !errors.is_empty() {
    eprintln!("Validation errors:");
    for error in errors {
        eprintln!("  - {}", error);
    }
}
```

---

## Testing

```bash
# All brain restore tests
cargo test brain_restore

# Specific test
cargo test test_replay_events

# With output
cargo test brain_restore -- --nocapture
```

### Test Coverage

13 comprehensive tests:
- Event loading (3 tests)
- Event replay (4 tests)
- Validation (2 tests)
- Dependencies (1 test)
- Edge cases (3 tests)

---

## Integration

### With Agent History (3:7.1)

```rust
// Uses existing event storage
let restore = DefaultBrainRestore::new(history_store);
let events = restore.load_events_until(agent_id, timestamp).await?;
```

### With Debugger (3:6.2)

```rust
// Restore to debugger breakpoint
let state = restore.restore_brain_state(&snapshot_id, options).await?;
debugger.set_state(state.brain_state.unwrap());
```

### With State Machine

```rust
// Analyze state transitions
let events = restore.filter_by_event_type(
    agent_id,
    vec![HistoryEventType::StateChange]
).await?;
```

---

## Performance

- **Event Loading**: O(n) with database indexes
- **Event Replay**: O(n) sequential processing
- **Causality Sort**: O(n log n) topological sort
- **Validation**: O(n) orphan checks

---

## Exports

Added to `descartes-core`:

```rust
// Core types
BrainRestore              // Trait
BrainState                // State model
DefaultBrainRestore       // Implementation
BrainRestoreOptions       // Configuration
BrainRestoreResult        // Result type

// Component types
ThoughtEntry              // Thought model
DecisionNode              // Decision model
ConversationState         // Conversation model
MessageEntry              // Message model

// Utilities
create_snapshot_from_state  // Create snapshot
compare_states               // Compare states
```

---

## Next Steps

1. **Use for Time-Travel Debugging**: Integrate with debugger (3:6.2)
2. **Implement Coordinated Restore**: Combine with body restore (3:7.3)
3. **Add Streaming Support**: Handle large event sets
4. **Create Restore UI**: Build visual restore interface

---

For detailed information, see:
- **Full Report**: `/home/user/descartes/PHASE3_7_2_IMPLEMENTATION_REPORT.md`
- **Source Code**: `/home/user/descartes/descartes/core/src/brain_restore.rs`
- **Phase 3:7.1**: `/home/user/descartes/PHASE3_7_1_SUMMARY.md`
