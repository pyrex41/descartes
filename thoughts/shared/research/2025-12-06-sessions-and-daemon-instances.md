---
date: 2025-12-06T23:31:27Z
researcher: Claude
git_commit: 41d09d4d1eff606d4c8c98a44bfc2296cbaef6d8
branch: master
repository: cap
topic: "Sessions and Daemon Instances in Descartes"
tags: [research, codebase, descartes, sessions, daemon, zmq, gui]
status: complete
last_updated: 2025-12-06
last_updated_by: Claude
---

# Research: Sessions and Daemon Instances in Descartes

**Date**: 2025-12-06T23:31:27Z
**Researcher**: Claude
**Git Commit**: 41d09d4d1eff606d4c8c98a44bfc2296cbaef6d8
**Branch**: master
**Repository**: cap

## Research Question

How do sessions and daemon instances work in Descartes?

## Summary

Descartes uses a **session-based architecture** where each workspace is represented by a `Session` that can have an associated daemon process. Sessions are discovered from the filesystem, managed by a `FileSystemSessionManager`, and persisted as JSON files. When a user activates a session, a daemon process is spawned that provides HTTP/WebSocket RPC endpoints for agent orchestration. The GUI communicates with daemons via HTTP/WS, while agents can communicate via ZMQ for distributed orchestration.

**Key architectural components:**
1. **Sessions** - Workspace metadata with lifecycle status and optional daemon connection info
2. **Session Manager** - Discovers, creates, and manages session lifecycle
3. **Daemon Processes** - Background servers providing RPC endpoints for each workspace
4. **State Store** - SQLite persistence for agent states, events, and tasks (separate from sessions)
5. **ZMQ Communication** - Distributed agent orchestration across multiple machines
6. **GUI Session State** - Iced-based UI for session management and daemon interaction

## Detailed Findings

### Session Data Model

#### Session Structure (`descartes/core/src/session.rs:14-30`)

```rust
pub struct Session {
    pub id: Uuid,                           // Unique identifier
    pub name: String,                       // Human-readable name
    pub path: PathBuf,                      // Workspace root directory
    pub created_at: DateTime<Utc>,          // Creation timestamp
    pub last_accessed: Option<DateTime<Utc>>, // Last access time
    pub status: SessionStatus,              // Lifecycle status
    pub daemon_info: Option<DaemonInfo>,    // Running daemon info (if any)
}
```

#### Session Status Lifecycle (`descartes/core/src/session.rs:68-83`)

```
Inactive ──→ Starting ──→ Active ──→ Stopping ──→ Inactive
    │                        │
    └──→ Archived            └──→ Error
```

- **Inactive**: Session exists but daemon not running
- **Starting**: Daemon is being spawned
- **Active**: Daemon running and healthy
- **Stopping**: Graceful shutdown in progress
- **Archived**: Session archived (excluded from normal listing)
- **Error**: Startup or runtime error occurred

#### DaemonInfo Structure (`descartes/core/src/session.rs:99-109`)

```rust
pub struct DaemonInfo {
    pub pid: Option<u32>,           // Process ID
    pub http_endpoint: String,      // HTTP RPC endpoint (e.g., "http://127.0.0.1:19280")
    pub ws_endpoint: Option<String>, // WebSocket endpoint for events
    pub started_at: DateTime<Utc>,  // Daemon start timestamp
}
```

### Session Manager (`descartes/core/src/session_manager.rs`)

The `FileSystemSessionManager` provides session lifecycle management:

#### Session Discovery (lines 216-245)

1. Scans configured search paths (default: `~/projects`, `~/.descartes`)
2. Recursively searches for workspace markers (`.scud/` directory OR `config.toml` file)
3. Loads session metadata from `.scud/session.json` if present
4. Creates new `Session` objects for directories with markers but no metadata
5. Caches sessions in `HashMap<Uuid, Session>` for fast lookup

#### Session Creation (lines 257-300)

Creates workspace directory structure:
```
<workspace>/
├── .scud/
│   ├── session.json          # Session metadata (JSON)
│   ├── tasks/
│   │   └── tasks.json        # Task storage
│   └── workflow-state.json   # CL workflow state
├── data/                     # Data directory
├── thoughts/                 # Thoughts directory
└── logs/                     # Logs directory
```

#### Daemon Spawning (lines 533-585)

1. Finds available ports via `find_available_ports()` (base HTTP: 19280, WS: 19380)
2. Spawns `descartes-daemon` process with arguments:
   - `--http-port <PORT>`
   - `--ws-port <PORT>`
   - `--workdir <PATH>`
   - `--config <PATH>` (optional)
