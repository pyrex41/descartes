# SCUD Bridge Architecture Design Document

**Task**: 6 - Design SCUD Bridge architecture
**Date**: 2026-01-20
**Status**: Complete
**Author**: Generated from implementation analysis

---

## Overview

The SCUD Bridge is the communication layer between the Descartes GUI (Iced application) and the SCUD CLI. It enables real-time, bi-directional communication through:

1. **Subprocess spawning** - Executing SCUD CLI commands
2. **JSON event streaming** - Parsing SCUD's structured output
3. **Async channels** - Thread-safe message passing to the GUI
4. **Iced subscription integration** - Real-time UI updates

---

## API Signatures

### ScudEvent Enum

Events emitted by the SCUD Bridge to notify the GUI of state changes.

```rust
/// Events emitted by SCUD execution
///
/// These events are sent from the ScudBridge to the GUI to update state.
#[derive(Debug, Clone)]
pub enum ScudEvent {
    /// Tasks loaded from SCUD storage
    TasksLoaded(Vec<TaskInfo>),

    /// Waves computed from task dependencies
    WavesComputed(Vec<Vec<String>>),

    /// Swarm execution started
    SwarmStarted { tag: String, total_waves: usize },

    /// A wave of tasks started
    WaveStarted { wave: usize, tasks: Vec<String> },

    /// Individual task started execution
    TaskStarted { task_id: String },

    /// Task output received (streaming)
    TaskOutput { task_id: String, text: String },

    /// Individual task completed
    TaskCompleted { task_id: String, success: bool },

    /// Validation started (backpressure check)
    ValidationStarted,

    /// Validation completed
    ValidationCompleted { passed: bool, output: String },

    /// Wave completed
    WaveCompleted { wave: usize },

    /// Swarm execution completed
    SwarmCompleted { success: bool },

    /// Generic output (for non-JSON streaming text)
    Output(String),

    /// Error occurred
    Error(String),
}
```

### ScudCommand Enum

Commands sent from the GUI to the SCUD Bridge.

```rust
/// Commands to send to SCUD
///
/// These commands are sent from the GUI to the ScudBridge.
#[derive(Debug, Clone)]
pub enum ScudCommand {
    /// Load tasks from SCUD, optionally filtered by tag
    LoadTasks { tag: Option<String> },

    /// Compute execution waves for a tag
    ComputeWaves { tag: String },

    /// Start swarm execution
    StartSwarm {
        tag: String,
        harness: String,
        round_size: usize,
    },

    /// Stop the currently running swarm
    StopSwarm,

    /// Mark a task as complete
    CompleteTask { task_id: String },

    /// Mark a task as blocked
    BlockTask { task_id: String },
}
```

### ScudBridge Struct

The main bridge component that manages subprocess communication.

```rust
/// Bridge between Iced GUI and SCUD CLI
///
/// Handles subprocess spawning, JSON parsing, and channel communication
/// for real-time updates during SCUD execution.
pub struct ScudBridge {
    /// Sender for events to GUI
    event_tx: mpsc::Sender<ScudEvent>,

    /// Receiver for commands from GUI
    command_rx: mpsc::Receiver<ScudCommand>,

    /// Handle to current swarm process (for cancellation)
    swarm_handle: Option<tokio::process::Child>,
}

impl ScudBridge {
    /// Create a new ScudBridge with the given channel endpoints
    pub fn new(
        event_tx: mpsc::Sender<ScudEvent>,
        command_rx: mpsc::Receiver<ScudCommand>,
    ) -> Self;

    /// Create a new ScudBridge and return the channel handles for the GUI
    ///
    /// Returns (bridge, command_sender, event_receiver)
    pub fn create() -> (
        Self,
        mpsc::Sender<ScudCommand>,
        mpsc::Receiver<ScudEvent>,
    );

    /// Main run loop - processes commands from the GUI
    pub async fn run(mut self);
}
```

### JSON Event Format (from SCUD CLI)

The bridge parses JSON events from SCUD's `--json-events` output:

