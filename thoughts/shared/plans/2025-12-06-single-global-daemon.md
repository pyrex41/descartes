# Single Global Daemon Implementation Plan

## Implementation Status: COMPLETE

All 5 phases have been implemented:
- [x] Phase 1: Add daemon launcher module to core (`daemon_launcher.rs`)
- [x] Phase 2: Simplify Session struct (daemon_info deprecated, serde skip_serializing)
- [x] Phase 3: Update GUI to use global daemon (auto-start on startup)
- [x] Phase 4: Update CLI to auto-start daemon (`connect_with_autostart()`)
- [x] Phase 5: Cleanup and verification

**Lines of code removed:** ~200+ lines (port allocation, daemon spawning helpers)
**New functionality:** ~120 lines (`daemon_launcher.rs` + CLI helper)

## Overview

Simplify Descartes from spawning one daemon per session to using a single user-level daemon that auto-starts when needed. This removes unnecessary complexity around port allocation, per-session process management, and daemon lifecycle coupling to sessions.

## Current State Analysis

### What Exists Now
- Each session can spawn its own `descartes-daemon` process with unique ports
- `Session` struct contains `daemon_info: Option<DaemonInfo>` with PID, endpoints
- `SessionManager` handles port allocation (base 19280/19380 + offset)
- GUI spawns daemons on session selection, connects per-session
- CLI uses Unix socket at `{base_path}/run/daemon.sock` but daemon isn't auto-started

### Key Discovery
The daemon itself is **already globally-scoped** - it has no workspace isolation. The per-session spawning is purely in the session manager and GUI layers. The daemon code doesn't even have a `--workdir` parameter.

## Desired End State

1. **Single daemon** runs at user level: `~/.descartes/run/daemon.sock` (Unix) / `http://127.0.0.1:19280` (HTTP)
2. **Auto-start**: Both GUI and CLI auto-start daemon if not running
3. **Sessions decoupled**: Sessions no longer track daemon info; daemon connection is app-level
4. **Workspace context**: Agent operations receive workspace path as parameter, not daemon configuration

### Verification
- GUI starts, daemon auto-starts if needed, connects to single endpoint
- CLI commands auto-start daemon if needed, use single socket
- Multiple GUI/CLI instances share the same daemon
- Session switching doesn't spawn/stop daemons
- `ps aux | grep descartes-daemon` shows at most one process

## What We're NOT Doing

- Multi-user daemon isolation (out of scope)
- Workspace-specific agent sandboxing (daemon already global)
- Changing the daemon's internal architecture
- Adding systemd/launchd service integration (future enhancement)

## Implementation Approach

Remove daemon lifecycle from sessions, add auto-start helper used by both GUI and CLI.

## Phase 1: Add Daemon Auto-Start Module

### Overview
Create a shared module for daemon discovery and auto-start that both GUI and CLI can use.

### Changes Required

#### 1. New daemon launcher module
**File**: `descartes/core/src/daemon_launcher.rs` (new file)

```rust
//! Daemon auto-start and connection utilities.
//!
//! Provides a single global daemon per user at ~/.descartes/run/daemon.sock

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Default ports for the global daemon
pub const DEFAULT_HTTP_PORT: u16 = 19280;
pub const DEFAULT_WS_PORT: u16 = 19380;

/// Get the path to the daemon socket
pub fn daemon_socket_path() -> PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".descartes")
        .join("run");
    base.join("daemon.sock")
}

/// Get the daemon HTTP endpoint
pub fn daemon_http_endpoint() -> String {
    format!("http://127.0.0.1:{}", DEFAULT_HTTP_PORT)
}

/// Get the daemon WebSocket endpoint
pub fn daemon_ws_endpoint() -> String {
    format!("ws://127.0.0.1:{}", DEFAULT_WS_PORT)
}

/// Check if daemon is running by testing the health endpoint
pub async fn is_daemon_running() -> bool {
    let endpoint = daemon_http_endpoint();
    match reqwest::get(&format!("{}/health", endpoint)).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Ensure daemon is running, starting it if necessary.
/// Returns Ok(true) if daemon was started, Ok(false) if already running.
pub async fn ensure_daemon_running() -> Result<bool, String> {
    if is_daemon_running().await {
        tracing::debug!("Daemon already running");
        return Ok(false);
    }

    tracing::info!("Starting daemon...");
    start_daemon().await?;
    Ok(true)
}

/// Start the daemon process in the background
async fn start_daemon() -> Result<(), String> {
    // Ensure run directory exists
    let socket_path = daemon_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create daemon run directory: {}", e))?;
    }

    // Spawn daemon process
    let mut cmd = Command::new("descartes-daemon");
    cmd.arg("--http-port")
        .arg(DEFAULT_HTTP_PORT.to_string())
        .arg("--ws-port")
        .arg(DEFAULT_WS_PORT.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    // Detach from parent process
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn daemon: {}. Is descartes-daemon in PATH?", e))?;

    // Wait for daemon to become healthy
    let mut attempts = 0;
    while attempts < 30 {
        if is_daemon_running().await {
            tracing::info!("Daemon started successfully on port {}", DEFAULT_HTTP_PORT);
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }

    Err("Daemon failed to start within 3 seconds".to_string())
}
```

