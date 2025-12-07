---
date: 2025-12-06T23:17:14Z
researcher: Claude
git_commit: 41d09d4d1eff606d4c8c98a44bfc2296cbaef6d8
branch: master
repository: cap
topic: "Descartes Daemon Architecture - Session-Daemon Relationship"
tags: [research, codebase, daemon, sessions, gui]
status: complete
last_updated: 2025-12-06
last_updated_by: Claude
---

# Research: Descartes Daemon Architecture

**Date**: 2025-12-06T23:17:14Z
**Researcher**: Claude
**Git Commit**: 41d09d4d1eff606d4c8c98a44bfc2296cbaef6d8
**Branch**: master
**Repository**: cap

## Research Question

How does Descartes manage daemons? Is there a daemon per session? Should a daemon auto-start on GUI start?

## Summary

Descartes uses a **one daemon per session/workspace** model. Each workspace can have its own daemon instance running on dynamically allocated ports. The GUI starts in a disconnected state and does NOT auto-start a daemon on launch. Daemons are spawned on-demand when a user selects a session (auto-start on selection) or explicitly clicks "Start Daemon".

## Detailed Findings

### Current Architecture: One Daemon Per Session

**Session-Daemon Relationship** (`core/src/session.rs:14-30`):
- Each `Session` struct contains `daemon_info: Option<DaemonInfo>`
- `DaemonInfo` stores: `pid`, `http_endpoint`, `ws_endpoint`, `started_at`
- A session without a running daemon has `daemon_info: None`

**Port Allocation** (`gui/src/main.rs:2099-2102`):
```rust
let port_offset = (session.id.as_u128() % 100) as u16;
let http_port = 8080 + port_offset;
let ws_port = 8180 + port_offset;
```
- Ports are deterministic based on session UUID
- HTTP ports range: 8080-8179
- WebSocket ports range: 8180-8279

### GUI Connection Flow

1. **On GUI Start** (`gui/src/main.rs:172-192`):
   - `daemon_connected: false`
   - `rpc_client: None`
   - Status: "Welcome to Descartes GUI! Select a session or connect to the daemon."
   - **No automatic daemon start or connection**

2. **On Session Selection** (`gui/src/main.rs:292-305`):
   - Checks if daemon is running for selected session
   - If not running, **automatically spawns daemon** via `spawn_daemon_for_session()`
   - After spawn, auto-connects to the daemon

3. **On "Connect" Button Click** (`gui/src/main.rs:203-245`):
   - Creates RPC client for active session's daemon endpoint
   - Falls back to `http://127.0.0.1:8080` if no active session

### Daemon Spawning Process

**Spawn Function** (`gui/src/main.rs:2094-2164`):
1. Calculate ports from session ID
2. Execute `descartes-daemon --http-port X --ws-port Y`
3. Set working directory to workspace path
4. Poll health endpoint (30 attempts × 100ms = 3s timeout)
5. On success, store `DaemonInfo` in session

**Daemon Binary** (`daemon/src/main.rs`):
- Standalone process: `descartes-daemon`
- Listens on HTTP for RPC and WebSocket for events
- Each daemon instance serves one workspace

### Session Discovery

**Discovery Logic** (`core/src/session_manager.rs:60-64`):
- Scans configured paths for workspaces
- A directory is a workspace if it contains `.scud/` or `config.toml`
- Sessions loaded from `.scud/session.json` if present

**Default Search Paths** (`core/src/session.rs:130-152`):
- `$HOME/projects`
- `$HOME/.descartes`
- Recursive scan up to depth 3

### Daemon Lifecycle

**States** (`core/src/session.rs:68-83`):
- `Inactive` - No daemon running (default)
- `Starting` - Daemon spawning
- `Active` - Daemon running and healthy
- `Stopping` - Daemon shutting down
- `Archived` - Session archived
- `Error` - Daemon failed

**Shutdown** (`gui/src/main.rs:2166-2200`):
1. Graceful: POST to `/shutdown` endpoint
2. Forceful: SIGTERM to PID (Unix only)

## Code References

- `descartes/core/src/session.rs` - Session and DaemonInfo structs
- `descartes/core/src/session_manager.rs` - FileSystemSessionManager implementation
- `descartes/gui/src/main.rs:2094-2164` - spawn_daemon_for_session()
- `descartes/gui/src/main.rs:292-305` - Auto-start on session selection
- `descartes/gui/src/session_state.rs` - GUI session state management
- `descartes/daemon/src/main.rs` - Daemon binary entry point
- `descartes/daemon/src/server.rs` - Daemon server implementation

## Architecture Documentation

### Current Model: Session-Scoped Daemons

```
┌─────────────────────────────────────────────────────────┐
│                    Descartes GUI                        │
│  ┌─────────────────────────────────────────────────┐   │
│  │ SessionState                                     │   │
│  │  - sessions: Vec<Session>                       │   │
│  │  - active_session_id: Option<Uuid>              │   │
│  └─────────────────────────────────────────────────┘   │
│                         │                               │
│              ┌──────────┴──────────┐                   │
│              ▼                     ▼                    │
│  ┌─────────────────┐   ┌─────────────────┐            │
│  │ Session A       │   │ Session B       │            │
│  │ port: 8080+X    │   │ port: 8080+Y    │            │
│  │ daemon_info: ✓  │   │ daemon_info: ✗  │            │
│  └────────┬────────┘   └─────────────────┘            │
│           │                                            │
└───────────┼────────────────────────────────────────────┘
            │
            ▼
┌─────────────────────┐
│ descartes-daemon    │
│ PID: 12345          │
│ HTTP: 8080+X        │
│ WS: 8180+X          │
│ workdir: /project/A │
└─────────────────────┘
```

### Key Design Decisions

1. **One daemon per workspace**: Isolation between projects
2. **Lazy daemon start**: Only spawn when session is selected
3. **Auto-start on selection**: User doesn't need to manually start
4. **Port-per-session**: Avoids conflicts, enables multiple concurrent daemons
5. **Health polling**: Wait for daemon ready before connecting

## Open Questions

1. Should GUI auto-start a "default" daemon on launch for faster first interaction?
2. Should there be a single shared daemon mode for resource-constrained systems?
3. How to handle port conflicts if another process uses the calculated port?
4. Should daemon survive GUI close (background mode)?
