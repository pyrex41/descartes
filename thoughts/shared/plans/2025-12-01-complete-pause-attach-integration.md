# Complete Pause/Attach Integration - Implementation Plan

## Overview

Complete the integration between CLI, Daemon, and GUI for the pause/attach feature. The core infrastructure exists but there's a critical gap: **the CLI commands only update the database, they don't actually pause/resume the running process or communicate with the daemon**.

## Current State Analysis

### What's Fully Implemented
1. **Core Library** (`descartes-core`):
   - `AgentRunner.pause()` and `resume()` trait methods (`traits.rs:141-151`)
   - `LocalProcessRunner` implementation with SIGSTOP/SIGCONT (`agent_runner.rs:514-629`)
   - `AttachTokenStore` for token management (`attach.rs`)
   - `AttachProtocol` message types (`attach_protocol.rs`)

2. **Daemon** (`descartes-daemon`):
   - RPC methods: `agent.pause`, `agent.resume`, `agent.attach.*` (`rpc_server.rs:92-121`)
   - `AttachSessionManager` for session lifecycle (`attach_session.rs`)
   - TUI handlers: `ClaudeCodeTuiHandler`, `OpenCodeTuiHandler`
   - Full attach protocol with handshake, I/O forwarding

3. **CLI Commands** (exist but incomplete):
   - `pause.rs`, `resume.rs`, `attach.rs` - update database only
   - No RPC calls to daemon
   - No actual process control

4. **GUI** (UI exists but not wired):
   - `SwarmMonitor` has pause/resume/attach buttons (`swarm_monitor.rs:1244-1316`)
   - Message types defined (`SwarmMonitorMessage::PauseAgent`, etc.)
   - **Not integrated** into main app (`main.rs:1084-1095` is placeholder)
   - No RPC client methods for pause/attach

### Key Discovery: Architecture Gap

The CLI commands (`pause.rs`, `resume.rs`, `attach.rs`) operate directly on the database:
- They don't call the daemon's RPC methods
- They don't send SIGSTOP/SIGCONT to processes
- They don't use `AttachTokenStore` (generate their own tokens)

**This means**: Pausing via CLI only changes DB status, the actual process keeps running.

## Desired End State

1. `descartes pause <id>` → calls daemon RPC → daemon pauses actual process via SIGSTOP/cooperative
2. `descartes resume <id>` → calls daemon RPC → daemon resumes process via SIGCONT/stdin notification
3. `descartes attach <id>` → calls daemon RPC → gets credentials → optionally launches TUI client
4. GUI buttons → RPC calls → same daemon operations
5. All operations use the daemon's `AttachTokenStore` and `AttachSessionManager`

### Verification Criteria
- `descartes pause --agent-id <id>` actually stops process execution
- `ps aux | grep <pid>` shows "T" (stopped) state for force-paused agents
- `descartes attach` credentials work to connect via Unix socket
- GUI can pause/resume agents through daemon

## What We're NOT Doing

1. **SSH tunneling** - documented but deferred
2. **Multi-user sessions** - single attach session per agent
3. **Windows support** - Unix SIGSTOP/SIGCONT only
4. **Automatic TUI launch** - CLI prints instructions, user runs command manually
5. **Persistent sessions** - all in-memory, lost on daemon restart

## Implementation Approach

The CLI needs to become a thin RPC client to the daemon. The daemon already has all the logic.

**Strategy**:
1. Add RPC client to CLI that connects to daemon
2. Replace database-only operations with RPC calls
3. Wire GUI into main.rs with RPC integration
4. Add TUI launcher as optional enhancement

---

## Phase 1: CLI RPC Client Integration

### Overview
Make CLI commands call daemon RPC instead of directly manipulating the database.

### Changes Required

#### 1.1 Add RPC Client to CLI

**File**: `descartes/cli/Cargo.toml`
**Changes**: Add dependency on daemon RPC client

```toml
[dependencies]
# ... existing deps ...
descartes-daemon = { path = "../daemon", features = ["client"] }
```

#### 1.2 Create CLI RPC Helper Module

**File**: `descartes/cli/src/rpc.rs` (new file)
**Changes**: Wrapper for daemon RPC calls