#### 2. Export from core lib
**File**: `descartes/core/src/lib.rs`
**Changes**: Add module export

```rust
pub mod daemon_launcher;
pub use daemon_launcher::{
    ensure_daemon_running, is_daemon_running,
    daemon_http_endpoint, daemon_ws_endpoint, daemon_socket_path,
    DEFAULT_HTTP_PORT, DEFAULT_WS_PORT,
};
```

### Success Criteria

#### Automated Verification:
- [x] Code compiles: `cargo build -p descartes_core`
- [x] Unit tests pass: `cargo test -p descartes_core`

#### Manual Verification:
- [x] N/A - this phase just adds the module, no runtime behavior yet

---

## Phase 2: Simplify Session Struct

### Overview
Remove `daemon_info` from `Session` since daemon is now global.

### Changes Required

#### 1. Remove DaemonInfo from Session
**File**: `descartes/core/src/session.rs`

Remove these fields and structs:
- Remove `daemon_info: Option<DaemonInfo>` from `Session` struct (line 29)
- Remove `DaemonInfo` struct entirely (lines 98-127)
- Remove `DaemonInfo` impl block
- Update `Session::is_running()` to just check status

```rust
// Before
pub struct Session {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub daemon_info: Option<DaemonInfo>,  // REMOVE
}

// After
pub struct Session {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub status: SessionStatus,
}
```

Update `is_running()`:
```rust
// Before
pub fn is_running(&self) -> bool {
    matches!(self.status, SessionStatus::Active | SessionStatus::Starting)
}

// After - simplify since we don't track daemon per-session
pub fn is_running(&self) -> bool {
    matches!(self.status, SessionStatus::Active)
}
```

#### 2. Simplify SessionStatus
**File**: `descartes/core/src/session.rs`

Remove daemon-related statuses:
```rust
// Before
pub enum SessionStatus {
    Inactive,
    Starting,   // REMOVE - daemon starting
    Active,
    Stopping,   // REMOVE - daemon stopping
    Archived,
    Error,      // REMOVE - daemon error
}

// After
pub enum SessionStatus {
    Inactive,
    Active,
    Archived,
}
```

#### 3. Remove daemon lifecycle from SessionManager
**File**: `descartes/core/src/session_manager.rs`

Remove these fields from `FileSystemSessionManager`:
- `base_http_port` (line 31)
- `base_ws_port` (line 33)
- `port_offset` (line 35)

Remove these methods entirely:
- `get_next_port_offset()` (lines 191-197)
- `check_daemon_health()` (lines 199-205)
- `start_daemon()` (lines 345-395)
- `stop_daemon()` (lines 397-424)
- `refresh_session()` (lines 461-476) - or simplify to not check daemon
- `is_port_available()` (lines 479-482)
- `find_available_port_from()` (lines 485-493)
- `find_available_ports()` (lines 496-530)
- `spawn_daemon()` (lines 532-585)
- `graceful_shutdown()` (lines 588-595)
- `kill_daemon()` (lines 598-617)
- `with_ports()` (lines 55-59)

Remove from `SessionManager` trait:
- `start_daemon()`
- `stop_daemon()`
- `refresh_session()` (or simplify)

Update constructor to not initialize port fields.

#### 4. Update session metadata serialization
**File**: `descartes/core/src/session_manager.rs`

The `save_session_metadata()` and `load_session_from_path()` functions will automatically work since we're just removing a field from the struct. Old session.json files with `daemon_info` will deserialize fine (serde ignores unknown fields by default, but we should add `#[serde(default)]` to be safe).

### Success Criteria

#### Automated Verification:
- [x] Core compiles: `cargo build -p descartes_core`
- [x] Core tests pass: `cargo test -p descartes_core`
- [x] daemon_info deprecated (uses serde skip_serializing for backwards compat)

#### Manual Verification:
- [x] Existing session.json files still load (backwards compat via serde(default))

