# Architecture Document: Descartes Phase 1 - Foundation

**Epic Tag:** phase1
**Date:** 2025-11-23
**Architect:** Claude (Architect Agent)
**Status:** Final

## 1. System Overview

Descartes is a local-first agent orchestration system that wraps AI CLI tools (starting with Claude Code) to provide event logging, state persistence, and context management. Phase 1 establishes the foundational architecture for process management, persistence, and CLI interaction.

**Core Value Proposition:**
- **Event Transparency**: Every agent action logged to local SQLite
- **Context Control**: Explicit file injection via glob patterns
- **Process Safety**: Graceful shutdown and signal handling
- **Local-First**: No cloud dependencies, full data ownership

**Architecture Diagram:**
```
┌─────────────────────────────────────────────────────────────┐
│                       CLI Layer (clap)                       │
│  descartes init | descartes spawn | descartes logs          │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    Core Traits (core crate)                  │
│  AgentRunner │ StateStore │ ContextSyncer                   │
└─────┬──────────────┬─────────────────┬──────────────────────┘
      │              │                 │
┌─────▼──────┐  ┌───▼──────────┐  ┌───▼──────────┐
│LocalProcess│  │ SqliteStore  │  │ FileReader   │
│  Runner    │  │              │  │  (glob)      │
│  (tokio)   │  │ (sqlx pool)  │  │              │
└─────┬──────┘  └───┬──────────┘  └──────────────┘
      │             │
      │         ┌───▼──────────┐
      │         │   SQLite DB  │
      │         │ .descartes/  │
      │         │  state.db    │
      │         └──────────────┘
      │
┌─────▼──────────────────┐
│  ClaudeCodeAdapter     │
│  (wraps claude CLI)    │
│  JSON stream parser    │
└────────┬───────────────┘
         │
    ┌────▼────┐
    │ claude  │ (external process)
    └─────────┘
```

**Key Components:**
- **CLI**: User-facing commands using clap
- **Core Traits**: Abstract interfaces for dependency injection
- **LocalProcessRunner**: Tokio-based process spawning and I/O streaming
- **ClaudeCodeAdapter**: Claude CLI wrapper with JSON stream parsing
- **SqliteStore**: Event persistence with hot path optimization
- **FileReader**: Context injection via glob patterns

## 2. Technology Stack

**Languages:**
- Rust (stable channel, edition 2021)

**Frameworks & Libraries:**

**Async Runtime:**
- `tokio = { version = "1", features = ["full"] }` - Async runtime for I/O, process management
- Rationale: Industry standard, excellent process management API, full ecosystem support

**Database:**
- `sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite"] }` - Async SQLite driver
- `sqlx-cli` - Migration management
- Rationale: Compile-time query checking, async support, type-safe, excellent migrations

**CLI:**
- `clap = { version = "4", features = ["derive"] }` - Command-line argument parsing
- Rationale: Derive macros for ergonomic API, excellent help generation

**Serialization:**
- `serde = { version = "1", features = ["derive"] }` - Data serialization
- `serde_json = "1"` - JSON parsing for Claude stream output
- Rationale: Standard ecosystem choice, excellent performance

**Logging & Tracing:**
- `tracing = "0.1"` - Structured logging
- `tracing-subscriber = "0.3"` - Log output management
- Rationale: Better than `log` crate, structured fields, async-aware

**Error Handling:**
- `thiserror = "1"` - Library error types (core, agent-runner crates)
- `anyhow = "1"` - CLI error handling (cli crate)
- Rationale: thiserror for typed errors in libraries, anyhow for ergonomic error propagation in binaries

**File Operations:**
- `glob = "0.3"` - Glob pattern matching
- `tokio::fs` - Async file I/O
- Rationale: Standard glob implementation, async fs avoids blocking executor

