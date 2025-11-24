# Phase 1 Implementation Guide

**Reference**: See `phase1-architecture.md` for comprehensive technical design.

This guide provides task-specific implementation details for all Phase 1 tasks.

---

## Task 1: Initialize Rust Workspace (Complexity: 3)

### Files to Create:
```
Cargo.toml
core/Cargo.toml
core/src/lib.rs
cli/Cargo.toml
cli/src/main.rs
agent-runner/Cargo.toml
agent-runner/src/lib.rs
gui/Cargo.toml (placeholder for Phase 3)
.github/workflows/ci.yml
```

### Cargo.toml (Root - Workspace):
```toml
[workspace]
members = ["core", "cli", "agent-runner", "gui"]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite"] }
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
glob = "0.3"
```

### CI/CD (.github/workflows/ci.yml):
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --all-features
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
```

**Testing**: Verify `cargo build` succeeds, workspace members recognized.

---

## Task 2: Define Core Traits (Complexity: 2)

### File: `core/src/traits.rs`

```rust
use async_trait::async_trait;
use crate::{Session, Event, Task, AgentEvent, TaskStatus, SessionStatus};
use std::error::Error;

/// AgentRunner: Spawn and manage child processes (agents)
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Spawn a new agent process
    async fn spawn(&self, config: AgentConfig) -> Result<Session, Box<dyn Error>>;

    /// Write input to agent's stdin
    async fn write_input(&mut self, data: &str) -> Result<(), Box<dyn Error>>;

    /// Get event stream receiver
    fn event_stream(&self) -> tokio::sync::mpsc::Receiver<AgentEvent>;

    /// Wait for agent to complete
    async fn wait(&mut self) -> Result<i32, Box<dyn Error>>;
}

/// StateStore: Persist events, sessions, tasks to SQLite
#[async_trait]
pub trait StateStore: Send + Sync {
    // Event operations (hot path)
    async fn log_event(&self, event: Event) -> Result<(), Box<dyn Error>>;
    async fn get_events(&self, session_id: &str, limit: usize) -> Result<Vec<Event>, Box<dyn Error>>;

    // Session operations
    async fn create_session(&self, session: Session) -> Result<String, Box<dyn Error>>;
    async fn update_session(&self, id: &str, status: SessionStatus, exit_code: Option<i32>) -> Result<(), Box<dyn Error>>;

    // Task operations
    async fn create_task(&self, task: Task) -> Result<i64, Box<dyn Error>>;
    async fn update_task(&self, id: i64, status: TaskStatus) -> Result<(), Box<dyn Error>>;
    async fn list_tasks(&self, status: Option<TaskStatus>) -> Result<Vec<Task>, Box<dyn Error>>;
}

/// ContextSyncer: Read files for context injection
#[async_trait]
pub trait ContextSyncer: Send + Sync {
    /// Read files matching glob pattern
    async fn read_files(&self, pattern: &str) -> Result<Vec<FileContent>, Box<dyn Error>>;

    /// Read single file
    async fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>>;
}

pub struct FileContent {
    pub path: String,
    pub content: String,
}

pub struct AgentConfig {
    pub model: Option<String>,
    pub prompt: String,
    pub stdin_input: Option<String>,
}
```

**Files to Create**: `core/src/lib.rs`, `core/src/traits.rs`, `core/src/events.rs`, `core/src/session.rs`, `core/src/task.rs`, `core/src/errors.rs`

**Reference**: See architecture doc section 3 (Data Models) for full model definitions.

**Testing**: Compile-time verification only (no runtime tests needed for traits).

---

## Task 3: Implement LocalProcessRunner (Complexity: 5)

### File: `agent-runner/src/local.rs`

**Key Implementation Points**:

1. **Struct Definition** (Task 3.1):
```rust
pub struct LocalProcessRunner {
    child: Option<tokio::process::Child>,
    stdin: Option<tokio::process::ChildStdin>,
    event_tx: tokio::sync::mpsc::Sender<AgentEvent>,
    event_rx: Option<tokio::sync::mpsc::Receiver<AgentEvent>>,
}
```

2. **Process Spawning** (Task 3.2):
```rust
async fn spawn_process(&mut self, command: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut child = tokio::process::Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    self.stdin = child.stdin.take();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Spawn handlers for stdout/stderr
    self.spawn_output_handler(stdout, stderr);

    self.child = Some(child);
    Ok(())
}
```

3. **I/O Streaming** (Task 3.3):
- Use `tokio::io::AsyncBufReadExt` to read lines from stdout/stderr
- Send lines to `event_tx` channel for processing
- Use `tokio::spawn` for concurrent read tasks

4. **Signal Handling** (Task 3.4):
```rust
// agent-runner/src/signals.rs
use tokio::signal::unix::{signal, SignalKind};

