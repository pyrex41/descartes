# Task Event Emitter Quick Start Guide

## Overview

The Task Event Emitter provides real-time change notifications for tasks stored in SQLite, enabling instant GUI updates via WebSocket.

## 5-Minute Setup

### 1. Import Dependencies

```rust
use descartes_daemon::{
    TaskEventEmitter, TaskEventEmitterConfig, EventBus
};
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::StateStore;
use std::sync::Arc;
```

### 2. Initialize Components

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create state store
    let mut state_store = SqliteStateStore::new("./data/tasks.db", false).await?;
    state_store.initialize().await?;
    let state_store = Arc::new(state_store) as Arc<dyn StateStore>;

    // Create event bus
    let event_bus = Arc::new(EventBus::new());

    // Create task event emitter
    let task_emitter = TaskEventEmitter::with_defaults(
        state_store,
        event_bus.clone(),
    );

    // Initialize cache with existing tasks
    task_emitter.initialize_cache().await?;

    Ok(())
}
```

### 3. Use the Emitter

```rust
use descartes_core::traits::{Task, TaskStatus, TaskPriority, TaskComplexity};
use uuid::Uuid;
use chrono::Utc;

// Create a task
let task = Task {
    id: Uuid::new_v4(),
    title: "Implement feature X".to_string(),
    description: Some("Add new API endpoint".to_string()),
    status: TaskStatus::Todo,
    priority: TaskPriority::High,
    complexity: TaskComplexity::Moderate,
    assigned_to: Some("agent-1".to_string()),
    dependencies: vec![],
    created_at: Utc::now().timestamp(),
    updated_at: Utc::now().timestamp(),
    metadata: None,
};

// Save task (automatically emits Created event)
task_emitter.save_task(&task).await?;

// Update task (automatically emits Updated event)
let mut updated_task = task.clone();
updated_task.status = TaskStatus::InProgress;
updated_task.updated_at = Utc::now().timestamp();
task_emitter.save_task(&updated_task).await?;
```

### 4. Subscribe to Events

```rust
// Subscribe to all task events
let (_subscription_id, mut event_receiver) = event_bus.subscribe(None).await;