3. Polls `/health` endpoint every 100ms up to 30 times (3s timeout)
4. On success, stores `DaemonInfo` with PID and endpoints

#### Port Allocation (lines 480-530)

- Uses atomic counter for port offset
- Tests port availability via `TcpListener::bind()`
- Scans up to 200 ports from base if preferred port unavailable
- Prevents port conflicts between multiple daemon instances

#### Daemon Shutdown (lines 397-424, 587-617)

Two-phase shutdown:
1. **Graceful**: POST to `/shutdown` endpoint
2. **Force**: SIGTERM via `nix::sys::signal::kill()` (Unix only)

### Daemon Server (`descartes/daemon/src/`)

Each daemon provides:

#### HTTP RPC Server
- `/health` - Health check endpoint
- `/shutdown` - Graceful shutdown endpoint
- Full RPC API for agent operations

#### WebSocket Event Stream
- Real-time event notifications
- Agent status updates
- Task state changes

#### Configuration (`descartes/daemon/src/config.rs`)
- TOML-based configuration
- Default ports: HTTP 8080, WebSocket 8081, Metrics 9090
- CLI arguments override config file values

### State Store (`descartes/core/src/state_store.rs`)

SQLite-backed persistence layer **separate from sessions**:

#### Stored Data
- **Events**: Conversation and system events linked to sessions
- **Tasks**: Global task manager tasks with status, priority, complexity
- **Sessions**: Session metadata (different from filesystem sessions)
- **Agent States**: Agent snapshots with key-based access
- **State Transitions**: Audit log for state changes
- **State Snapshots**: Point-in-time backups

#### Schema Migrations (lines 186-286)
1. Create agent states table
2. Create state transitions table
3. Create state snapshots table
4. Add performance indexes
5. Enhance task model with dependencies

**Note**: The state store manages **agent runtime data**, while sessions manage **workspace/daemon metadata**. They are independent persistence mechanisms.

### ZMQ Communication (`descartes/core/src/zmq_*.rs`)

Provides distributed agent orchestration:

#### Message Types (`zmq_agent_runner.rs:536-580`)

**Requests:**
- `SpawnRequest` - Spawn agent with config and timeout
- `ControlCommand` - Agent control (pause, resume, stop, kill, etc.)
- `ListAgentsRequest` - List agents with filters
- `HealthCheckRequest` - Server health check
- `BatchControlCommand` - Control multiple agents

**Responses:**
- `SpawnResponse` - Returns `AgentInfo` or error
- `CommandResponse` - Operation result with agent status
- `ListAgentsResponse` - Array of `AgentInfo`
- `StatusUpdate` - Async push from server to clients

#### Socket Patterns (`zmq_communication.rs:103-113`)

- `Req`/`Rep` - Synchronous request/response (default)
- `Dealer`/`Router` - Asynchronous multi-client (supported)

#### Connection Management

- Connection state machine: Disconnected → Connecting → Connected
- Automatic reconnection with exponential backoff
- 10MB max message size with MessagePack serialization
- Request/response correlation via UUID-keyed oneshot channels

#### ZMQ Server (`zmq_server.rs:71-101`)

```rust
pub struct ZmqServerConfig {
    pub endpoint: String,            // Default: "tcp://0.0.0.0:5555"
    pub server_id: String,           // Auto-generated UUID
    pub max_agents: usize,           // Default: 100
    pub status_update_interval_secs: u64, // Default: 10
    pub enable_status_updates: bool, // Default: true
}
```

### GUI Session State (`descartes/gui/src/session_state.rs`)

Iced-based UI state management:

#### SessionState Structure (lines 10-28)

```rust
pub struct SessionState {
    pub sessions: Vec<Session>,          // All discovered sessions
    pub active_session_id: Option<Uuid>, // Currently active session
    pub new_session_name: String,        // Create dialog input
    pub new_session_path: String,        // Create dialog input
    pub loading: bool,                   // Loading indicator
    pub error: Option<String>,           // Error display
    pub show_create_dialog: bool,        // Dialog visibility
    pub filter: SessionFilter,           // List filtering
}
```

#### User Operations

| Operation | Message | Handler |
|-----------|---------|---------|
| Discover sessions | `RefreshSessions` | Scans filesystem |
| Select session | `SelectSession(Uuid)` | Auto-starts daemon if inactive |
| Create session | `CreateSession` | Creates workspace structure |
| Start daemon | `StartDaemon(Uuid)` | Spawns daemon process |
| Stop daemon | `StopDaemon(Uuid)` | Graceful then force shutdown |
| Archive session | `ArchiveSession(Uuid)` | Stops daemon, sets status |
| Delete session | `DeleteSession(Uuid)` | Removes from filesystem |