```rust
//! RPC client for communicating with the Descartes daemon.

use anyhow::Result;
use descartes_core::DescaratesConfig;

/// Get the daemon socket path from config
pub fn get_daemon_socket(config: &DescaratesConfig) -> String {
    format!("{}/run/daemon.sock", config.storage.base_path)
}

/// Check if daemon is running
pub async fn is_daemon_running(config: &DescaratesConfig) -> bool {
    let socket_path = get_daemon_socket(config);
    std::path::Path::new(&socket_path).exists()
}

/// Connect to daemon or bail with helpful error
pub async fn connect_or_bail(config: &DescaratesConfig) -> Result<descartes_daemon::UnixSocketRpcClient> {
    let socket_path = get_daemon_socket(config);

    if !std::path::Path::new(&socket_path).exists() {
        anyhow::bail!(
            "Daemon not running. Start it with 'descartes daemon' or use '--no-daemon' for database-only mode."
        );
    }

    descartes_daemon::UnixSocketRpcClient::connect(&socket_path).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to daemon: {}", e))
}
```

#### 1.3 Update Pause Command to Use RPC

**File**: `descartes/cli/src/commands/pause.rs`
**Changes**: Call daemon RPC instead of database

Replace lines 20-68 (database operations) with RPC call:

```rust
use crate::rpc;

pub async fn execute(config: &DescaratesConfig, id: &str, force: bool) -> Result<()> {
    let mode = if force { "forced (SIGSTOP)" } else { "cooperative" };
    println!(
        "{}",
        format!("Pausing agent: {} (mode: {})", id, mode)
            .yellow()
            .bold()
    );

    // Parse UUID
    let _agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    // Connect to daemon
    let client = rpc::connect_or_bail(config).await?;

    // Call pause RPC
    let result: serde_json::Value = client
        .call("agent.pause", serde_json::json!({
            "agent_id": id,
            "force": force
        }))
        .await?;

    // Parse result
    let agent_id = result["agent_id"].as_str().unwrap_or(id);
    let paused_at = result["paused_at"].as_i64().unwrap_or(0);
    let pause_mode = result["pause_mode"].as_str().unwrap_or(mode);

    println!("\n{}", "Agent paused successfully.".green().bold());
    println!("  Mode: {}", pause_mode.cyan());
    println!(
        "  Paused at: {}",
        chrono::DateTime::from_timestamp(paused_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string())
            .cyan()
    );

    // Show attach hint
    println!(
        "\n{}",
        "To attach an external TUI, run: descartes attach <agent-id>".cyan()
    );

    Ok(())
}
```

#### 1.4 Update Resume Command to Use RPC

**File**: `descartes/cli/src/commands/resume.rs`
**Changes**: Call daemon RPC instead of database

Similar pattern - replace database operations with:

```rust
use crate::rpc;

pub async fn execute(config: &DescaratesConfig, id: &str) -> Result<()> {
    println!("{}", format!("Resuming agent: {}", id).yellow().bold());

    let _agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    let client = rpc::connect_or_bail(config).await?;

    let result: serde_json::Value = client
        .call("agent.resume", serde_json::json!({
            "agent_id": id
        }))
        .await?;

    let resumed_at = result["resumed_at"].as_i64().unwrap_or(0);

    println!("\n{}", "Agent resumed successfully.".green().bold());
    println!(
        "  Resumed at: {}",
        chrono::DateTime::from_timestamp(resumed_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string())
            .cyan()
    );

    Ok(())
}
```

#### 1.5 Update Attach Command to Use RPC

**File**: `descartes/cli/src/commands/attach.rs`
**Changes**: Use daemon's token store via RPC