**Testing:**
- Built-in `#[tokio::test]` for async tests
- `tempfile = "3"` - Temporary test databases
- `assert_cmd = "2"` - CLI integration tests
- `predicates = "3"` - Test assertions

**Infrastructure:**
- GitHub Actions for CI/CD
- `cargo-nextest` for faster test execution (optional)

**Technology Decisions:**

**Decision 1: Full Async Architecture**
- **Choice**: Use tokio throughout, including CLI entry point
- **Why**: Process I/O, SQLite operations, and file reading all benefit from async. Consistent model simplifies implementation.
- **Trade-off**: Slightly more complex than sync, but avoids blocking on I/O

**Decision 2: SQLite over Embedded KV**
- **Choice**: SQLite with sqlx
- **Why**: Structured queries for events/tasks, battle-tested, excellent tooling, migrations support
- **Trade-off**: Slightly heavier than sled/redb, but more flexible for complex queries in future phases

**Decision 3: Blocking Process Mode First**
- **Choice**: `descartes spawn` blocks until agent completes
- **Why**: Simpler implementation, easier signal handling, natural UX for piping
- **Trade-off**: Can't run multiple agents concurrently (add in Phase 2 if needed)

**Decision 4: Mock Claude CLI for Tests**
- **Choice**: Create test helper that simulates `claude` JSON output
- **Why**: CI doesn't have Claude API keys, tests must be deterministic
- **Trade-off**: Need to maintain mock, but enables reliable testing

## 3. Data Models

### Database Schema

**SQLite Schema (sqlx migrations):**

```sql
-- Migration 001: Create events table (Hot Path)
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_type TEXT NOT NULL,  -- 'tool_use', 'text', 'error', etc.
    timestamp TEXT NOT NULL,   -- ISO 8601
    payload TEXT NOT NULL,     -- JSON blob
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_events_session ON events(session_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);

-- Migration 002: Create sessions table
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,        -- UUID v4
    agent_type TEXT NOT NULL,   -- 'claude-code', 'openai', etc.
    model TEXT,                 -- 'claude-sonnet-4', etc.
    prompt TEXT,                -- Initial prompt
    status TEXT NOT NULL,       -- 'running', 'completed', 'failed'
    exit_code INTEGER,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Migration 003: Create tasks table (Global Task Manager)
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,            -- NULL for manual tasks
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,       -- 'pending', 'in_progress', 'completed', 'failed'
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_session ON tasks(session_id);
```

### Rust Data Models

```rust
// core/src/events.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    ToolUse { tool: String, args: serde_json::Value },
    TextOutput { content: String },
    Error { message: String },
    ProcessStart { pid: u32 },
    ProcessExit { code: i32 },
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: Option<i64>,
    pub session_id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub payload: AgentEvent,
}

// core/src/session.rs
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,           // UUID
    pub agent_type: String,
    pub model: Option<String>,
    pub prompt: String,
    pub status: SessionStatus,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
}

// core/src/task.rs
#[derive(Debug, Clone)]
pub struct Task {
    pub id: Option<i64>,
    pub session_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
```

### API Contracts (Internal Traits)

