# Task Board Real-Time Updates - Quick Start Guide

## 5-Minute Setup

### Step 1: Add Event Handler to Your App

```rust
use descartes_gui::EventHandler;
use descartes_gui::task_board::{TaskBoardState, TaskBoardMessage, view, update};

struct MyApp {
    task_board: TaskBoardState,
    event_handler: EventHandler,
}

impl MyApp {
    fn new() -> Self {
        Self {
            task_board: TaskBoardState::new(),
            event_handler: EventHandler::default(), // Connects to ws://127.0.0.1:8080/events
        }
    }
}
```

### Step 2: Add Event Subscription

```rust
impl MyApp {
    fn subscription(&self) -> iced::Subscription<Message> {
        // Subscribe to real-time events
        self.event_handler.subscription(|event| {
            Message::TaskBoard(TaskBoardMessage::EventReceived(event))
        })
    }
}
```

### Step 3: Handle Messages

```rust
impl MyApp {
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::TaskBoard(msg) => {
                descartes_gui::task_board::update(&mut self.task_board, msg);
            }
            Message::Connect => {
                return self.event_handler.connect();
            }
        }
        iced::Task::none()
    }
}
```

### Step 4: Render View

```rust
impl MyApp {
    fn view(&self) -> iced::Element<Message> {
        descartes_gui::task_board::view(&self.task_board)
            .map(Message::TaskBoard)
    }
}
```

That's it! Your Task Board now receives real-time updates.

---

## Common Customizations

### Custom WebSocket URL

```rust
let event_handler = EventHandler::new("ws://localhost:9090/events".to_string());
```

### Filter Events

```rust
use descartes_daemon::{EventFilter, EventCategory};

let filter = EventFilter {
    event_categories: vec![EventCategory::Task],
    ..Default::default()
};

let event_handler = EventHandler::with_filter(
    "ws://127.0.0.1:8080/events".to_string(),
    filter
);
```

### Disable Real-Time Updates

```rust
// User can toggle
update(&mut task_board, TaskBoardMessage::ToggleRealtimeUpdates);

// Or disable at startup
task_board.realtime_state.enabled = false;
```

### Adjust Debouncing

```rust
// Change debounce interval (milliseconds)
task_board.realtime_state.debounce_ms = 200; // 200ms instead of default 100ms
```

---

## Troubleshooting

### Not Receiving Updates?

**Check connection:**
```rust
if task_board.realtime_state.connected {
    println!("Connected!");
} else {
    println!("Not connected");
}
```

**Check if updates enabled:**
```rust
if task_board.realtime_state.enabled {
    println!("Real-time updates enabled");
}
```

### Too Many Updates?

**Increase debounce interval:**
```rust
task_board.realtime_state.debounce_ms = 500; // 500ms
```

**Or flush pending updates manually:**
```rust
update(&mut task_board, TaskBoardMessage::FlushPendingUpdates);
```

### Connection Lost?

The UI will show: "Real-time connection lost"

**To reconnect:**
```rust
// EventClient handles reconnection automatically
// Or manually:
event_handler.disconnect().await;
event_handler.connect();
```

---

## Testing

### Run Tests

```bash
# GUI tests
cd descartes/gui
cargo test --test task_board_integration_tests

# Daemon tests
cd descartes/daemon
cargo test --test task_board_realtime_tests
```

### Check Statistics

```rust
println!("Events received: {}", task_board.realtime_state.events_received);
println!("Updates applied: {}", task_board.realtime_state.updates_applied);
```

---

## Full Example

```rust
use descartes_gui::{EventHandler, task_board::{TaskBoardState, TaskBoardMessage, view, update}};
use iced::{Element, Subscription, Task};

#[derive(Debug, Clone)]
enum Message {
    TaskBoard(TaskBoardMessage),
    Connect,
    Disconnect,
}

struct TaskBoardApp {
    state: TaskBoardState,
    event_handler: EventHandler,
}

impl TaskBoardApp {
    fn new() -> Self {
        Self {
            state: TaskBoardState::new(),
            event_handler: EventHandler::default(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TaskBoard(msg) => {
                update(&mut self.state, msg);
            }
            Message::Connect => {
                return self.event_handler.connect();
            }
            Message::Disconnect => {
                tokio::spawn(async move {
                    self.event_handler.disconnect().await;
                });
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        self.event_handler.subscription(|event| {
            Message::TaskBoard(TaskBoardMessage::EventReceived(event))
        })
    }

    fn view(&self) -> Element<Message> {
        view(&self.state).map(Message::TaskBoard)
    }
}

fn main() -> iced::Result {
    iced::application("Task Board", TaskBoardApp::update, TaskBoardApp::view)
        .subscription(TaskBoardApp::subscription)
        .run_with(|| (TaskBoardApp::new(), TaskBoardApp::update(Message::Connect)))
}
```

---

## Next Steps

- 📖 Read [REALTIME_UPDATES_README.md](./REALTIME_UPDATES_README.md) for detailed documentation
- 🧪 Check test files for more examples
- 🎨 Customize filters and debouncing for your use case
- 📊 Monitor statistics to optimize performance

**Happy coding!** 🚀