```rust
use crate::rpc;

pub async fn execute(
    config: &DescaratesConfig,
    id: &str,
    client_type: &str,
    output_json: bool,
) -> Result<()> {
    if !output_json {
        println!(
            "{}",
            format!("Requesting attach credentials for agent: {}", id)
                .yellow()
                .bold()
        );
    }

    let _agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    let client = rpc::connect_or_bail(config).await?;

    let result: serde_json::Value = client
        .call("agent.attach.request", serde_json::json!({
            "agent_id": id,
            "client_type": client_type
        }))
        .await?;

    let token = result["token"].as_str().ok_or_else(|| anyhow::anyhow!("No token in response"))?;
    let connect_url = result["connect_url"].as_str().ok_or_else(|| anyhow::anyhow!("No URL in response"))?;
    let expires_at = result["expires_at"].as_i64().unwrap_or(0);

    if output_json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("\n{}", "Attach credentials generated:".green().bold());
        println!("  Token: {}", token.cyan());
        println!("  Connect URL: {}", connect_url.cyan());
        println!(
            "  Expires at: {}",
            chrono::DateTime::from_timestamp(expires_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "Unknown".to_string())
                .cyan()
        );

        println!("\n{}", "Usage:".yellow().bold());
        match client_type {
            "claude-code" => {
                println!("  claude --attach-token {} --connect {}", token, connect_url);
            }
            "opencode" => {
                println!("  opencode attach --token {} --url {}", token, connect_url);
            }
            _ => {
                println!("  Pass the token and connect_url to your TUI client");
            }
        }
    }

    Ok(())
}
```

#### 1.6 Register RPC Module

**File**: `descartes/cli/src/main.rs`
**Changes**: Add mod declaration

```rust
mod rpc;
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-cli` succeeds
- [ ] `cargo test -p descartes-cli` passes (add basic tests)
- [ ] `cargo clippy -p descartes-cli` no new warnings

#### Manual Verification:
- [ ] Start daemon: `descartes daemon`
- [ ] Spawn agent: `descartes spawn --task "sleep 60" --name test`
- [ ] `descartes pause <id>` - verify `ps aux | grep` shows "T" state
- [ ] `descartes resume <id>` - verify process continues
- [ ] `descartes attach <id>` - verify credentials returned
- [ ] Without daemon running, commands show helpful error

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to the next phase.

---

## Phase 2: Daemon Socket Server for Attach

### Overview
The daemon needs to start the Unix socket listener that TUI clients connect to.

### Changes Required

#### 2.1 Start Attach Server on Pause

**File**: `descartes/daemon/src/rpc_server.rs`
**Changes**: When an agent is paused, start the attach socket listener

In `pause_agent_internal()` after successful pause, start the attach server:

```rust
// After pause succeeds, start attach server for this agent
let socket_path = format!("{}/run/attach-{}.sock",
    self.config.storage.base_path,
    agent_id
);

// Spawn attach server task
let session_manager = Arc::clone(&self.attach_manager);
let agent_name = agent_info.name.clone();
let agent_task = agent_info.task.clone();

tokio::spawn(async move {
    if let Err(e) = descartes_daemon::claude_code_tui::start_attach_server(
        &socket_path,
        session_manager,
        agent_id,
        agent_name,
        agent_task,
    ).await {
        tracing::error!("Attach server failed: {}", e);
    }
});
```

#### 2.2 Stop Attach Server on Resume

**File**: `descartes/daemon/src/rpc_server.rs`
**Changes**: When an agent is resumed, terminate attach sessions and stop server

```rust
// Before resuming, terminate any active attach sessions
self.attach_manager.terminate_sessions_for_agent(agent_id).await;

// Remove socket file
let socket_path = format!("{}/run/attach-{}.sock",
    self.config.storage.base_path,
    agent_id
);
let _ = std::fs::remove_file(&socket_path);
```

#### 2.3 Wire Up I/O Channels

**File**: `descartes/daemon/src/rpc_server.rs`
**Changes**: Pass stdin_tx and stdout/stderr_rx to attach handlers

The attach handlers need access to the agent's I/O channels. This requires:
1. Storing channel handles in agent registry
2. Passing them to TUI handler on attach

```rust
// In attach_request_internal, get I/O channels from agent handle
let agent_handle = self.agent_runner.get_handle(&agent_id).await?;
let stdin_tx = agent_handle.stdin_sender();
let stdout_rx = agent_handle.stdout_receiver();
let stderr_rx = agent_handle.stderr_receiver();
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-daemon` succeeds
- [ ] `cargo test -p descartes-daemon` passes

#### Manual Verification:
- [ ] Pause agent, verify socket file created at `/run/attach-<id>.sock`
- [ ] Resume agent, verify socket file removed
- [ ] Connect to socket with test client, verify handshake works

