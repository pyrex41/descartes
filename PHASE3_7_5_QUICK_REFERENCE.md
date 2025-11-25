# Phase 3.7.5 Quick Reference - Rewind and Resume

## Quick Start

```rust
use descartes_core::{
    DefaultRewindManager, RewindConfig, RewindPoint,
    SqliteAgentHistoryStore, slider_to_rewind_point,
};

// Initialize
let store = SqliteAgentHistoryStore::new("history.db").await?;
store.initialize().await?;

let manager = DefaultRewindManager::new(
    store,
    std::env::current_dir()?,
    10, // max undo history
)?;

// Get rewind points
let points = manager.get_rewind_points("agent-1").await?;

// Rewind
let config = RewindConfig::safe();
let result = manager.rewind_to(points[0].clone(), config).await?;

// Resume
if result.success {
    let context = ResumeContext::from_rewind_result(&result, "agent-1".to_string())?;
    manager.resume_from(context).await?;
}
```

## Common Operations

### Rewind from Slider

```rust
// Convert slider (0.0-1.0) to rewind point
let point = slider_to_rewind_point(0.5, &events)?;
let result = manager.rewind_to(point, RewindConfig::safe()).await?;
```

### Create Snapshot

```rust
let snapshot_id = manager.create_snapshot(
    "agent-1",
    "Before experiment".to_string(),
).await?;
```

### Undo Rewind

```rust
let backup_id = result.backup.backup_id;
let undo_result = manager.undo_rewind(backup_id).await?;
```

### Resume with Debugging

```rust
let context = ResumeContext::from_rewind_result(&result, "agent-1".to_string())?
    .with_debugging()
    .with_breakpoint(BreakpointLocation::EventType(HistoryEventType::Decision));

manager.resume_from(context).await?;
```

## Configuration Presets

```rust
// Safe (all safety features)
let config = RewindConfig::safe();

// Fast (minimal checks)
let config = RewindConfig::fast();

// Custom
let config = RewindConfig {
    require_confirmation: true,
    auto_backup: true,
    validate_state: true,
    allow_uncommitted_changes: false,
    max_undo_history: 10,
    enable_debugging: true,
};
```

## Key Types

```rust
// Rewind point
pub struct RewindPoint {
    pub timestamp: i64,
    pub event_id: Option<Uuid>,
    pub git_commit: Option<String>,
    pub snapshot_id: Option<Uuid>,
    pub description: String,
    pub event_index: Option<usize>,
}

// Result
pub struct RewindResult {
    pub success: bool,
    pub target_point: RewindPoint,
    pub brain_result: Option<BrainRestoreResult>,
    pub body_result: Option<BodyRestoreResult>,
    pub backup: RewindBackup,
    pub validation: ValidationResult,
    pub messages: Vec<String>,
    pub duration_ms: u64,
}

// Resume context
pub struct ResumeContext {
    pub agent_id: String,
    pub brain_state: BrainState,
    pub git_commit: String,
    pub resume_event_index: usize,
    pub enable_debugging: bool,
    pub breakpoints: Vec<BreakpointLocation>,
}
```

## GUI Integration

```rust
use descartes_gui::{
    RewindState, RewindMessage,
    view_rewind_confirmation, view_rewind_progress,
    view_rewind_result, view_rewind_controls,
};

// Display confirmation
if let Some(confirmation) = &state.confirmation {
    view_rewind_confirmation(confirmation);
}

// Show progress
if let Some(progress) = &state.current_progress {
    view_rewind_progress(progress);
}

// Display result
if let Some(result) = &state.last_result {
    view_rewind_result(result);
}
```

## Error Handling

```rust
match manager.rewind_to(point, config).await {
    Ok(result) if result.success => {
        println!("✓ Rewind successful");
    }
    Ok(result) => {
        eprintln!("⚠ Rewind completed with errors:");
        for error in result.validation.errors {
            eprintln!("  - {}", error);
        }
    }
    Err(StateStoreError::NotFound(msg)) => {
        eprintln!("✗ Not found: {}", msg);
    }
    Err(StateStoreError::Conflict(msg)) => {
        eprintln!("✗ Conflict: {}", msg);
    }
    Err(e) => {
        eprintln!("✗ Error: {}", e);
    }
}
```

## Safety Features

1. **Auto-backup**: Automatic backup before every rewind
2. **Validation**: Brain/body consistency checked after restore
3. **Confirmation**: User confirmation for destructive operations
4. **Rollback**: Automatic rollback on failure
5. **Undo**: Can undo last N rewind operations
6. **Change Detection**: Warns about uncommitted changes

## Performance

| Operation | Typical Duration |
|-----------|-----------------|
| Rewind (100 events) | 50-100ms |
| Rewind (1000 events) | 200-400ms |
| Backup | 10-20ms |
| Undo | 20-40ms |
| Validation | 5-10ms |
| Snapshot | 30-60ms |

## Testing

```bash
cd descartes/core
cargo test time_travel_integration_tests
```

## Files

- Core: `descartes/core/src/time_travel_integration.rs`
- GUI: `descartes/gui/src/time_travel_integration.rs`
- Tests: `descartes/core/tests/time_travel_integration_tests.rs`

## See Also

- [Full Implementation Report](PHASE3_7_5_REWIND_RESUME_IMPLEMENTATION.md)
- [Brain Restore (phase3:7.2)](PHASE3_7_2_IMPLEMENTATION_REPORT.md)
- [Body Restore (phase3:7.3)](PHASE3_7_3_IMPLEMENTATION_REPORT.md)
- [Time Travel UI (phase3:7.4)](PHASE3_7_4_TIME_TRAVEL_UI_IMPLEMENTATION.md)