#### Daemon Communication

1. GUI retrieves `daemon_info.http_endpoint` from active session
2. Creates `GuiRpcClient` with endpoint
3. Establishes connection via HTTP
4. Subscribes to WebSocket events for real-time updates

## Code References

### Core Session Files
- `descartes/core/src/session.rs` - Session and DaemonInfo structures
- `descartes/core/src/session_manager.rs` - FileSystemSessionManager implementation
- `descartes/core/src/state_store.rs` - SQLite state persistence

### Daemon Files
- `descartes/daemon/src/main.rs` - Daemon entry point
- `descartes/daemon/src/server.rs` - RPC server implementation
- `descartes/daemon/src/config.rs` - Daemon configuration
- `descartes/daemon/src/rpc_server.rs` - RPC handler

### ZMQ Files
- `descartes/core/src/zmq_communication.rs` - Socket management
- `descartes/core/src/zmq_server.rs` - ZMQ server
- `descartes/core/src/zmq_client.rs` - ZMQ client
- `descartes/core/src/zmq_agent_runner.rs` - Message types

### GUI Files
- `descartes/gui/src/session_state.rs` - Session state management
- `descartes/gui/src/session_selector.rs` - Session UI components
- `descartes/gui/src/main.rs` - Main application with daemon connection

### CLI Commands
- `descartes/cli/src/commands/spawn.rs` - Spawn daemon instances
- `descartes/cli/src/commands/kill.rs` - Kill daemon instances
- `descartes/cli/src/commands/attach.rs` - Attach to sessions
- `descartes/cli/src/commands/ps.rs` - List running instances

## Architecture Documentation

### Session-Daemon Relationship

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Interface                           │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │   GUI App   │    │     CLI     │    │   Web UI    │         │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘         │
│         │                  │                  │                 │
│         └──────────────────┼──────────────────┘                 │
│                            │                                    │
│                    HTTP/WebSocket RPC                           │
└────────────────────────────┼────────────────────────────────────┘
                             │
┌────────────────────────────┼────────────────────────────────────┐
│                            ▼                                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Session Manager (Core)                       │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐      │  │
│  │  │Session A│  │Session B│  │Session C│  │Session D│      │  │
│  │  │ Active  │  │Inactive │  │Starting │  │Archived │      │  │
│  │  └────┬────┘  └─────────┘  └────┬────┘  └─────────┘      │  │
│  │       │                        │                          │  │
│  └───────┼────────────────────────┼──────────────────────────┘  │
│          │                        │                             │
│          ▼                        ▼                             │
│  ┌───────────────┐        ┌───────────────┐                    │
│  │  Daemon (A)   │        │  Daemon (C)   │                    │
│  │  HTTP: 19280  │        │  HTTP: 19281  │                    │
│  │  WS:   19380  │        │  WS:   19381  │                    │
│  └───────┬───────┘        └───────────────┘                    │
│          │                                                      │
│          ▼                                                      │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   Agent Orchestration                      │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐                    │ │
│  │  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │                    │ │
│  │  └─────────┘  └─────────┘  └─────────┘                    │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│                        Descartes Core                           │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Session Discovery**: Filesystem scan → Cache in HashMap → Return to UI
2. **Daemon Start**: Find ports → Spawn process → Health check → Store DaemonInfo
3. **Agent Spawn**: HTTP/ZMQ request → Daemon → Create agent → Return AgentInfo
4. **Event Updates**: Agent state change → Daemon → WebSocket/ZMQ → UI update

### Persistence Strategy

| Data Type | Storage Location | Format |
|-----------|------------------|--------|
| Session metadata | `{workspace}/.scud/session.json` | JSON |
| Active session ID | `~/.descartes/sessions.json` | JSON |
| Agent states | SQLite database | SQL |
| Events/Tasks | SQLite database | SQL |
| Configuration | `config.toml` or `daemon.toml` | TOML |

## Related Research

- `thoughts/shared/research/2025-12-06-descartes-daemon-architecture.md` - Daemon architecture details

## Open Questions

1. How does session discovery handle moved workspaces? (Currently updates path in session.json)
2. What happens if a daemon crashes without cleanup? (Detected by health check on refresh)
3. How are multiple GUI instances coordinated? (Currently single active session per user)
4. Is there automatic daemon restart on crash? (Not currently implemented)