**Implementation Note**: After completing this phase, pause for manual verification before Phase 3.

---

## Phase 3: GUI Integration

### Overview
Wire the SwarmMonitor UI into the main application with RPC calls.

### Changes Required

#### 3.1 Add RPC Client Methods

**File**: `descartes/gui/src/rpc_client.rs`
**Changes**: Add pause/resume/attach methods

```rust
impl GuiRpcClient {
    pub async fn pause_agent(&self, agent_id: Uuid, force: bool) -> Result<PauseResult> {
        self.client()
            .call("agent.pause", json!({"agent_id": agent_id.to_string(), "force": force}))
            .await
    }

    pub async fn resume_agent(&self, agent_id: Uuid) -> Result<ResumeResult> {
        self.client()
            .call("agent.resume", json!({"agent_id": agent_id.to_string()}))
            .await
    }

    pub async fn attach_request(&self, agent_id: Uuid, client_type: &str) -> Result<AttachCredentials> {
        self.client()
            .call("agent.attach.request", json!({
                "agent_id": agent_id.to_string(),
                "client_type": client_type
            }))
            .await
    }
}
```

#### 3.2 Integrate SwarmMonitor into Main App

**File**: `descartes/gui/src/main.rs`
**Changes**: Replace placeholder with actual SwarmMonitor

1. Import swarm_monitor module
2. Add SwarmMonitorState to App state
3. Route SwarmMonitor messages to handler
4. Render actual SwarmMonitor view

```rust
// In App struct
swarm_monitor_state: swarm_monitor::SwarmMonitorState,

// In view function, replace placeholder:
View::SwarmMonitor => {
    self.swarm_monitor_state.view().map(Message::SwarmMonitor)
}

// Add message variant:
enum Message {
    // ...
    SwarmMonitor(swarm_monitor::SwarmMonitorMessage),
}

// In update function:
Message::SwarmMonitor(msg) => {
    match msg {
        SwarmMonitorMessage::PauseAgent(id) => {
            // Spawn async RPC task
            iced::Task::perform(
                self.rpc_client.pause_agent(id, false),
                move |result| Message::SwarmMonitor(
                    SwarmMonitorMessage::PauseResult(id, result.map_err(|e| e.to_string()))
                )
            )
        }
        // ... similar for resume, attach
        _ => self.swarm_monitor_state.update(msg)
    }
}
```

#### 3.3 Add Attach Credentials Modal

**File**: `descartes/gui/src/swarm_monitor.rs`
**Changes**: Show modal with credentials after attach request succeeds

```rust
fn view_attach_modal(&self, credentials: &(Uuid, String, String)) -> Element<SwarmMonitorMessage> {
    let (agent_id, token, url) = credentials;

    Column::new()
        .push(Text::new("Attach Credentials").size(20))
        .push(Text::new(format!("Token: {}", token)))
        .push(Text::new(format!("URL: {}", url)))
        .push(
            Button::new(Text::new("Copy to Clipboard"))
                .on_press(SwarmMonitorMessage::CopyCredentials)
        )
        .push(
            Button::new(Text::new("Close"))
                .on_press(SwarmMonitorMessage::CloseAttachModal)
        )
        .into()
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-gui` succeeds
- [ ] GUI launches without errors

#### Manual Verification:
- [ ] Open GUI, navigate to Swarm Monitor
- [ ] Click Pause on running agent, verify status changes
- [ ] Click Resume on paused agent, verify status changes
- [ ] Click Attach, verify credentials modal appears
- [ ] Verify credentials can be copied

**Implementation Note**: After completing this phase, pause for manual verification before Phase 4.

---

## Phase 4: TUI Launcher (Optional Enhancement)

### Overview
Add ability to automatically launch Claude Code or OpenCode TUI clients.

### Changes Required

#### 4.1 Create TUI Launcher Module

**File**: `descartes/cli/src/tui_launchers/mod.rs` (new)
```rust
pub mod claude_code;
pub mod opencode;
```

#### 4.2 Claude Code Launcher