---

## Phase 3: Update GUI to Use Global Daemon

### Overview
Simplify GUI to connect to single global daemon on startup, remove per-session daemon management.

### Changes Required

#### 1. Add daemon connection state to app
**File**: `descartes/gui/src/main.rs`

Replace session-based daemon tracking with app-level:

```rust
// In DescartesGui struct, change:
// daemon_connected: bool,  // KEEP - but now means global daemon

// Remove these from new():
// rpc_client starts as None - will be set on startup

// Add to update() - handle new startup message
Message::StartupComplete(result) => {
    match result {
        Ok(_) => {
            self.daemon_connected = true;
            self.status_message = Some("Connected to daemon".to_string());
        }
        Err(e) => {
            self.daemon_connected = false;
            self.connection_error = Some(e);
        }
    }
    iced::Task::none()
}
```

#### 2. Auto-start daemon on GUI launch
**File**: `descartes/gui/src/main.rs`

Update `DescartesGui::new()` to return startup task:

```rust
fn new() -> (Self, iced::Task<Message>) {
    let app = Self {
        // ... existing fields ...
        daemon_connected: false,
        rpc_client: None,
        // ...
    };

    // Return task to ensure daemon running and connect
    let startup_task = iced::Task::perform(
        async {
            // Ensure daemon is running
            descartes_core::ensure_daemon_running().await?;

            // Create RPC client
            let endpoint = descartes_core::daemon_http_endpoint();
            let client = GuiRpcClient::new(&endpoint)?;
            client.connect().await?;

            Ok::<_, String>(client)
        },
        |result| Message::StartupComplete(result.map(|c| Arc::new(c)))
    );

    (app, startup_task)
}
```

#### 3. Simplify session messages
**File**: `descartes/gui/src/session_state.rs`

Remove daemon-related messages:
```rust
// REMOVE these variants from SessionMessage:
// StartDaemon(Uuid),
// DaemonStarted(Session),
// StopDaemon(Uuid),
// DaemonStopped(Uuid),
// DaemonError(String),
```

#### 4. Update session selection
**File**: `descartes/gui/src/main.rs`

Session selection no longer starts daemons:
```rust
// Before (lines 292-306):
SessionMessage::SelectSession(id) => {
    if session.daemon_info.is_none() ... {
        // spawn daemon
    }
}

// After:
SessionMessage::SelectSession(id) => {
    // Just update active session, daemon is already running
    session_state::update(&mut self.session_state, msg);
    iced::Task::none()
}
```

#### 5. Remove daemon spawn/stop functions
**File**: `descartes/gui/src/main.rs`

Delete these functions entirely:
- `spawn_daemon_for_session()` (lines 2110-2193)
- `stop_daemon()` (lines 2195-2228)
- `find_available_port()` (lines 2096-2107)

#### 6. Update session selector UI
**File**: `descartes/gui/src/session_selector.rs`

Remove Start/Stop daemon buttons from session cards:
```rust
// Remove the daemon control buttons (lines 224-269)
// Session cards just show status, clicking selects the session
```

#### 7. Update session state update function
**File**: `descartes/gui/src/session_state.rs`

Remove daemon message handling from `update()` function (lines 219-239).

### Success Criteria

#### Automated Verification:
- [x] GUI compiles: `cargo build -p descartes_gui`
- [x] spawn_daemon_for_session removed from main.rs
- [x] Session selection simplified (no daemon spawning)
- [x] Startup auto-starts global daemon

#### Manual Verification:
- [ ] Start GUI when no daemon running - daemon auto-starts
- [ ] Start GUI when daemon already running - connects to existing
- [ ] Session selection works without daemon spawn delay
- [ ] Multiple GUI windows share same daemon connection

---

## Phase 4: Update CLI to Auto-Start Daemon

### Overview
Update CLI to auto-start daemon when needed for commands that require it.

### Changes Required

#### 1. Update RPC connection helper
**File**: `descartes/cli/src/rpc.rs`

Replace `connect_or_bail` with auto-start version:

```rust
use descartes_core::{ensure_daemon_running, daemon_socket_path};

/// Connect to daemon, auto-starting if necessary
pub async fn connect_with_autostart() -> anyhow::Result<UnixSocketRpcClient> {
    // Ensure daemon is running (starts if needed)
    ensure_daemon_running().await
        .map_err(|e| anyhow::anyhow!("Failed to start daemon: {}", e))?;

    let socket_path = daemon_socket_path();

    let client = UnixSocketRpcClientBuilder::new()
        .socket_path(&socket_path)
        .timeout(30)
        .build()?;

    client.test_connection().await
        .map_err(|e| anyhow::anyhow!("Failed to connect to daemon: {}", e))?;

    Ok(client)
}

// Remove get_daemon_socket() - use daemon_socket_path() from core
// Remove is_daemon_running() - use from core
// Remove connect_or_bail() - replaced by connect_with_autostart()
```