// Listen for events
tokio::spawn(async move {
    while let Ok(event) = event_receiver.recv().await {
        match event {
            DescartesEvent::TaskEvent(task_event) => {
                println!("Task {} changed: {:?}",
                    task_event.task_id,
                    task_event.event_type
                );

                // Access change details
                if let Some(change_type) = task_event.data.get("change_type") {
                    match change_type.as_str() {
                        Some("created") => {
                            println!("New task created!");
                        }
                        Some("updated") => {
                            if let Some(prev) = task_event.data.get("previous_status") {
                                if let Some(new) = task_event.data.get("new_status") {
                                    println!("Status changed: {} → {}",
                                        prev.as_str().unwrap(),
                                        new.as_str().unwrap()
                                    );
                                }
                            }
                        }
                        Some("deleted") => {
                            println!("Task deleted!");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
});
```

## Configuration Options

### Default Configuration

```rust
let emitter = TaskEventEmitter::with_defaults(state_store, event_bus);
// - Debouncing enabled (100ms)
// - Full task data included in events
// - Verbose logging disabled
```

### Custom Configuration

```rust
let config = TaskEventEmitterConfig {
    enable_debouncing: true,
    debounce_interval_ms: 200,  // 200ms debounce
    include_task_data: true,     // Include full task in events
    verbose_logging: true,       // Enable debug logs
};

let emitter = TaskEventEmitter::new(state_store, event_bus, config);
```

## Common Use Cases

### Use Case 1: Real-time Kanban Board

```rust
// GUI subscribes to task events
let (_sub_id, mut rx) = event_bus.subscribe(None).await;

// Handle events
while let Ok(event) = rx.recv().await {
    if let DescartesEvent::TaskEvent(task_event) = event {
        let change_type = task_event.data["change_type"].as_str().unwrap();

        match change_type {
            "created" => {
                let task = serde_json::from_value(
                    task_event.data["task"].clone()
                ).unwrap();
                kanban_board.add_task(task);
            }
            "updated" => {
                let task = serde_json::from_value(
                    task_event.data["task"].clone()
                ).unwrap();
                kanban_board.update_task(task);
            }
            "deleted" => {
                kanban_board.remove_task(&task_event.task_id);
            }
            _ => {}
        }
    }
}
```

### Use Case 2: Task Status Notifications

```rust
// Monitor for tasks moving to Done status
while let Ok(event) = rx.recv().await {
    if let DescartesEvent::TaskEvent(task_event) = event {
        if let Some("updated") = task_event.data["change_type"].as_str() {
            if let Some("Done") = task_event.data["new_status"].as_str() {
                let task = &task_event.data["task"];
                send_notification(&format!(
                    "Task '{}' completed!",
                    task["title"].as_str().unwrap()
                ));
            }
        }
    }
}
```

### Use Case 3: Concurrent Task Updates

```rust
// Multiple agents can safely update tasks concurrently
let emitter = Arc::new(task_emitter);

let mut handles = vec![];
for agent_id in 0..10 {
    let emitter = emitter.clone();
    let handle = tokio::spawn(async move {
        let mut task = get_assigned_task(agent_id).await?;
        task.status = TaskStatus::InProgress;
        emitter.save_task(&task).await?;  // Thread-safe, emits event
        Ok::<(), Error>(())
    });
    handles.push(handle);
}

for handle in handles {
    handle.await??;
}
```

## Event Structure

### Created Event

```json
{
  "type": "Event",
  "payload": {
    "TaskEvent": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "task_id": "123e4567-e89b-12d3-a456-426614174000",
      "agent_id": null,
      "timestamp": "2025-11-24T10:30:00Z",
      "event_type": "created",
      "data": {
        "change_type": "created",
        "task": {
          "id": "123e4567-e89b-12d3-a456-426614174000",
          "title": "Implement feature X",
          "status": "Todo",
          "priority": "high",
          ...
        },
        "timestamp": 1732447800
      }
    }
  }
}
```

### Updated Event

```json
{
  "type": "Event",
  "payload": {
    "TaskEvent": {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "task_id": "123e4567-e89b-12d3-a456-426614174000",
      "agent_id": null,
      "timestamp": "2025-11-24T10:35:00Z",
      "event_type": "progress",
      "data": {
        "change_type": "updated",
        "task": { ... },
        "previous_status": "Todo",
        "new_status": "InProgress",
        "timestamp": 1732448100
      }
    }
  }
}
```

### Deleted Event

```json
{
  "type": "Event",
  "payload": {
    "TaskEvent": {
      "id": "550e8400-e29b-41d4-a716-446655440002",
      "task_id": "123e4567-e89b-12d3-a456-426614174000",
      "agent_id": null,
      "timestamp": "2025-11-24T10:40:00Z",
      "event_type": "cancelled",
      "data": {
        "change_type": "deleted",
        "timestamp": 1732448400
      }
    }
  }
}
```

## Performance Tips

### 1. Enable Debouncing for High-Frequency Updates

```rust
let config = TaskEventEmitterConfig {
    enable_debouncing: true,
    debounce_interval_ms: 100,  // Adjust based on update frequency
    ..Default::default()
};
```

### 2. Disable Task Data for Large Tasks

If tasks have large metadata, disable full task inclusion:

```rust
let config = TaskEventEmitterConfig {
    include_task_data: false,  // Only include task_id
    ..Default::default()
};
```

Then fetch task separately if needed:

```rust
if let DescartesEvent::TaskEvent(task_event) = event {
    let task = task_emitter.get_task(&task_event.task_id.parse()?).await?;
}
```

### 3. Filter Events at Subscription Level

```rust
use descartes_daemon::events::{EventFilter, EventCategory};

// Only subscribe to task events
let filter = EventFilter {
    event_categories: vec![EventCategory::Task],
    ..Default::default()
};

let (_sub_id, mut rx) = event_bus.subscribe(Some(filter)).await;
```

### 4. Monitor Statistics

```rust
let stats = task_emitter.get_statistics().await;
println!("Cached tasks: {}", stats.cached_tasks);
println!("Pending debounced events: {}", stats.pending_debounced_events);
```

## Troubleshooting

### Problem: No events received

**Solution:** Ensure `initialize_cache()` was called:

```rust
task_emitter.initialize_cache().await?;
```

### Problem: Too many events

**Solution:** Enable or increase debounce interval:

```rust
let config = TaskEventEmitterConfig {
    enable_debouncing: true,
    debounce_interval_ms: 200,  // Increase interval
    ..Default::default()
};
```

### Problem: Events delayed

**Solution:** Flush pending events:

```rust
task_emitter.flush_debounced_events().await;
```

### Problem: Out of sync with database

**Solution:** Reinitialize cache:

```rust
task_emitter.initialize_cache().await?;
```

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_task_creation() {
    let (emitter, event_bus) = setup_test_system().await;
    let (_sub, mut rx) = event_bus.subscribe(None).await;

    let task = create_test_task();
    emitter.save_task(&task).await.unwrap();

    let event = rx.recv().await.unwrap();
    assert!(matches!(event, DescartesEvent::TaskEvent(_)));
}
```

### Integration Tests

See `/home/user/descartes/descartes/daemon/tests/task_event_integration_test.rs` for comprehensive examples.

## Next Steps

1. **Integrate with RPC Server**: Replace direct StateStore calls
2. **Connect GUI**: Subscribe to WebSocket events
3. **Add Metrics**: Monitor event emission rates
4. **Implement Filtering**: Add business logic filters

## Resources

- Full Implementation Report: `/working_docs/implementation/PHASE3_TASK_4_3_REPORT.md`
- Event Bus Guide: `/EVENT_SUBSCRIPTION_QUICK_START.md`
- API Reference: `/descartes/daemon/src/task_event_emitter.rs`
- Integration Tests: `/descartes/daemon/tests/task_event_integration_test.rs`
