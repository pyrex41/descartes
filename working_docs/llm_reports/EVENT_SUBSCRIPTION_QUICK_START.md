# Event Subscription Quick Start Guide

This guide shows you how to quickly add event subscription to your Descartes application.

---

## For Daemon Developers: Publishing Events

### Step 1: Get EventBus Reference

```rust
use descartes_daemon::{EventBus, AgentEvent, TaskEvent, SystemEvent};
use std::sync::Arc;

// In your daemon struct
pub struct Daemon {
    event_bus: Arc<EventBus>,
    // ... other fields
}
```

### Step 2: Publish Events

```rust
// Agent spawned
self.event_bus.publish(
    AgentEvent::spawned(
        agent_id.clone(),
        serde_json::json!({"config": "basic"}),
    )
).await;

// Task progress
self.event_bus.publish(
    TaskEvent::progress(
        task_id.clone(),
        0.5,
        "Processing data".to_string(),
    )
).await;

// System metrics
self.event_bus.publish(
    SystemEvent::metrics_update(
        serde_json::json!({
            "cpu": 45.2,
            "memory": 1024,
        })
    )
).await;
```

---

## For GUI Developers: Receiving Events

### Step 1: Add EventHandler to App State

```rust
use descartes_gui::EventHandler;
use descartes_daemon::DescartesEvent;

struct App {
    event_handler: EventHandler,
    // ... other fields
}
```

### Step 2: Initialize in Constructor

```rust
impl App {
    fn new() -> Self {
        Self {
            event_handler: EventHandler::new(
                "ws://127.0.0.1:8080/events".to_string()
            ),
            // ... other fields
        }
    }
}
```

### Step 3: Add Event Message

```rust
enum Message {
    Connect,
    EventReceived(DescartesEvent),
    // ... other messages
}
```

### Step 4: Handle Connect Message

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::Connect => {
            self.event_handler.connect()
        }
        Message::EventReceived(event) => {
            match event {
                DescartesEvent::AgentEvent(e) => {
                    println!("Agent {}: {:?}", e.agent_id, e.event_type);
                }
                DescartesEvent::TaskEvent(e) => {
                    println!("Task {}: {:?}", e.task_id, e.event_type);
                }
                _ => {}
            }
            Task::none()
        }
        // ... other messages
    }
}
```

### Step 5: Add Subscription

```rust
fn subscription(&self) -> Subscription<Message> {
    self.event_handler.subscription(Message::EventReceived)
}
```

### Step 6: Add Subscription to Application

```rust
fn main() -> iced::Result {
    iced::application("My App", App::update, App::view)
        .subscription(App::subscription)  // Add this line
        .run_with(|| (App::new(), Task::none()))
}
```

---

## Advanced: Event Filtering

### Filter by Agent

```rust
use descartes_daemon::EventFilter;

let event_handler = EventHandler::with_filter(
    "ws://127.0.0.1:8080/events".to_string(),
    EventFilter::for_agent("my-agent-id".to_string()),
);
```

### Custom Filter

```rust
use descartes_daemon::{EventFilter, EventCategory};

let filter = EventFilter {
    agent_ids: vec!["agent-1".to_string()],
    task_ids: vec![],
    workflow_ids: vec![],
    event_categories: vec![
        EventCategory::Agent,
        EventCategory::Task,
    ],
};

let event_handler = EventHandler::with_filter(
    "ws://127.0.0.1:8080/events".to_string(),
    filter,
);
```

---

## Advanced: EventClient Configuration

```rust
use descartes_daemon::{EventClientBuilder, EventFilter};
use std::time::Duration;

let (client, event_rx) = EventClientBuilder::new()
    .url("ws://127.0.0.1:8080/events")
    .connect_timeout(Duration::from_secs(10))
    .max_reconnect_attempts(5)  // or -1 for infinite
    .reconnect_delay(Duration::from_secs(10))
    .filter(EventFilter::for_agent("my-agent"))
    .build();