```rust
/// JSON event format from SCUD CLI when running with --json-events
#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ScudJsonEvent {
    SwarmStarted { tag: String, total_waves: usize },
    WaveStarted { wave: usize, tasks: Vec<String> },
    TaskStarted { task_id: String },
    TaskOutput { task_id: String, text: String },
    TaskCompleted { task_id: String, success: bool },
    ValidationStarted,
    ValidationCompleted { passed: bool, #[serde(default)] output: String },
    WaveCompleted { wave: usize },
    SwarmCompleted { success: bool },
}
```

### Task Info Structure

```rust
/// JSON task format from SCUD CLI when running with --json
#[derive(Debug, Deserialize)]
struct ScudJsonTask {
    id: String,
    title: String,
    status: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    complexity: Option<usize>,
}

/// Simplified task info for GUI display
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub status: String,
}
```

---

## Data Flow Diagrams

### Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           Descartes GUI (Iced)                            │
│                                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │                    Main Application (UI Thread)                      │ │
│  │                                                                       │ │
│  │  ┌─────────────┐    ┌────────────┐    ┌─────────────┐               │ │
│  │  │   view()    │◀───│  update()  │◀───│ AppState    │               │ │
│  │  │ - Waves     │    │ - Message  │    │ - waves     │               │ │
│  │  │ - Agents    │    │   dispatch │    │ - tasks     │               │ │
│  │  │ - Output    │    │            │    │ - status    │               │ │
│  │  └─────────────┘    └────────────┘    └─────────────┘               │ │
│  │                           ▲                                          │ │
│  │                           │                                          │ │
│  │                  Message::ScudEvent(event)                          │ │
│  │                           │                                          │ │
│  │  ┌────────────────────────┴──────────────────────────────┐          │ │
│  │  │              Subscription::run_with()                  │          │ │
│  │  │   async_stream::stream! { yield Message::ScudEvent }   │          │ │
│  │  │   Arc<TokioMutex<Option<Receiver<ScudEvent>>>>         │          │ │
│  │  └────────────────────────┬──────────────────────────────┘          │ │
│  │                           │                                          │ │
│  └───────────────────────────┼──────────────────────────────────────────┘ │
│                              │                                            │
│         mpsc::Receiver<ScudEvent>                                         │
│                              │                                            │
├──────────────────────────────┼────────────────────────────────────────────┤
│                              │                                            │
│         Background Thread (OS Thread + Tokio Runtime)                     │
│                              │                                            │
│  ┌───────────────────────────┴──────────────────────────────────────────┐ │
│  │                         ScudBridge                                    │ │
│  │                                                                       │ │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │ │
│  │  │                     Main Loop (async)                           │ │ │
│  │  │  while let Some(cmd) = command_rx.recv().await {                │ │ │
│  │  │      match cmd {                                                │ │ │
│  │  │          LoadTasks { tag } => load_tasks(tag).await             │ │ │
│  │  │          ComputeWaves { tag } => compute_waves(tag).await       │ │ │
│  │  │          StartSwarm { ... } => run_swarm(...).await             │ │ │
│  │  │          StopSwarm => stop_swarm().await                        │ │ │
│  │  │          CompleteTask { id } => complete_task(id).await         │ │ │
│  │  │          BlockTask { id } => block_task(id).await               │ │ │
│  │  │      }                                                          │ │ │
│  │  │  }                                                              │ │ │
│  │  └─────────────────────────────────────────────────────────────────┘ │ │
│  │                       │                                              │ │
│  │         event_tx.send(ScudEvent::*)                                 │ │
│  │                       │                                              │ │
│  └───────────────────────┼──────────────────────────────────────────────┘ │
│                          │                                                │
└──────────────────────────┼────────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                            SCUD CLI                                       │
│                                                                           │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
│  │  scud list       │  │  scud waves      │  │  scud swarm      │       │
│  │  --json          │  │  --json          │  │  --json-events   │       │
│  │  [--tag TAG]     │  │  --tag TAG       │  │  --tag TAG       │       │
│  │                  │  │                  │  │  --harness X     │       │
│  │  → JSON array    │  │  → Wave array    │  │  --round-size N  │       │
│  │    of tasks      │  │    of task IDs   │  │  → Event stream  │       │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘       │
│                                                                           │
│  ┌──────────────────┐  ┌──────────────────┐                              │
│  │  scud set-status │  │  scud set-status │                              │
│  │  <id> done       │  │  <id> blocked    │                              │
│  └──────────────────┘  └──────────────────┘                              │
│                                                                           │
│                    .scud/tasks/*.scg                                      │
│                    (Task storage)                                         │
│                                                                           │
└──────────────────────────────────────────────────────────────────────────┘
```

### Command Flow: Load Tasks

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   User Action   │     │   GUI Update    │     │   ScudBridge    │
│   (UI Thread)   │     │   (UI Thread)   │     │ (Background)    │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │  Click "Refresh"      │                       │
         │──────────────────────▶│                       │
         │                       │                       │
         │                       │  Message::RefreshTasks│
         │                       │───────────────────────│
         │                       │                       │
         │                       │  Task::perform()      │
         │                       │  (send LoadTasks cmd) │
         │                       │──────────────────────▶│
         │                       │                       │
         │                       │                       │  Execute:
         │                       │                       │  scud list --json
         │                       │                       │  [--tag TAG]
         │                       │                       │
         │                       │                       │  Parse JSON output
         │                       │                       │  as Vec<ScudJsonTask>
         │                       │                       │
         │                       │  ScudEvent::          │
         │                       │  TasksLoaded(tasks)   │
         │                       │◀──────────────────────│
         │                       │                       │
         │                       │  Subscription yields  │
         │                       │  Message::ScudEvent   │
         │                       │                       │
         │                       │  update() handles     │
         │                       │  state.tasks = tasks  │
         │                       │  state.waves = [tasks]│
         │                       │                       │
         │  view() re-renders    │                       │
         │◀──────────────────────│                       │
         │                       │                       │
```

### Command Flow: Swarm Execution

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   GUI       │    │  ScudBridge │    │  SCUD CLI   │    │   Harness   │
│  (UI)       │    │ (Background)│    │  (swarm)    │    │ (Agent)     │
└──────┬──────┘    └──────┬──────┘    └──────┬──────┘    └──────┬──────┘
       │                  │                  │                  │
       │ StartSwarm       │                  │                  │
       │─────────────────▶│                  │                  │
       │                  │                  │                  │
       │                  │ Spawn subprocess │                  │
       │                  │─────────────────▶│                  │
       │                  │                  │                  │
       │                  │                  │ SwarmStarted     │
       │                  │◀─────────────────│                  │
       │ SwarmStarted     │                  │                  │
       │◀─────────────────│                  │                  │
       │                  │                  │                  │
       │                  │                  │ WaveStarted      │
       │                  │◀─────────────────│                  │
       │ WaveStarted      │                  │                  │
       │◀─────────────────│                  │                  │
       │                  │                  │                  │
       │                  │                  │ TaskStarted      │
       │                  │◀─────────────────│─────────────────▶│
       │ TaskStarted      │                  │                  │
       │◀─────────────────│                  │                  │ Task
       │                  │                  │                  │ runs
       │                  │                  │ TaskOutput       │
       │                  │◀─────────────────│◀─────────────────│
       │ TaskOutput       │                  │                  │
       │◀─────────────────│                  │ (streaming)      │
       │                  │                  │                  │
       │                  │                  │ TaskCompleted    │
       │                  │◀─────────────────│◀─────────────────│
       │ TaskCompleted    │                  │                  │
       │◀─────────────────│                  │                  │
       │                  │                  │                  │
       │                  │                  │ ValidationStarted│
       │                  │◀─────────────────│                  │
       │ ValidationStart  │                  │                  │
       │◀─────────────────│                  │ (run build/test) │
       │                  │                  │                  │
       │                  │                  │ ValidationDone   │
       │                  │◀─────────────────│                  │
       │ ValidationDone   │                  │                  │
       │◀─────────────────│                  │                  │
       │                  │                  │                  │
       │                  │                  │ WaveCompleted    │
       │                  │◀─────────────────│                  │
       │ WaveCompleted    │                  │                  │
       │◀─────────────────│                  │                  │
       │                  │                  │                  │
       │                  │                  │ ... (more waves) │
       │                  │                  │                  │
       │                  │                  │ SwarmCompleted   │
       │                  │◀─────────────────│                  │
       │ SwarmCompleted   │                  │                  │
       │◀─────────────────│                  │                  │
       │                  │                  │                  │
```

---

## Integration with Iced's Event System

### Subscription Pattern

The GUI uses Iced's `Subscription` mechanism to receive events from the ScudBridge without polling:

```rust
/// Wrapper for ScudEvent receiver that implements Hash for Iced subscriptions
struct ScudEventReceiver(Arc<TokioMutex<Option<mpsc::Receiver<ScudEvent>>>>);

impl std::hash::Hash for ScudEventReceiver {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use a fixed hash since we only have one receiver
        "scud-event-receiver".hash(state);
    }
}

fn subscription(&self) -> Subscription<Message> {
    let rx = self.scud_event_rx.clone();
    Subscription::run_with(ScudEventReceiver(rx), |ScudEventReceiver(rx)| {
        let rx = rx.clone();
        async_stream::stream! {
            // Take the receiver from the mutex (only happens once)
            let mut receiver = {
                let mut guard = rx.lock().await;
                guard.take()
            };

            if let Some(ref mut rx) = receiver {
                while let Some(event) = rx.recv().await {
                    yield Message::ScudEvent(event);
                }
            }
        }
    })
}
```

**Key Design Decisions**:

1. **Arc<TokioMutex<Option<...>>>**: Iced requires `Hash` on subscription identifiers. Since `Receiver<T>` doesn't implement `Hash`, we wrap it in a custom struct with a fixed hash value.

2. **Option + take()**: The receiver can only be consumed once. The `Option` wrapper allows us to safely take ownership in the subscription's async stream.

3. **async_stream**: We use `async_stream::stream!` to create a stream that yields messages as they arrive from the channel.

### Message Dispatch

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        // Commands sent to ScudBridge
        Message::LoadTasksViaScud { tag } => {
            if let Some(ref tx) = self.scud_command_tx {
                let tx = tx.clone();
                return Task::perform(
                    async move {
                        let _ = tx.send(ScudCommand::LoadTasks { tag }).await;
                    },
                    |_| Message::Tick,
                );
            }
            Task::none()
        }

        // Events received from ScudBridge
        Message::ScudEvent(event) => {
            match event {
                ScudEvent::TasksLoaded(tasks) => {
                    self.state.tasks = tasks.clone();
                    self.state.waves = vec![tasks];
                }
                ScudEvent::SwarmStarted { tag, total_waves } => {
                    self.state.agent_status = AgentStatus::Running;
                    self.state.output_buffer.push_str(&format!(
                        "Swarm started for tag '{}' with {} waves\n",
                        tag, total_waves
                    ));
                }
                // ... handle other events
            }
            Task::none()
        }
    }
}
```

---

## Subprocess Communication

### SCUD CLI Commands Used

| Operation | Command | Output Format |
|-----------|---------|---------------|
| Load tasks | `scud list --json [--tag TAG]` | JSON array of tasks |
| Compute waves | `scud waves --json --tag TAG` | JSON array of arrays |
| Start swarm | `scud swarm --tag TAG --harness H --round-size N --json-events` | NDJSON event stream |
| Complete task | `scud set-status ID done` | Exit code |
| Block task | `scud set-status ID blocked` | Exit code |

### JSON Parsing Strategy

1. **Array format** (list, waves): Parse full stdout as JSON array
2. **NDJSON format** (swarm events): Parse each line individually
3. **Fallback**: If JSON parsing fails for `list`, try line-by-line parsing

```rust
async fn load_tasks(&self, tag: Option<String>) {
    match Command::new("scud").args(&args).output().await {
        Ok(output) => {
            if output.status.success() {
                // First try: parse as JSON array
                match serde_json::from_slice::<Vec<ScudJsonTask>>(&output.stdout) {
                    Ok(tasks) => {
                        let _ = self.event_tx.send(ScudEvent::TasksLoaded(tasks)).await;
                    }
                    Err(e) => {
                        // Fallback: parse as newline-delimited JSON
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let tasks: Vec<TaskInfo> = stdout
                            .lines()
                            .filter_map(|line| serde_json::from_str(line).ok())
                            .collect();
                        // ...
                    }
                }
            }
        }
    }
}
```

### Streaming Output

For swarm execution, stdout is read line-by-line as events occur:

```rust
async fn run_swarm(&mut self, tag: &str, harness: &str, round_size: usize) {
    let mut child = Command::new("scud")
        .args(&["swarm", "--tag", tag, "--json-events", ...])
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            // Try to parse as JSON event
            if let Ok(event) = serde_json::from_str::<ScudJsonEvent>(&line) {
                let scud_event: ScudEvent = event.into();
                event_tx.send(scud_event).await;
            } else {
                // Non-JSON line - send as generic output
                event_tx.send(ScudEvent::Output(line)).await;
            }
        }
    }
}
```

---

## Threading Model

```
┌────────────────────────────────────────────────────────────────┐
│                         Main Thread                             │
│                     (Iced Event Loop)                           │
│                                                                 │
│  - Handles user input                                           │
│  - Renders UI via view()                                        │
│  - Processes messages via update()                              │
│  - Owns mpsc::Sender<ScudCommand>                              │
│  - Owns Arc<TokioMutex<Option<Receiver<ScudEvent>>>>           │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          │ std::thread::spawn()
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                      Background Thread                           │
│                  (Dedicated Tokio Runtime)                       │
│                                                                  │
│  - Runs ScudBridge::run() event loop                            │
│  - Owns mpsc::Receiver<ScudCommand>                             │
│  - Owns mpsc::Sender<ScudEvent>                                 │
│  - Spawns SCUD subprocesses                                     │
│  - Parses JSON output                                           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Key Points**:

1. **Separate OS Thread**: The ScudBridge runs on its own OS thread with a dedicated Tokio runtime to avoid blocking the Iced event loop.

2. **Channel Capacity**: Both command and event channels have a capacity of 100 messages.

3. **Process Cleanup**: The `swarm_handle` field stores the child process handle for potential cancellation via `stop_swarm()`.

---

## Error Handling

| Error Type | Handling |
|------------|----------|
| SCUD command not found | `ScudEvent::Error("Failed to run scud: ...")` |
| SCUD exits with error | `ScudEvent::Error("scud X failed: <stderr>")` |
| JSON parse failure | Try NDJSON fallback, then `ScudEvent::Error` |
| Channel send failure | Log warning, break loop |
| Channel closed | Bridge shuts down gracefully |

---

## Configuration

### Channel Capacity

```rust
let (event_tx, event_rx) = mpsc::channel(100);  // Events to GUI
let (command_tx, command_rx) = mpsc::channel(100);  // Commands from GUI
```

### Default Values

| Setting | Default | Notes |
|---------|---------|-------|
| Harness | `"claude-code"` | Used in GUI's Start Swarm button |
| Round size | `3` | Tasks per round in swarm |
| Tag | `"refactor"` | Default when no tag selected |

---

## Testing Strategy

### Unit Tests (scud_bridge.rs)

- JSON event parsing for all event types
- Task info conversion from JSON
- ScudEvent conversion from ScudJsonEvent

### Integration Tests (main.rs)

- Full workflow: load → start → pause → resume → complete
- Error handling: network failures, parse errors
- UI interactions: button clicks, view switching
- State transitions: Idle → Running → Paused → Idle

### Test App Fixture

```rust
fn test_app() -> DescartesGui {
    DescartesGui {
        view: ViewMode::Waves,
        state: AppState::default(),
        control_tx: None,
        scud_command_tx: None,  // No actual bridge
        scud_event_rx: Arc::new(TokioMutex::new(None)),
        error: None,
    }
}
```

---

## Future Enhancements

1. **ZMQ PUB/SUB**: Replace subprocess stdout streaming with ZMQ sockets for lower latency and remote monitoring capability.

2. **Persistent Connections**: Keep SCUD processes running for faster command execution.

3. **Event Buffering**: Implement event history for scroll-back in output view.

4. **Progress Indicators**: Show estimated completion based on wave progress.

5. **Error Recovery**: Auto-retry failed commands with exponential backoff.

---

## References

- Implementation: `descartes-gui/src/scud_bridge.rs`
- GUI Integration: `descartes-gui/src/main.rs`
- JSON Format Spec: `.scud/docs/json-event-format.md`
- Refactor Plan: `thoughts/shared/plans/2026-01-19-descartes-scud-integration-refactor.md`
- Iced Framework: https://iced.rs/