#### 2. Update commands using daemon
**File**: `descartes/cli/src/commands/pause.rs`

```rust
// Before:
let client = rpc::connect_or_bail(config).await?;

// After:
let client = rpc::connect_with_autostart().await?;
```

Same change in:
- `descartes/cli/src/commands/resume.rs`
- `descartes/cli/src/commands/attach.rs`

#### 3. Update doctor command
**File**: `descartes/cli/src/commands/doctor.rs`

Update daemon check to use new utilities:
```rust
fn check_daemon() -> (Status, String) {
    // Use the shared daemon check
    let rt = tokio::runtime::Runtime::new().unwrap();
    let running = rt.block_on(descartes_core::is_daemon_running());

    if running {
        (Status::Ok, format!("running on port {}", descartes_core::DEFAULT_HTTP_PORT))
    } else {
        (Status::NotConfigured, "not running (will auto-start when needed)".to_string())
    }
}
```

### Success Criteria

#### Automated Verification:
- [x] CLI compiles: `cargo build -p descartes_cli`
- [x] CLI tests pass: `cargo test -p descartes_cli`
- [x] `connect_with_autostart()` added to rpc.rs
- [x] `pause`, `resume`, `attach` commands updated to auto-start

#### Manual Verification:
- [ ] `descartes pause <id>` auto-starts daemon if not running
- [ ] `descartes resume <id>` works with running daemon
- [ ] `descartes doctor` shows daemon status correctly
- [ ] Multiple CLI invocations share same daemon

---

## Phase 5: Cleanup and Documentation

### Overview
Remove dead code, update documentation, ensure consistency.

### Changes Required

#### 1. Remove unused imports
Run `cargo clippy` and fix unused import warnings across all crates.

#### 2. Update README/docs
**File**: `descartes/README.md`

Update to reflect single daemon architecture:
- Remove references to per-session daemons
- Document auto-start behavior
- Update architecture diagrams if any

#### 3. Remove dead test code
Check for tests that reference removed functionality:
- `daemon_info` field
- Per-session port allocation
- `start_daemon`/`stop_daemon` methods

### Success Criteria

#### Automated Verification:
- [x] Core, GUI, CLI build: `cargo build -p descartes-core -p descartes-gui -p descartes-cli`
- [x] Core lib tests pass: `cargo test -p descartes-core --lib`
- [x] CLI lib tests pass: `cargo test -p descartes-cli --lib`
- [x] daemon_launcher tests pass (3 tests)
- [x] session/session_manager tests pass (17 tests)
- Note: Pre-existing zmq_integration_tests.rs has compilation errors (unrelated to this change)

#### Manual Verification:
- [ ] Documentation is accurate
- [ ] Example workflows still work

---

## Testing Strategy

### Unit Tests
- `daemon_launcher::is_daemon_running()` returns correct state
- `daemon_launcher::ensure_daemon_running()` is idempotent
- Session serialization works without daemon_info field

### Integration Tests
- GUI startup with no daemon → daemon starts
- GUI startup with daemon running → connects to existing
- CLI command with no daemon → daemon starts
- CLI command with daemon → uses existing
- Multiple clients share daemon

### Manual Testing Steps
1. Kill any running daemon: `pkill descartes-daemon`
2. Start GUI → verify daemon starts automatically
3. Run CLI command → verify it uses same daemon
4. Close GUI → daemon continues running
5. Run another CLI command → still works
6. Check `ps aux | grep descartes-daemon` shows exactly 1 process

## Migration Notes

### Backwards Compatibility
- Old `session.json` files with `daemon_info` field will still load (serde ignores unknown fields)
- Add `#[serde(default)]` to Session struct for safety

### No User Action Required
- Daemon auto-starts, no manual intervention needed
- Existing sessions work as before (just won't have per-session daemon)

## References

- Research: `thoughts/shared/research/2025-12-06-sessions-and-daemon-instances.md`
- Session struct: `descartes/core/src/session.rs:14-30`
- Session manager: `descartes/core/src/session_manager.rs`
- GUI daemon handling: `descartes/gui/src/main.rs:2110-2228`
- CLI RPC: `descartes/cli/src/rpc.rs`