```

---

## Testing Your Implementation

### 1. Start the Daemon
```bash
cd /home/user/descartes/descartes
cargo run --bin descartes-daemon
```

### 2. Run Example App
```bash
cargo run --example event_subscription_example
```

### 3. Trigger Events
```bash
# Spawn an agent (triggers AgentEvent::Spawned)
curl -X POST http://127.0.0.1:8080/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"agent.spawn","params":{"name":"test","agent_type":"basic","config":{}},"id":1}'
```

---

## Event Types Reference

### Agent Events
- `Spawned` - Agent created
- `Started` - Agent began execution
- `StatusChanged` - Agent status updated
- `Paused` - Agent paused
- `Resumed` - Agent resumed
- `Completed` - Agent finished successfully
- `Failed` - Agent encountered error
- `Killed` - Agent terminated
- `Log` - Agent log message
- `Metric` - Agent metric update

### Task Events
- `Created` - Task created
- `Started` - Task execution started
- `Progress` - Task progress update
- `Completed` - Task finished
- `Failed` - Task failed
- `Cancelled` - Task cancelled

### Workflow Events
- `Started` - Workflow execution started
- `StepCompleted` - Workflow step completed
- `Completed` - Workflow finished
- `Failed` - Workflow failed
- `Paused` - Workflow paused
- `Resumed` - Workflow resumed

### System Events
- `DaemonStarted` - Daemon started
- `DaemonStopping` - Daemon shutting down
- `HealthCheck` - Health check update
- `MetricsUpdate` - System metrics update
- `ConnectionEstablished` - Client connected
- `ConnectionClosed` - Client disconnected
- `Error` - System error

### State Events
- `Created` - State created
- `Updated` - State modified
- `Deleted` - State removed

---

## Common Patterns

### Pattern 1: Display Recent Events

```rust
struct App {
    event_handler: EventHandler,
    recent_events: Vec<String>,
    max_events: usize,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventReceived(event) => {
                let summary = format!("{:?}", event);
                self.recent_events.insert(0, summary);

                if self.recent_events.len() > self.max_events {
                    self.recent_events.truncate(self.max_events);
                }

                Task::none()
            }
            // ...
        }
    }
}
```

### Pattern 2: Agent Status Tracking

```rust
use std::collections::HashMap;

struct App {
    event_handler: EventHandler,
    agent_status: HashMap<String, AgentStatus>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventReceived(DescartesEvent::AgentEvent(e)) => {
                match e.event_type {
                    AgentEventType::StatusChanged => {
                        if let Some(status) = e.data.get("status") {
                            self.agent_status.insert(
                                e.agent_id.clone(),
                                parse_status(status),
                            );
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            // ...
        }
    }
}
```

### Pattern 3: Task Progress Bar

```rust
struct App {
    event_handler: EventHandler,
    task_progress: HashMap<String, f32>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventReceived(DescartesEvent::TaskEvent(e)) => {
                match e.event_type {
                    TaskEventType::Progress => {
                        if let Some(progress) = e.data.get("progress") {
                            if let Some(p) = progress.as_f64() {
                                self.task_progress.insert(
                                    e.task_id.clone(),
                                    p as f32,
                                );
                            }
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            // ...
        }
    }
}
```

---

## Troubleshooting

### Connection Failed
- Ensure daemon is running
- Check URL is correct (ws:// not wss://)
- Verify firewall allows WebSocket connections

### No Events Received
- Check event filter isn't too restrictive
- Verify daemon is publishing events
- Check subscription was confirmed (check logs)

### Frequent Disconnections
- Increase reconnection delay
- Check network stability
- Review daemon logs for errors

---

## Performance Tips

1. **Use Filters**: Reduce bandwidth by filtering events server-side
2. **Limit Event History**: Don't accumulate unlimited events in memory
3. **Debounce UI Updates**: Update UI in batches for high-frequency events
4. **Use Event Categories**: Subscribe only to categories you need

---

## Next Steps

- See full example: `descartes/gui/examples/event_subscription_example.rs`
- Read full report: `working_docs/implementation/PHASE3_TASK_3_3_REPORT.md`
- Check API docs: `cargo doc --open -p descartes-daemon`

---

**Happy Eventing! 🚀**