**File**: `descartes/cli/src/tui_launchers/claude_code.rs` (new)
```rust
use anyhow::Result;
use std::process::Command;

pub fn is_available() -> bool {
    which::which("claude").is_ok()
}

pub fn launch(token: &str, connect_url: &str) -> Result<()> {
    if !is_available() {
        anyhow::bail!("Claude Code not found. Install it or use --no-launch");
    }

    let mut cmd = Command::new("claude");
    cmd.arg("--attach-token").arg(token);
    cmd.arg("--connect").arg(connect_url);

    // Set environment variables as backup
    cmd.env("DESCARTES_ATTACH_TOKEN", token);
    cmd.env("DESCARTES_ATTACH_URL", connect_url);

    println!("Launching Claude Code...");
    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Claude Code exited with status: {}", status);
    }

    Ok(())
}
```

#### 4.3 Update Attach Command with --launch Flag

**File**: `descartes/cli/src/commands/attach.rs`
**Changes**: Add `--launch` flag to auto-launch TUI

```rust
// In main.rs, add to Attach command:
#[arg(long, default_value = "false")]
launch: bool,

// In attach.rs execute():
if args.launch {
    match client_type {
        "claude-code" => crate::tui_launchers::claude_code::launch(&token, &connect_url)?,
        "opencode" => crate::tui_launchers::opencode::launch(&token, &connect_url)?,
        _ => println!("Unknown client type, cannot auto-launch"),
    }
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-cli` succeeds
- [ ] Launcher handles missing executables gracefully

#### Manual Verification:
- [ ] `descartes attach <id> --launch` launches Claude Code (if installed)
- [ ] `descartes attach <id> --client opencode --launch` launches OpenCode (if installed)
- [ ] Missing executable shows helpful error

---

## Testing Strategy

### Unit Tests

#### CLI RPC Tests (`cli/tests/rpc_tests.rs`)
- Test RPC client connection
- Test error handling when daemon not running
- Mock RPC responses for pause/resume/attach

#### Daemon Integration Tests (`daemon/tests/attach_e2e_tests.rs`)
- Test full pause → attach → resume flow
- Test token expiration
- Test concurrent attach attempts (should fail with max sessions)

### Integration Tests

#### E2E Pause/Attach Flow (`tests/e2e/pause_attach.rs`)
1. Start daemon
2. Spawn agent with test task
3. CLI: pause agent
4. Verify process state (SIGSTOP)
5. CLI: attach, get credentials
6. Connect with test client, verify handshake
7. Send stdin, verify forwarded
8. CLI: resume
9. Verify process continues

### Manual Testing Steps

1. **Basic CLI Flow**
   - `descartes daemon &`
   - `descartes spawn --task "sleep 300" --name test`
   - `descartes ps` → verify running
   - `descartes pause <id>`
   - `ps aux | grep sleep` → verify "T" state
   - `descartes attach <id> --json` → save credentials
   - `descartes resume <id>`
   - Verify sleep continues

2. **GUI Flow**
   - Open GUI, go to Swarm Monitor
   - Verify agents displayed
   - Click Pause → verify button changes
   - Click Attach → verify modal with credentials
   - Click Resume → verify agent continues

---

## Performance Considerations

1. **RPC Connection Pooling**: CLI creates fresh connection per command. For high-frequency use, consider connection caching.

2. **Socket Cleanup**: Attach sockets must be cleaned up on:
   - Agent resume
   - Agent kill
   - Daemon shutdown
   - Token expiration

3. **Buffer Limits**: Output buffers in TUI handlers have 1MB/10k line limits to prevent memory exhaustion during long pauses.

---

## Migration Notes

No database schema changes required. Existing database tables work as-is.

The change is behavioral: CLI will require daemon to be running for pause/resume/attach. Add `--no-daemon` flag if database-only mode is needed for debugging.

---

## References

- Compilation fixes: `thoughts/shared/research/2025-11-26-daemon-compilation-fixes.md`
- Original plan: `thoughts/shared/plans/2025-01-25-subagent-pause-attach.md`
- Task breakdown: `thoughts/shared/plans/2025-01-25-subagent-pause-attach-tasks.md`
- Sprint assessment: `thoughts/shared/research/2025-11-24-descartes-sprint-assessment.md`