```rust
// core/src/traits.rs

#[async_trait::async_trait]
pub trait AgentRunner: Send + Sync {
    /// Spawn an agent process
    async fn spawn(
        &self,
        config: AgentConfig,
    ) -> Result<Session, AgentError>;

    /// Write input to running agent
    async fn write_input(&mut self, data: &str) -> Result<(), AgentError>;

    /// Stream events from agent
    fn event_stream(&self) -> tokio::sync::mpsc::Receiver<AgentEvent>;

    /// Wait for agent to complete
    async fn wait(&mut self) -> Result<i32, AgentError>;
}

#[async_trait::async_trait]
pub trait StateStore: Send + Sync {
    /// Log an event (hot path - must be fast)
    async fn log_event(&self, event: Event) -> Result<(), StoreError>;

    /// Create session
    async fn create_session(&self, session: Session) -> Result<String, StoreError>;

    /// Update session status
    async fn update_session(&self, id: &str, status: SessionStatus, exit_code: Option<i32>) -> Result<(), StoreError>;

    /// Query events
    async fn get_events(&self, session_id: &str, limit: usize) -> Result<Vec<Event>, StoreError>;

    /// Task CRUD
    async fn create_task(&self, task: Task) -> Result<i64, StoreError>;
    async fn update_task(&self, id: i64, status: TaskStatus) -> Result<(), StoreError>;
    async fn list_tasks(&self, status: Option<TaskStatus>) -> Result<Vec<Task>, StoreError>;
}

#[async_trait::async_trait]
pub trait ContextSyncer: Send + Sync {
    /// Read files matching glob pattern
    async fn read_files(&self, pattern: &str) -> Result<Vec<FileContent>, ContextError>;

    /// Get file content
    async fn read_file(&self, path: &str) -> Result<String, ContextError>;
}

pub struct FileContent {
    pub path: String,
    pub content: String,
}
```

### Data Flows

**Agent Spawn Flow:**
```
User: descartes spawn "Hello"
  ↓
CLI parses command
  ↓
Create Session in DB → session_id
  ↓
LocalProcessRunner spawns child process
  ↓
ClaudeCodeAdapter wraps claude CLI
  ↓
JSON stream parser → AgentEvents
  ↓
Events logged to DB (hot path)
  ↓
Stream stdout to user terminal
  ↓
Process exits → Update session status
```

**Event Logging (Hot Path):**
```
AgentEvent received
  ↓
Convert to Event struct
  ↓
SqliteStore::log_event()
  ↓
INSERT via sqlx (< 10ms target)
  ↓
Continue processing (non-blocking)
```

## 4. Component Architecture

### Component A: CLI (`cli` crate)
**Responsibility:** User-facing command-line interface
**Interfaces:**
- Entry point: `main()` with tokio runtime
- Commands: `init`, `spawn`, `logs`
- Uses: clap for parsing, calls into core traits

**Dependencies:**
- `core` crate for traits and models
- `agent-runner` crate for implementations
- `clap`, `anyhow`, `tokio`

**Implementation:**
```rust
// cli/src/main.rs
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
    /// Initialize .descartes directory and database
    Init,

    /// Spawn an AI agent
    Spawn {
        /// Prompt for the agent
        prompt: String,

        /// Model to use (default: claude-sonnet-4)
        #[arg(long)]
        model: Option<String>,
    },

    /// Tail event logs
    Logs {
        /// Session ID (default: latest)
        #[arg(long)]
        session: Option<String>,

        /// Follow mode
        #[arg(short, long)]
        follow: bool,
    },
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

### Component B: Core Traits (`core` crate)
**Responsibility:** Define abstract interfaces and core data models
**Interfaces:** Public traits (AgentRunner, StateStore, ContextSyncer)
**Dependencies:** `serde`, `tokio`, `async-trait`, `chrono`

**Files:**
- `src/traits.rs` - Trait definitions
- `src/events.rs` - Event models
- `src/session.rs` - Session models
- `src/task.rs` - Task models
- `src/errors.rs` - Error types with thiserror

### Component C: LocalProcessRunner (`agent-runner` crate)
**Responsibility:** Spawn and manage child processes with tokio
**Interfaces:** Implements `AgentRunner` trait
**Dependencies:** `tokio`, `core` crate

**Implementation Details:**
```rust
// agent-runner/src/local.rs
pub struct LocalProcessRunner {
    child: Option<tokio::process::Child>,
    stdin: Option<tokio::process::ChildStdin>,
    event_tx: tokio::sync::mpsc::Sender<AgentEvent>,
    event_rx: Option<tokio::sync::mpsc::Receiver<AgentEvent>>,
}

impl LocalProcessRunner {
    pub fn new() -> Self {
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        Self {
            child: None,
            stdin: None,
            event_tx,
            event_rx: Some(event_rx),
        }
    }