pub async fn setup_signal_handlers(child_pid: u32) {
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {
                unsafe { libc::kill(child_pid as i32, libc::SIGINT); }
            }
            _ = sigterm.recv() => {
                unsafe { libc::kill(child_pid as i32, libc::SIGTERM); }
            }
        }
    });
}
```

**Dependencies**: `tokio`, `async-trait`, `core` crate

**Testing**:
- Unit test: Spawn `echo "hello"`, verify output received
- Integration test: Spawn long-running process, send SIGINT, verify graceful exit

**Reference**: Architecture doc section 4.C (LocalProcessRunner)

---

## Task 4: Implement ClaudeCodeAdapter (Complexity: 5)

### File: `agent-runner/src/claude_adapter.rs`

**Key Implementation Points**:

1. **Structure** (Task 4.1):
```rust
pub struct ClaudeCodeAdapter {
    runner: LocalProcessRunner,
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self { runner: LocalProcessRunner::new() }
    }
}
```

2. **JSON Streaming Parser** (Task 4.2):
```rust
async fn parse_stream_json(&self, line: String) -> Option<AgentEvent> {
    let parsed: serde_json::Value = serde_json::from_str(&line).ok()?;

    match parsed.get("type")?.as_str()? {
        "tool_use" => Some(AgentEvent::ToolUse {
            tool: parsed["tool"].as_str()?.to_string(),
            args: parsed["args"].clone(),
        }),
        "text" => Some(AgentEvent::TextOutput {
            content: parsed["content"].as_str()?.to_string(),
        }),
        "error" => Some(AgentEvent::Error {
            message: parsed["message"].as_str()?.to_string(),
        }),
        _ => None,
    }
}
```

3. **Event Mapping** (Task 4.3):
- Map Claude JSON events to `AgentEvent` enum (defined in `core/src/events.rs`)
- Handle unknown event types gracefully (log warning, continue)

4. **Testing** (Task 4.4):
- Create `tests/fixtures/mock-claude.sh` that outputs sample JSON
- Test JSON parsing with malformed input
- Integration test with mock Claude CLI

**Command to wrap**:
```bash
claude --output-format=stream-json "<prompt>"
```

**Reference**: Architecture doc section 4.D (ClaudeCodeAdapter)

---

## Task 5: Design SQLite Schema (Complexity: 5)

### Files:
- `migrations/001_create_events.sql`
- `migrations/002_create_sessions.sql`
- `migrations/003_create_tasks.sql`

**Migration 001** (Task 5.1, 5.3):
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_events_session ON events(session_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);
```

**Migration 002**:
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    model TEXT,
    prompt TEXT,
    status TEXT NOT NULL,
    exit_code INTEGER,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