    async fn spawn_process(&mut self, command: &str, args: &[String]) -> Result<(), AgentError> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        self.stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Spawn tasks to stream stdout/stderr
        self.spawn_output_handler(stdout, stderr);

        self.child = Some(child);
        Ok(())
    }
}
```

**Signal Handling:**
```rust
// agent-runner/src/signals.rs
use tokio::signal;

pub async fn setup_signal_handlers(child_pid: u32) {
    tokio::spawn(async move {
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {
                // Forward SIGINT to child process
                unsafe {
                    libc::kill(child_pid as i32, libc::SIGINT);
                }
            }
            _ = sigterm.recv() => {
                // Forward SIGTERM to child process
                unsafe {
                    libc::kill(child_pid as i32, libc::SIGTERM);
                }
            }
        }
    });
}
```

### Component D: ClaudeCodeAdapter (`agent-runner` crate)
**Responsibility:** Wrap `claude` CLI and parse JSON stream output
**Interfaces:** Implements `AgentRunner` trait
**Dependencies:** `serde_json`, `core` crate, wraps `LocalProcessRunner`

**JSON Stream Parsing:**
```rust
// agent-runner/src/claude_adapter.rs
pub struct ClaudeCodeAdapter {
    runner: LocalProcessRunner,
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self {
            runner: LocalProcessRunner::new(),
        }
    }

    async fn parse_stream_json(&self, line: String) -> Option<AgentEvent> {
        // Claude outputs newline-delimited JSON when using --output-format=stream-json
        // Example: {"type":"tool_use","tool":"read","args":{"path":"..."}}

        let parsed: serde_json::Value = serde_json::from_str(&line).ok()?;

        match parsed.get("type")?.as_str()? {
            "tool_use" => {
                Some(AgentEvent::ToolUse {
                    tool: parsed["tool"].as_str()?.to_string(),
                    args: parsed["args"].clone(),
                })
            }
            "text" => {
                Some(AgentEvent::TextOutput {
                    content: parsed["content"].as_str()?.to_string(),
                })
            }
            "error" => {
                Some(AgentEvent::Error {
                    message: parsed["message"].as_str()?.to_string(),
                })
            }
            _ => None,
        }
    }
}
```

### Component E: SqliteStore (`agent-runner` crate)
**Responsibility:** Persist events, sessions, tasks to SQLite
**Interfaces:** Implements `StateStore` trait
**Dependencies:** `sqlx`, `core` crate

**Connection Pooling:**
```rust
// agent-runner/src/sqlite_store.rs
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new(db_path: &str) -> Result<Self, StoreError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)  // Conservative for SQLite (write serialization)
            .connect(&format!("sqlite://{}", db_path))
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), StoreError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl StateStore for SqliteStore {
    async fn log_event(&self, event: Event) -> Result<(), StoreError> {
        // Hot path - optimized for speed
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
}
```

### Component F: FileReader (`agent-runner` crate)
**Responsibility:** Read files with glob pattern support
**Interfaces:** Implements `ContextSyncer` trait
**Dependencies:** `glob`, `tokio::fs`, `core` crate

**Implementation:**
```rust
// agent-runner/src/file_reader.rs
use glob::glob;

pub struct FileReader;

#[async_trait::async_trait]
impl ContextSyncer for FileReader {
    async fn read_files(&self, pattern: &str) -> Result<Vec<FileContent>, ContextError> {
        let mut results = Vec::new();

        for entry in glob(pattern)? {
            let path = entry?;
            if path.is_file() {
                let content = tokio::fs::read_to_string(&path).await?;
                results.push(FileContent {
                    path: path.to_string_lossy().to_string(),
                    content,
                });
            }
        }

        Ok(results)
    }

    async fn read_file(&self, path: &str) -> Result<String, ContextError> {
        Ok(tokio::fs::read_to_string(path).await?)
    }
}
```

## 5. Integration Points

### External Process: `claude` CLI
**Purpose:** Execute AI agent tasks via Claude Code CLI
**Endpoints:**
- Command: `claude --output-format=stream-json <prompt>`
- JSON stream on stdout
- Exit code on completion

**Error Handling:**
- **Process spawn failure**: Return `AgentError::SpawnFailed`
- **Invalid JSON**: Log warning, skip malformed lines
- **Non-zero exit**: Capture in session, return error to user
- **Signal interruption**: Forward signal to child, wait for graceful exit

**Retry Logic:** None (user-initiated only)

### File System
**Purpose:** Store SQLite database and read context files
**Locations:**
- `.descartes/state.db` - SQLite database
- `.descartes/config.toml` - Configuration (future)

**Error Handling:**
- **DB corruption**: Return clear error, suggest `descartes init`
- **Permission denied**: Return actionable error message
- **Disk full**: Fail fast with error

## 6. Security Considerations

**Authentication:** None (local-only tool)

**Authorization:** File system permissions only

**Data Protection:**
- Events contain raw agent I/O → could include sensitive data
- Store in user's home directory (`.descartes/`)
- Respect `.gitignore` for context file reading
- No encryption in Phase 1 (add in Phase 2 if needed)

**Input Validation:**
- Sanitize file paths to prevent directory traversal
- Validate glob patterns before execution
- Limit event payload size (10MB max to prevent DB bloat)

**Security Risks:**

| Risk | Impact | Mitigation |
|------|--------|------------|
| Command injection via unsanitized args | High | Use tokio Command API (no shell execution) |
| Sensitive data in event logs | Medium | Document clearly, add encryption in Phase 2 |
| Malicious glob patterns (e.g., `/*`) | Low | Validate patterns, limit file count (100 max) |
| SQLite injection | Low | Use sqlx parameterized queries (compile-time checked) |

## 7. Performance Considerations

**Expected Load:**
- Single user, local machine
- 1-10 events/second during active agent session
- Database size: ~100MB per 10k events

**Bottlenecks:**
- **SQLite write contention**: Single writer, serialized writes
- **JSON parsing**: serde_json is fast but allocates

**Optimizations:**

1. **Hot Path Event Logging:**
   - Use prepared statements via sqlx macros
   - Keep payload JSON compact
   - Target: < 10ms per event write

2. **Connection Pooling:**
   - Max 5 connections (SQLite bottleneck is writes)
   - Use WAL mode for better concurrency

3. **Indexing:**
   - Index on `session_id`, `timestamp` for efficient queries
   - Index on task `status` for filtering

4. **Streaming:**
   - Stream stdout/stderr without buffering (unbounded mpsc channel)
   - Process events as they arrive

**Monitoring:**
- Use `tracing` spans for operation timing
- Log slow queries (> 100ms) at WARN level
- Expose metrics via `descartes stats` command (future)

**SQLite Configuration:**
```sql
-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;

-- Synchronous = NORMAL (faster writes, safe for crashes)
PRAGMA synchronous = NORMAL;

-- Increase cache size (10MB)
PRAGMA cache_size = -10000;
```

## 8. Testing Strategy

**Unit Tests:**
- **Scope:** All public functions in `core`, `agent-runner` crates
- **Tools:** `#[tokio::test]`, mock traits with `mockall` if needed
- **Coverage Target:** 80%+

**Examples:**
```rust
#[tokio::test]
async fn test_log_event() {
    let store = SqliteStore::new(":memory:").await.unwrap();
    store.run_migrations().await.unwrap();

    let event = Event {
        session_id: "test-123".to_string(),
        event_type: "text".to_string(),
        timestamp: Utc::now(),
        payload: AgentEvent::TextOutput { content: "hello".to_string() },
    };

    store.log_event(event).await.unwrap();

    let events = store.get_events("test-123", 10).await.unwrap();
    assert_eq!(events.len(), 1);
}
```

**Integration Tests:**
- **Scope:** CLI commands end-to-end
- **Tools:** `assert_cmd`, `predicates`, `tempfile`
- **Coverage:** All CLI commands, happy + error paths

**Examples:**
```rust
#[test]
fn test_descartes_init() {
    let temp_dir = tempfile::tempdir().unwrap();

    Command::cargo_bin("descartes")
        .unwrap()
        .arg("init")
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .descartes"));

    assert!(temp_dir.path().join(".descartes/state.db").exists());
}
```

**Mock Claude CLI:**
```bash
#!/bin/bash
# tests/fixtures/mock-claude.sh
# Simulates claude --output-format=stream-json

echo '{"type":"text","content":"Hello from mock Claude"}'
echo '{"type":"tool_use","tool":"read","args":{"path":"test.txt"}}'
exit 0
```

**E2E Tests:**
- **Scope:** Full workflow with real SQLite, mock Claude CLI
- **Scenarios:**
  - `init` → `spawn` → check DB
  - `spawn` with stdin pipe
  - `logs` command output
  - Signal handling (send SIGINT mid-execution)

**Performance Tests:**
- Not critical for Phase 1 (local single-user)
- Add benchmarks if event logging becomes bottleneck

**Test Organization:**
```
tests/
├── integration/
│   ├── cli_init.rs
│   ├── cli_spawn.rs
│   └── cli_logs.rs
├── fixtures/
│   ├── mock-claude.sh
│   └── sample-events.json
└── common/
    └── helpers.rs
```

## 9. Risks & Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Claude CLI changes JSON format | High | Medium | Version pin `claude` CLI, add schema validation tests |
| SQLite write performance insufficient | Medium | Low | Use WAL mode, benchmark early, consider write batching |
| Signal handling race conditions | Medium | Medium | Thorough testing, use tokio signal primitives correctly |
| Glob pattern DOS (e.g., `**/*`) | Low | Low | Limit file count to 100, timeout at 5s |
| Process zombie on crash | Low | Medium | Ensure proper Drop impl, test signal paths |
| Large event payloads bloating DB | Medium | Low | Implement 10MB max payload size, truncate if needed |
| User expects background mode | Low | High | Document blocking behavior clearly, add background in Phase 2 |

**Technical Debt Anticipated:**
- Hard-coded paths (`.descartes/`) → add config file in Phase 2
- Blocking spawn mode → add detached mode in Phase 2
- No multi-session support → add session management in Phase 2
- Basic glob only → add .gitignore awareness in Phase 2

## 10. Implementation Plan

### Phase 1.1: Foundation (Tasks 1, 2, 5)
**Tasks:**
- 1.1: Create Rust Workspace Structure
- 1.2: Add Necessary Dependencies
- 1.3: Set Up GitHub Actions CI/CD
- 2: Define Core Traits
- 5.1: Design Table Schemas
- 5.2-5.4: Set Up sqlx Migrations

**Rationale:** Establish project structure, define contracts before implementation
**Duration:** 2-4 hours
**Success Criteria:**
- ✅ `cargo build` succeeds
- ✅ Traits defined with clear documentation
- ✅ Migrations run successfully

### Phase 1.2: Process Management (Tasks 3, 4)
**Tasks:**
- 3.1-3.4: Implement LocalProcessRunner
- 4.1-4.4: Implement ClaudeCodeAdapter

**Rationale:** Core agent execution engine
**Duration:** 4-6 hours
**Success Criteria:**
- ✅ Can spawn child process
- ✅ Can stream stdout/stderr
- ✅ Signal handling works
- ✅ JSON parsing handles Claude output

**Dependencies:** Tasks 2 (traits must exist)

### Phase 1.3: Persistence (Task 6)
**Tasks:**
- 6.1-6.5: Implement SqliteStore

**Rationale:** Event logging critical for observability
**Duration:** 3-4 hours
**Success Criteria:**
- ✅ All StateStore methods implemented
- ✅ Hot path logging < 10ms
- ✅ Unit tests pass

**Dependencies:** Tasks 2, 5 (traits and schema must exist)

### Phase 1.4: CLI Commands (Tasks 7, 8)
**Tasks:**
- 7.1-7.4: Implement Basic CLI Commands
- 8.1-8.4: Implement CLI Pipe Support

**Rationale:** User-facing interface
**Duration:** 3-4 hours
**Success Criteria:**
- ✅ `descartes init` creates DB
- ✅ `descartes spawn` works end-to-end
- ✅ `descartes logs` displays events
- ✅ Pipe input works: `echo "hi" | descartes spawn`

**Dependencies:** Tasks 2, 3, 4, 6 (all core components must exist)

### Phase 1.5: Context Engine (Task 9)
**Tasks:**
- 9.1-9.4: Implement Basic File Reading

**Rationale:** Enable context injection for agents
**Duration:** 2-3 hours
**Success Criteria:**
- ✅ Glob patterns work
- ✅ Files readable asynchronously
- ✅ Integration with ContextSyncer trait

**Dependencies:** Task 2 (traits must exist)

### Phase 1.6: Testing & Polish (All test subtasks)
**Tasks:**
- 1.3: CI/CD pipeline
- 4.4, 6.5, 7.4, 8.4, 9.4: Test tasks

**Rationale:** Ensure quality before declaring Phase 1 complete
**Duration:** 4-6 hours
**Success Criteria:**
- ✅ All unit tests pass
- ✅ Integration tests pass
- ✅ CI pipeline green
- ✅ Acceptance criteria met (see below)

**Dependencies:** All implementation tasks

---

## Acceptance Criteria (Phase 1 Complete)

From planning document + architecture requirements:

1. ✅ Can run `descartes init` to create `.descartes/` directory and SQLite DB
2. ✅ Can run `descartes spawn "Say hello"` and see Claude output streamed to terminal
3. ✅ Can pipe a file: `cat README.md | descartes spawn "Summarize this"`
4. ✅ All events logged to SQLite (verify with `descartes logs`)
5. ✅ Signal handling works (Ctrl-C gracefully stops agent)
6. ✅ Unit test coverage > 80%
7. ✅ Integration tests pass in CI
8. ✅ No clippy warnings
9. ✅ Documentation complete (README, code comments)

---

## Appendix

### A. Crate Dependency Graph
```
cli
 ├─ core
 └─ agent-runner
     └─ core

(gui not used in Phase 1)
```

### B. Error Hierarchy
```rust
// core/src/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Process I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid JSON in stream: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Glob pattern error: {0}")]
    GlobError(#[from] glob::PatternError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### C. Sample Usage
```bash
# Initialize project
$ descartes init
✓ Initialized .descartes directory
✓ Created database: .descartes/state.db

# Spawn agent
$ descartes spawn "What files are in this directory?"
[Agent starts...]
{"type":"tool_use","tool":"bash","args":{"command":"ls"}}
README.md
src/
Cargo.toml
[Agent completes]

# Pipe context
$ cat src/main.rs | descartes spawn "Explain this code"
[Agent analyzes piped content...]

# View logs
$ descartes logs
session-abc123 | 2025-11-23T10:00:00Z | tool_use | {"tool":"bash",...}
session-abc123 | 2025-11-23T10:00:01Z | text | {"content":"I see 3 files..."}
```

### D. Future Enhancements (Phase 2+)
- Background agent mode (detached sessions)
- Multi-session management
- Configuration file (`.descartes/config.toml`)
- Event encryption for sensitive data
- Context awareness (.gitignore integration)
- Agent templates/presets
- Web UI (gui crate)

---

**End of Architecture Document**