```

**Migration 003**:
```sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_session ON tasks(session_id);
```

**Setup** (Task 5.2):
```bash
# In agent-runner crate root
sqlx database create
sqlx migrate add create_events
sqlx migrate add create_sessions
sqlx migrate add create_tasks
sqlx migrate run
```

**Testing** (Task 5.4):
- Run migrations in test database
- Verify indexes exist: `PRAGMA index_list(events);`
- Benchmark insert performance (target < 10ms per event)

**Reference**: Architecture doc section 3 (Data Models)

---

## Task 6: Implement SqliteStore (Complexity: 8)

### File: `agent-runner/src/sqlite_store.rs`

**Key Implementation Points**:

1. **Setup** (Task 6.1):
```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}", db_path))
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), Box<dyn Error>> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}
```

2. **StateStore Trait Implementation** (Task 6.2):
```rust
#[async_trait]
impl StateStore for SqliteStore {
    async fn log_event(&self, event: Event) -> Result<(), Box<dyn Error>> {
        let payload_json = serde_json::to_string(&event.payload)?;

        sqlx::query!(
            "INSERT INTO events (session_id, event_type, timestamp, payload) VALUES (?, ?, ?, ?)",
            event.session_id,
            event.event_type,
            event.timestamp.to_rfc3339(),
            payload_json
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Implement other methods...
}
```

3. **Hot Path Logger** (Task 6.3):
- Use unbounded `tokio::sync::mpsc` channel for event buffering
- Spawn background task to consume events and write to DB
- Target latency: < 10ms per event

4. **All Persistence Methods** (Task 6.4):
- Implement all CRUD operations for sessions and tasks
- Use transactions for multi-step operations
- Proper error handling with `thiserror`

5. **Testing** (Task 6.5):
- Unit tests with `:memory:` database
- Integration tests with temp database file
- Concurrency test: Multiple tasks writing events simultaneously
- Data integrity test: Verify foreign keys enforced

**Dependencies**: `sqlx`, `tokio`, `serde_json`, `chrono`

**Reference**: Architecture doc section 4.E (SqliteStore)

---

## Task 7: Implement Basic CLI Commands (Complexity: 5)

### File: `cli/src/main.rs`

**Structure** (Task 7.1, 7.2):
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "descartes")]
#[command(about = "AI agent orchestration")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Spawn { prompt: String, #[arg(long)] model: Option<String> },
    Logs { #[arg(long)] session: Option<String>, #[arg(short, long)] follow: bool },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cmd_init().await?,
        Commands::Spawn { prompt, model } => cmd_spawn(&prompt, model).await?,
        Commands::Logs { session, follow } => cmd_logs(session, follow).await?,
    }

    Ok(())
}
```

**`descartes init`** (Task 7.2):
```rust
async fn cmd_init() -> anyhow::Result<()> {
    let descartes_dir = std::path::Path::new(".descartes");

    if descartes_dir.exists() {
        println!("✓ .descartes directory already exists");
    } else {
        tokio::fs::create_dir(descartes_dir).await?;
        println!("✓ Created .descartes directory");
    }

    let db_path = descartes_dir.join("state.db");
    let store = SqliteStore::new(db_path.to_str().unwrap()).await?;
    store.run_migrations().await?;

    println!("✓ Initialized database: .descartes/state.db");
    Ok(())
}
```

**`descartes spawn`** (Task 7.3):
```rust
async fn cmd_spawn(prompt: &str, model: Option<String>) -> anyhow::Result<()> {
    let db_path = ".descartes/state.db";
    let store = SqliteStore::new(db_path).await?;

    let session = Session {
        id: uuid::Uuid::new_v4().to_string(),
        agent_type: "claude-code".to_string(),
        model,
        prompt: prompt.to_string(),
        status: SessionStatus::Running,
        exit_code: None,
        started_at: chrono::Utc::now(),
        completed_at: None,
    };

    store.create_session(session.clone()).await?;

    let mut adapter = ClaudeCodeAdapter::new();
    let config = AgentConfig {
        model,
        prompt: prompt.to_string(),
        stdin_input: None,
    };

    adapter.spawn(config).await?;
    let exit_code = adapter.wait().await?;

    store.update_session(&session.id, SessionStatus::Completed, Some(exit_code)).await?;

    Ok(())
}
```

**`descartes logs`** (Task 7.4):
```rust
async fn cmd_logs(session: Option<String>, follow: bool) -> anyhow::Result<()> {
    let db_path = ".descartes/state.db";
    let store = SqliteStore::new(db_path).await?;

    let session_id = session.unwrap_or_else(|| {
        // Get latest session ID from DB
        "latest".to_string()
    });

    let events = store.get_events(&session_id, 100).await?;

    for event in events {
        println!("{} | {} | {} | {:?}",
            event.session_id,
            event.timestamp.to_rfc3339(),
            event.event_type,
            event.payload
        );
    }

    Ok(())
}
```

**Testing**:
- Integration test: `descartes init` creates directory and DB
- Integration test: `descartes spawn "hello"` executes and logs events
- Integration test: `descartes logs` displays events

**Dependencies**: All core components (tasks 2, 3, 4, 6)

---

## Task 8: Implement CLI Pipe Support (Complexity: 5)

### File: `cli/src/main.rs` (modify)

**Detect piped input** (Task 8.1, 8.2):
```rust
use tokio::io::{self, AsyncReadExt};

async fn read_stdin_if_piped() -> anyhow::Result<Option<String>> {
    // Check if stdin is a TTY
    if atty::is(atty::Stream::Stdin) {
        return Ok(None);
    }

    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer).await?;

    Ok(Some(buffer))
}
```

**Pass to agent** (Task 8.3):
```rust
async fn cmd_spawn(prompt: &str, model: Option<String>) -> anyhow::Result<()> {
    let stdin_input = read_stdin_if_piped().await?;

    let config = AgentConfig {
        model,
        prompt: prompt.to_string(),
        stdin_input, // Pass piped input here
    };

    // ... rest of spawn logic
}
```

**Testing** (Task 8.4):
```bash
# Integration test
echo "Hello from pipe" | target/debug/descartes spawn "Summarize this"
```

**Dependencies**: `atty` crate for TTY detection

---

## Task 9: Implement Basic File Reading (Complexity: 5)

### File: `agent-runner/src/file_reader.rs`

**Interface** (Task 9.1):
```rust
pub struct FileReader;

#[async_trait]
impl ContextSyncer for FileReader {
    async fn read_files(&self, pattern: &str) -> Result<Vec<FileContent>, Box<dyn Error>> {
        // Implementation below
    }

    async fn read_file(&self, path: &str) -> Result<String, Box<dyn Error>> {
        Ok(tokio::fs::read_to_string(path).await?)
    }
}
```

**Glob Implementation** (Task 9.2):
```rust
use glob::glob;

async fn read_files(&self, pattern: &str) -> Result<Vec<FileContent>, Box<dyn Error>> {
    let mut results = Vec::new();
    let mut count = 0;

    for entry in glob(pattern)? {
        let path = entry?;

        if !path.is_file() {
            continue;
        }

        let content = tokio::fs::read_to_string(&path).await?;
        results.push(FileContent {
            path: path.to_string_lossy().to_string(),
            content,
        });

        count += 1;
        if count >= 100 {
            break; // Safety limit
        }
    }

    Ok(results)
}
```

**Integration** (Task 9.3):
- FileReader implements ContextSyncer trait
- Can be passed to agents for context injection

**Testing** (Task 9.4):
- Unit test: Read single file
- Unit test: Glob pattern matching `*.md`
- Integration test: Verify 100-file limit
- Test error handling for invalid paths

**Dependencies**: `glob`, `tokio::fs`

---

## Implementation Order

Based on dependencies and architecture phases (from section 10 of architecture doc):

### Phase 1.1: Foundation (2-4 hours)
1. Task 1.1: Create workspace structure
2. Task 1.2: Add dependencies
3. Task 2: Define core traits
4. Task 5.1-5.3: Design and create SQLite schema

### Phase 1.2: Process Management (4-6 hours)
5. Task 3.1-3.4: LocalProcessRunner
6. Task 4.1-4.3: ClaudeCodeAdapter (without tests initially)

### Phase 1.3: Persistence (3-4 hours)
7. Task 6.1-6.4: SqliteStore implementation

### Phase 1.4: CLI (3-4 hours)
8. Task 7.1-7.4: Basic CLI commands
9. Task 8.1-8.3: Pipe support

### Phase 1.5: Context Engine (2-3 hours)
10. Task 9.1-9.3: File reading

### Phase 1.6: Testing & Polish (4-6 hours)
11. Task 1.3: CI/CD pipeline
12. All `.4` and `.5` test tasks
13. Integration testing

**Total Estimated Time**: 18-27 hours

---

## Quick Reference: File Structure

```
descartes/
├── Cargo.toml (workspace)
├── .github/
│   └── workflows/
│       └── ci.yml
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs
│       ├── events.rs
│       ├── session.rs
│       ├── task.rs
│       └── errors.rs
├── agent-runner/
│   ├── Cargo.toml
│   ├── migrations/
│   │   ├── 001_create_events.sql
│   │   ├── 002_create_sessions.sql
│   │   └── 003_create_tasks.sql
│   └── src/
│       ├── lib.rs
│       ├── local.rs (LocalProcessRunner)
│       ├── claude_adapter.rs
│       ├── sqlite_store.rs
│       ├── file_reader.rs
│       └── signals.rs
├── cli/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── gui/ (placeholder for Phase 3)
│   └── Cargo.toml
└── tests/
    ├── integration/
    │   ├── cli_init.rs
    │   ├── cli_spawn.rs
    │   └── cli_logs.rs
    └── fixtures/
        └── mock-claude.sh
```

---

**End of Implementation Guide**
