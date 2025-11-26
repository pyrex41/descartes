# Subagent Pause, Connect & TUI Attachment Implementation Plan

## Overview

This plan implements the ability to pause running AI agents, expose their I/O streams for external attachment, and enable human operators to connect via Claude Code or OpenCode TUIs. The implementation spans core process control, RPC infrastructure, CLI commands, and GUI integration.

## Current State Analysis

### What Exists
- **State Machine** (`core/src/state_machine.rs:67-86`): `WorkflowState::Paused` exists with valid transitions
- **Agent Status** (`core/src/traits.rs:185`): `AgentStatus::Paused` exists but underutilized
- **Signal Handling** (`core/src/agent_runner.rs:386-463`): `AgentSignal::Interrupt` exists, sets status to `Paused`
- **LocalProcessRunner** (`core/src/agent_runner.rs:40-464`): Full process lifecycle with stdout/stderr buffering via mpsc channels
- **ZMQ Infrastructure** (`core/src/zmq_*.rs`): Distributed agent control with stdin/stdout protocol defined but not implemented server-side
- **RPC Server** (`daemon/src/rpc_server.rs`): Unix socket JSON-RPC with existing spawn/list/approve methods
- **Event Bus** (`daemon/src/events.rs`): Pub/sub system for agent lifecycle events

### Key Discoveries
- ZMQ already defines `WriteStdin`, `ReadStdout`, `ReadStderr` commands (`zmq_agent_runner.rs:154-160`) but server returns "not yet implemented" (`zmq_server.rs:534-552`)
- Stdout/stderr already buffered via unbounded mpsc channels (`agent_runner.rs:504-505`)
- SIGSTOP/SIGCONT require Unix-specific handling, not available on Windows
- Existing `GracefulShutdown` struct provides pattern for timeout-based control (`agent_runner.rs:832-893`)

## Desired End State

After implementation:
1. Users can pause any running agent via CLI (`descartes pause <agent_id>`) or RPC
2. Paused agents freeze execution but retain buffered I/O
3. Users can request attach credentials for paused agents
4. External TUIs (Claude Code, OpenCode) can connect to paused agents
5. Connected sessions receive historical output and can send stdin
6. Users can resume agents or detach without resuming
7. GUI displays pause/resume/attach buttons with real-time status
8. All operations emit structured events for observability

### Verification Criteria
- Unit tests for pause/resume state transitions
- Integration tests for attach token generation and validation
- E2E tests spawning real TUIs and verifying connection
- GUI screenshot tests showing pause/attach UI

## What We're NOT Doing

1. **Distributed pause orchestration** - Only local agent pause (ZMQ distributed pause is future work)
2. **Multi-user attachment** - Single attach session per paused agent initially
3. **Persistent attach sessions** - Attach tokens expire, no session persistence across daemon restarts
4. **Windows SIGSTOP equivalent** - Windows will only support cooperative pause
5. **SSH tunneling for OpenCode** - SSH support documented but deferred to Phase 5
6. **Streaming video/screen share** - Text-only I/O attachment

## Implementation Approach

We'll use a hybrid pause mechanism:
- **Cooperative pause**: Send pause notification via stdin, wait for acknowledgment
- **Forced pause (emergency stop)**: SIGSTOP on Unix (falls back to SIGTERM on Windows)

For attachment, we'll leverage the existing ZMQ infrastructure:
- Create dedicated ZMQ DEALER/ROUTER sockets for attached sessions
- Use `ipc://` transport for local, `tcp://` for remote (cross-platform)
- Implement stdin/stdout proxying in ZMQ server

---

## Phase 1: Core Pause/Resume Mechanism

### Overview
Implement proper pause/resume with cooperative and forced modes, updating state machine and process control.

### Changes Required

#### 1.1 Extend AgentSignal and AgentStatus

**File**: `descartes/core/src/traits.rs`
**Changes**: Add `ForcePause` signal variant and ensure `Paused` status is correctly used

```rust
// Around line 154-159, extend AgentSignal:
#[derive(Debug, Clone, Copy)]
pub enum AgentSignal {
    Interrupt,    // SIGINT - cooperative pause request
    Terminate,    // SIGTERM - graceful shutdown
    Kill,         // SIGKILL - force kill
    ForcePause,   // SIGSTOP - emergency freeze (Unix only)
    Resume,       // SIGCONT - resume from forced pause
}
```

#### 1.2 Implement Pause/Resume in LocalProcessRunner

**File**: `descartes/core/src/agent_runner.rs`
**Changes**: Add pause/resume methods with cooperative and forced modes

```rust
// Add to LocalProcessRunner impl (around line 385):

/// Pause an agent cooperatively or forcefully.
///
/// Cooperative mode sends a pause notification and waits for acknowledgment.
/// Forced mode uses SIGSTOP (Unix) to immediately freeze the process.
pub async fn pause(&self, agent_id: &Uuid, force: bool) -> AgentResult<()> {
    // Implementation:
    // 1. Get handle from registry
    // 2. If cooperative: write pause notification to stdin, set status to Paused
    // 3. If forced (Unix): send SIGSTOP, set status to Paused
    // 4. Emit AgentEvent::Paused
}

/// Resume a paused agent.
pub async fn resume(&self, agent_id: &Uuid) -> AgentResult<()> {
    // Implementation:
    // 1. Get handle, verify status is Paused
    // 2. If was force-paused (Unix): send SIGCONT
    // 3. Write resume notification to stdin
    // 4. Set status to Running
    // 5. Emit AgentEvent::Resumed
}
```

#### 1.3 Add Pause Metadata to AgentInfo

**File**: `descartes/core/src/traits.rs`
**Changes**: Track pause state for attach eligibility

```rust
// Around line 162, extend AgentInfo:
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfo {
    pub id: Uuid,
    pub name: String,
    pub status: AgentStatus,
    pub model_backend: String,
    pub started_at: std::time::SystemTime,
    pub task: String,
    // New fields:
    pub paused_at: Option<std::time::SystemTime>,
    pub pause_mode: Option<PauseMode>,
    pub attach_info: Option<AttachInfo>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum PauseMode {
    Cooperative,
    Forced,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttachInfo {
    pub connect_url: String,
    pub token: String,
    pub expires_at: i64,  // Unix timestamp
}
```

#### 1.4 Update AgentRunner Trait

**File**: `descartes/core/src/traits.rs`
**Changes**: Add pause/resume to trait interface

```rust
// Around line 125, extend AgentRunner trait:
#[async_trait]
pub trait AgentRunner: Send + Sync {
    async fn spawn(&self, config: AgentConfig) -> AgentResult<Box<dyn AgentHandle>>;
    async fn list_agents(&self) -> AgentResult<Vec<AgentInfo>>;
    async fn get_agent(&self, agent_id: &Uuid) -> AgentResult<Option<AgentInfo>>;
    async fn kill(&self, agent_id: &Uuid) -> AgentResult<()>;
    async fn signal(&self, agent_id: &Uuid, signal: AgentSignal) -> AgentResult<()>;
    // New methods:
    async fn pause(&self, agent_id: &Uuid, force: bool) -> AgentResult<()>;
    async fn resume(&self, agent_id: &Uuid) -> AgentResult<()>;
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo test -p descartes_core agent_runner` passes
- [ ] `cargo test -p descartes_core pause` passes (new tests)
- [ ] `cargo clippy -p descartes_core` has no new warnings
- [ ] `cargo build --release` succeeds

#### Manual Verification:
- [ ] Spawn agent, pause cooperatively, verify status shows "paused"
- [ ] Spawn agent, force pause (Unix), verify process frozen via `ps aux | grep <pid>`
- [ ] Resume paused agent, verify execution continues

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Attach Infrastructure

### Overview
Implement attachment socket creation, token generation, and stdin/stdout proxying via ZMQ.

### Changes Required

#### 2.1 Attach Token Generation

**File**: `descartes/core/src/attach.rs` (new file)
**Changes**: Token generation and validation

```rust
//! Attachment token management for paused agent sessions.

use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

/// Default TTL for attach tokens (5 minutes)
const DEFAULT_TOKEN_TTL_SECS: i64 = 300;

/// Attachment token with expiration
#[derive(Debug, Clone)]
pub struct AttachToken {
    pub token: String,
    pub agent_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

/// Token store for managing attach tokens
pub struct AttachTokenStore {
    tokens: Arc<RwLock<HashMap<String, AttachToken>>>,
    ttl_secs: i64,
}

impl AttachTokenStore {
    pub fn new() -> Self { /* ... */ }

    pub fn with_ttl(ttl_secs: i64) -> Self { /* ... */ }

    /// Generate a new attach token for an agent
    pub async fn generate(&self, agent_id: Uuid) -> AttachToken { /* ... */ }

    /// Validate a token, returning agent_id if valid
    pub async fn validate(&self, token: &str) -> Option<Uuid> { /* ... */ }

    /// Revoke a token
    pub async fn revoke(&self, token: &str) -> bool { /* ... */ }

    /// Cleanup expired tokens
    pub async fn cleanup_expired(&self) -> usize { /* ... */ }
}
```

#### 2.2 Implement ZMQ Stdin/Stdout Server-Side

**File**: `descartes/core/src/zmq_server.rs`
**Changes**: Complete the stdin/stdout implementation (currently returns "not implemented")

```rust
// Around line 534-552, replace the stub implementations:

ControlCommandType::WriteStdin => {
    let data = payload
        .get("data")
        .and_then(|d| d.as_str())
        .ok_or_else(|| AgentError::ExecutionError("Missing stdin data".into()))?;

    let decoded = base64::decode(data)
        .map_err(|e| AgentError::ExecutionError(format!("Base64 decode error: {}", e)))?;

    // Get agent handle and write to stdin
    if let Some(agent) = self.agents.get(&agent_id) {
        let agent_guard = agent.read().await;
        let mut stdin = agent_guard.stdin.lock().await;
        stdin.write_all(&decoded).await?;
        stdin.flush().await?;
        Ok(ControlResponse::success(json!({"bytes_written": decoded.len()})))
    } else {
        Err(AgentError::NotFound(format!("Agent {} not found", agent_id)))
    }
}

ControlCommandType::ReadStdout => {
    if let Some(agent) = self.agents.get(&agent_id) {
        let agent_guard = agent.read().await;
        let mut buffer = agent_guard.stdout_buffer.lock().await;
        let data: Vec<Vec<u8>> = std::iter::from_fn(|| buffer.try_recv().ok()).collect();
        let combined: Vec<u8> = data.into_iter().flatten().collect();
        let encoded = base64::encode(&combined);
        Ok(ControlResponse::success(json!({"data": encoded, "bytes": combined.len()})))
    } else {
        Err(AgentError::NotFound(format!("Agent {} not found", agent_id)))
    }
}
```

#### 2.3 Create Attach Session Manager

**File**: `descartes/daemon/src/attach_session.rs` (new file)
**Changes**: Manage active attach sessions

```rust
//! Attach session management for paused agents.

use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use descartes_core::attach::AttachToken;

/// Active attach session
pub struct AttachSession {
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub token: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub client_type: ClientType,
    pub zmq_endpoint: String,
}

#[derive(Debug, Clone)]
pub enum ClientType {
    ClaudeCode,
    OpenCode,
    Custom(String),
}

/// Manager for active attach sessions
pub struct AttachSessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, AttachSession>>>,
    token_store: Arc<descartes_core::attach::AttachTokenStore>,
}

impl AttachSessionManager {
    pub fn new(token_store: Arc<descartes_core::attach::AttachTokenStore>) -> Self { /* ... */ }

    /// Create a new attach session for an agent
    pub async fn create_session(&self, agent_id: Uuid, client_type: ClientType) -> Result<AttachSession, AttachError> { /* ... */ }

    /// Validate token and return session if valid
    pub async fn validate_and_get_session(&self, token: &str) -> Option<&AttachSession> { /* ... */ }

    /// Terminate an attach session
    pub async fn terminate_session(&self, session_id: Uuid) -> bool { /* ... */ }

    /// Get all active sessions for an agent
    pub async fn get_sessions_for_agent(&self, agent_id: Uuid) -> Vec<&AttachSession> { /* ... */ }
}
```

#### 2.4 Add Attach-Related Events

**File**: `descartes/daemon/src/events.rs`
**Changes**: Add attach-specific event types

```rust
// Around line 48-69, extend AgentEventType:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEventType {
    Spawned,
    Started,
    StatusChanged,
    Paused,        // New
    Resumed,       // New
    Completed,
    Failed,
    Killed,
    Log,
    Metric,
    AttachRequested,   // New
    AttachConnected,   // New
    AttachDisconnected, // New
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo test -p descartes_core attach` passes
- [ ] `cargo test -p descartes_daemon attach_session` passes
- [ ] Token generation produces valid UUIDs with correct TTL
- [ ] Token validation rejects expired/revoked tokens
- [ ] ZMQ stdin/stdout tests pass: `cargo test -p descartes_core zmq_server`

#### Manual Verification:
- [ ] Generate attach token for paused agent via RPC
- [ ] Connect to ZMQ endpoint with token, send stdin data
- [ ] Verify stdout data is received via ZMQ

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: RPC & CLI Integration

### Overview
Expose pause/resume/attach functionality via RPC methods and CLI commands.

### Changes Required

#### 3.1 Add RPC Methods

**File**: `descartes/daemon/src/rpc_server.rs`
**Changes**: Add agent.pause, agent.resume, agent.attach.* methods

```rust
// Extend DescartesRpc trait (around line 29):
#[rpc(server)]
pub trait DescartesRpc {
    // ... existing methods ...

    /// Pause a running agent
    #[method(name = "agent.pause")]
    async fn pause_agent(
        &self,
        agent_id: String,
        force: bool,
    ) -> Result<PauseResult, ErrorObjectOwned>;

    /// Resume a paused agent
    #[method(name = "agent.resume")]
    async fn resume_agent(
        &self,
        agent_id: String,
    ) -> Result<ResumeResult, ErrorObjectOwned>;

    /// Request attach credentials for a paused agent
    #[method(name = "agent.attach.request")]
    async fn request_attach(
        &self,
        agent_id: String,
        client_type: String,
    ) -> Result<AttachCredentials, ErrorObjectOwned>;

    /// Validate attach token
    #[method(name = "agent.attach.validate")]
    async fn validate_attach(
        &self,
        token: String,
    ) -> Result<AttachValidation, ErrorObjectOwned>;

    /// Revoke attach token
    #[method(name = "agent.attach.revoke")]
    async fn revoke_attach(
        &self,
        token: String,
    ) -> Result<bool, ErrorObjectOwned>;
}
```

#### 3.2 Add Response Types

**File**: `descartes/daemon/src/types.rs`
**Changes**: Add RPC response types for pause/attach operations

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseResult {
    pub agent_id: String,
    pub paused_at: i64,
    pub pause_mode: String,  // "cooperative" or "forced"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeResult {
    pub agent_id: String,
    pub resumed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachCredentials {
    pub agent_id: String,
    pub token: String,
    pub connect_url: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachValidation {
    pub valid: bool,
    pub agent_id: Option<String>,
    pub expires_at: Option<i64>,
}
```

#### 3.3 Add CLI Commands

**File**: `descartes/cli/src/commands/mod.rs`
**Changes**: Add pause, resume, attach modules

```rust
pub mod init;
pub mod kill;
pub mod logs;
pub mod ps;
pub mod spawn;
pub mod plugins;
pub mod pause;    // New
pub mod resume;   // New
pub mod attach;   // New
```

**File**: `descartes/cli/src/commands/pause.rs` (new file)
```rust
//! Pause a running agent.

use clap::Args;
use uuid::Uuid;
use descartes_daemon::client::RpcClient;

#[derive(Args)]
pub struct PauseArgs {
    /// Agent ID to pause
    #[arg(short, long)]
    agent_id: String,

    /// Force pause using SIGSTOP (emergency stop)
    #[arg(short, long, default_value = "false")]
    force: bool,
}

pub async fn execute(args: PauseArgs, config: &Config) -> anyhow::Result<()> {
    let client = RpcClient::new(&config.daemon_url)?;
    let result = client.call("agent.pause", json!({
        "agent_id": args.agent_id,
        "force": args.force,
    })).await?;

    println!("Agent {} paused at {}", result.agent_id, result.paused_at);
    if args.force {
        println!("Mode: FORCED (emergency stop)");
    }
    Ok(())
}
```

**File**: `descartes/cli/src/commands/attach.rs` (new file)
```rust
//! Attach to a paused agent.

use clap::Args;

#[derive(Args)]
pub struct AttachArgs {
    /// Agent ID to attach to
    #[arg(short, long)]
    agent_id: String,

    /// Client type: claude-code, opencode, or custom
    #[arg(short, long, default_value = "claude-code")]
    client: String,

    /// Auto-launch the TUI client
    #[arg(short, long, default_value = "true")]
    launch: bool,

    /// Just print connection info, don't launch
    #[arg(long)]
    info_only: bool,
}

pub async fn execute(args: AttachArgs, config: &Config) -> anyhow::Result<()> {
    let client = RpcClient::new(&config.daemon_url)?;

    // Request attach credentials
    let creds = client.call("agent.attach.request", json!({
        "agent_id": args.agent_id,
        "client_type": args.client,
    })).await?;

    if args.info_only {
        println!("Connect URL: {}", creds.connect_url);
        println!("Token: {}", creds.token);
        println!("Expires: {}", creds.expires_at);
        return Ok(());
    }

    if args.launch {
        match args.client.as_str() {
            "claude-code" => launch_claude_code(&creds).await?,
            "opencode" => launch_opencode(&creds).await?,
            _ => {
                println!("Unknown client type. Connection info:");
                println!("Connect URL: {}", creds.connect_url);
                println!("Token: {}", creds.token);
            }
        }
    }

    Ok(())
}
```

#### 3.4 Update CLI Main

**File**: `descartes/cli/src/main.rs`
**Changes**: Add pause/resume/attach subcommands

```rust
// Around line 122, add to Commands enum:
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    /// Pause a running agent
    Pause(commands::pause::PauseArgs),
    /// Resume a paused agent
    Resume(commands::resume::ResumeArgs),
    /// Attach to a paused agent
    Attach(commands::attach::AttachArgs),
}

// In the match block (around line 149):
Commands::Pause(args) => commands::pause::execute(args, &config).await?,
Commands::Resume(args) => commands::resume::execute(args, &config).await?,
Commands::Attach(args) => commands::attach::execute(args, &config).await?,
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo test -p descartes_cli` passes
- [ ] `cargo test -p descartes_daemon rpc` passes
- [ ] `descartes --help` shows pause, resume, attach commands
- [ ] RPC method dispatch tests pass for new methods

#### Manual Verification:
- [ ] `descartes spawn --task "test" --name "test-agent"` works
- [ ] `descartes pause --agent-id <id>` pauses agent
- [ ] `descartes ps` shows agent as "paused"
- [ ] `descartes attach --agent-id <id> --info-only` prints connection info
- [ ] `descartes resume --agent-id <id>` resumes agent

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 4: Claude Code TUI Attachment

### Overview
Implement the launch flow for connecting Claude Code to a paused agent.

### Changes Required

#### 4.1 Claude Code Launcher

**File**: `descartes/cli/src/tui_launchers/mod.rs` (new file)
```rust
pub mod claude_code;
pub mod opencode;
```

**File**: `descartes/cli/src/tui_launchers/claude_code.rs` (new file)
```rust
//! Claude Code TUI launcher for attaching to paused agents.

use crate::types::AttachCredentials;
use std::process::Command;
use anyhow::Result;

/// Launch Claude Code with connection to a paused agent.
pub async fn launch(creds: &AttachCredentials) -> Result<()> {
    // Build environment for Claude Code
    let mut cmd = Command::new("claude");

    // Set environment variables for connection
    cmd.env("DESCARTES_ATTACH_URL", &creds.connect_url);
    cmd.env("DESCARTES_ATTACH_TOKEN", &creds.token);
    cmd.env("DESCARTES_AGENT_ID", &creds.agent_id);

    // Add resume flag for session bootstrap
    cmd.arg("--resume");
    cmd.arg("--session-url").arg(&creds.connect_url);

    // Spawn and wait
    println!("Launching Claude Code...");
    println!("Connecting to agent: {}", creds.agent_id);

    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Claude Code exited with status: {}", status);
    }

    Ok(())
}

/// Check if Claude Code is available
pub fn is_available() -> bool {
    which::which("claude").is_ok()
}
```

#### 4.2 Session Bootstrap Protocol

**File**: `descartes/core/src/attach_protocol.rs` (new file)
```rust
//! Protocol for bootstrapping attached TUI sessions.

use serde::{Deserialize, Serialize};

/// Initial handshake message from client
#[derive(Debug, Serialize, Deserialize)]
pub struct AttachHandshake {
    pub version: String,
    pub token: String,
    pub client_type: String,
    pub client_version: String,
}

/// Server response to handshake
#[derive(Debug, Serialize, Deserialize)]
pub struct AttachHandshakeResponse {
    pub success: bool,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub buffered_output_lines: usize,
    pub error: Option<String>,
}

/// Historical output sent after successful handshake
#[derive(Debug, Serialize, Deserialize)]
pub struct HistoricalOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub timestamp_start: i64,
    pub timestamp_end: i64,
}
```

#### 4.3 ZMQ Attach Endpoint

**File**: `descartes/daemon/src/attach_endpoint.rs` (new file)
```rust
//! ZMQ endpoint for TUI attachment.

use zeromq::{Socket, SocketRecv, SocketSend};
use descartes_core::attach_protocol::*;

pub struct AttachEndpoint {
    router: zeromq::RouterSocket,
    session_manager: Arc<AttachSessionManager>,
    agent_runner: Arc<dyn AgentRunner>,
}

impl AttachEndpoint {
    pub async fn new(bind_addr: &str, session_manager: Arc<AttachSessionManager>, agent_runner: Arc<dyn AgentRunner>) -> Result<Self> {
        let mut router = zeromq::RouterSocket::new();
        router.bind(bind_addr).await?;

        Ok(Self {
            router,
            session_manager,
            agent_runner,
        })
    }

    /// Run the attachment endpoint
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let msg = self.router.recv().await?;
            let client_id = msg.get(0).unwrap().to_vec();
            let payload = msg.get(1).unwrap();

            match self.handle_message(&client_id, payload).await {
                Ok(response) => {
                    self.router.send(vec![client_id.into(), response.into()].into()).await?;
                }
                Err(e) => {
                    let error_response = json!({"error": e.to_string()});
                    self.router.send(vec![client_id.into(), error_response.to_string().into()].into()).await?;
                }
            }
        }
    }

    async fn handle_message(&self, client_id: &[u8], payload: &[u8]) -> Result<String> {
        let msg: serde_json::Value = serde_json::from_slice(payload)?;

        match msg.get("type").and_then(|t| t.as_str()) {
            Some("handshake") => self.handle_handshake(client_id, msg).await,
            Some("stdin") => self.handle_stdin(client_id, msg).await,
            Some("read_output") => self.handle_read_output(client_id, msg).await,
            Some("disconnect") => self.handle_disconnect(client_id, msg).await,
            _ => Err(anyhow::anyhow!("Unknown message type")),
        }
    }
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo test -p descartes_cli tui_launchers` passes
- [ ] `cargo test -p descartes_daemon attach_endpoint` passes
- [ ] `cargo test -p descartes_core attach_protocol` passes

#### Manual Verification:
- [ ] Pause an agent, run `descartes attach --agent-id <id> --client claude-code`
- [ ] Verify Claude Code launches
- [ ] Verify Claude Code shows agent context/history
- [ ] Type in Claude Code, verify stdin reaches agent
- [ ] Verify stdout from agent appears in Claude Code

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 5: OpenCode TUI Attachment

### Overview
Implement OpenCode-specific attachment with SSH support preparation.

### Changes Required

#### 5.1 OpenCode Launcher

**File**: `descartes/cli/src/tui_launchers/opencode.rs` (new file)
```rust
//! OpenCode TUI launcher for attaching to paused agents.

use crate::types::AttachCredentials;
use std::process::Command;
use anyhow::Result;

/// Launch OpenCode with connection to a paused agent.
pub async fn launch(creds: &AttachCredentials) -> Result<()> {
    let mut cmd = Command::new("opencode");

    // Set environment variables
    cmd.env("DESCARTES_ATTACH_URL", &creds.connect_url);
    cmd.env("DESCARTES_ATTACH_TOKEN", &creds.token);
    cmd.env("DESCARTES_AGENT_ID", &creds.agent_id);

    // OpenCode-specific flags
    cmd.arg("--attach");
    cmd.arg("--url").arg(&creds.connect_url);
    cmd.arg("--token").arg(&creds.token);

    println!("Launching OpenCode...");
    println!("Connecting to agent: {}", creds.agent_id);

    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("OpenCode exited with status: {}", status);
    }

    Ok(())
}

/// Check if OpenCode is available
pub fn is_available() -> bool {
    which::which("opencode").is_ok()
}
```

#### 5.2 SSH Tunnel Preparation (Documentation Only)

**File**: `descartes/docs/SSH_ATTACHMENT.md` (new file)
```markdown
# SSH-Based Remote Attachment

This document describes the planned SSH tunnel support for remote agent attachment.

## Overview

For scenarios where the Descartes daemon runs on a remote machine, users can
attach via SSH tunnel to forward the ZMQ socket.

## Usage (Future)

```bash
# On remote machine, pause agent
descartes pause --agent-id <id>

# Get attach info
descartes attach --agent-id <id> --info-only
# Output: Connect URL: ipc:///tmp/descartes-agent-<id>.sock

# On local machine, create SSH tunnel
ssh -L /tmp/local-agent.sock:/tmp/descartes-agent-<id>.sock user@remote

# Attach locally
descartes attach --agent-id <id> --connect-url ipc:///tmp/local-agent.sock
```

## Implementation Notes

- Requires SSH ControlMaster for socket forwarding
- Alternative: Use TCP socket with SSH port forwarding
- Consider: Built-in SSH tunnel management in CLI
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo test -p descartes_cli tui_launchers::opencode` passes
- [ ] OpenCode launcher handles missing binary gracefully

#### Manual Verification:
- [ ] Pause an agent, run `descartes attach --agent-id <id> --client opencode`
- [ ] Verify OpenCode launches (if installed)
- [ ] Verify connection and I/O work

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 6: GUI & Telemetry Integration

### Overview
Add pause/resume/attach buttons to GUI and emit structured telemetry events.

### Changes Required

#### 6.1 GUI Pause/Resume Buttons

**File**: `descartes/gui/src/agent_panel.rs` (or equivalent)
**Changes**: Add pause/resume/attach buttons to agent panel

```rust
// Add to agent panel UI:
fn view_agent_controls(&self, agent: &AgentInfo) -> Element<Message> {
    let mut controls = Row::new().spacing(10);

    match agent.status {
        AgentStatus::Running | AgentStatus::Thinking => {
            controls = controls.push(
                Button::new(Text::new("Pause"))
                    .on_press(Message::PauseAgent(agent.id))
            );
        }
        AgentStatus::Paused => {
            controls = controls
                .push(
                    Button::new(Text::new("Resume"))
                        .on_press(Message::ResumeAgent(agent.id))
                )
                .push(
                    Button::new(Text::new("Attach"))
                        .on_press(Message::AttachToAgent(agent.id))
                );
        }
        _ => {}
    }

    controls.into()
}
```

#### 6.2 GUI Message Handling

**File**: `descartes/gui/src/event_handler.rs`
**Changes**: Handle pause/resume/attach messages

```rust
// Add message variants:
pub enum Message {
    // ... existing variants ...
    PauseAgent(Uuid),
    ResumeAgent(Uuid),
    AttachToAgent(Uuid),
    AttachCredentialsReceived(AttachCredentials),
}

// Add handlers:
Message::PauseAgent(agent_id) => {
    // Call RPC to pause agent
    Command::perform(
        async move { rpc_client.pause_agent(agent_id, false).await },
        |result| Message::AgentPaused(result)
    )
}

Message::AttachToAgent(agent_id) => {
    // Request attach credentials, then launch TUI
    Command::perform(
        async move { rpc_client.request_attach(agent_id, "claude-code").await },
        |result| Message::AttachCredentialsReceived(result)
    )
}
```

#### 6.3 Telemetry Events

**File**: `descartes/daemon/src/telemetry.rs` (new or extend existing)
**Changes**: Emit structured events for pause/attach operations

```rust
/// Telemetry event types for pause/attach operations
#[derive(Debug, Serialize)]
pub enum TelemetryEvent {
    AgentPaused {
        agent_id: Uuid,
        pause_mode: String,
        timestamp: i64,
    },
    AgentResumed {
        agent_id: Uuid,
        pause_duration_secs: i64,
        timestamp: i64,
    },
    AttachSessionStarted {
        agent_id: Uuid,
        client_type: String,
        timestamp: i64,
    },
    AttachSessionEnded {
        agent_id: Uuid,
        session_duration_secs: i64,
        stdin_bytes: usize,
        stdout_bytes: usize,
        timestamp: i64,
    },
}
```

#### 6.4 Metrics Collection

**File**: `descartes/daemon/src/metrics.rs`
**Changes**: Add pause/attach metrics

```rust
// Add metrics:
pub struct PauseMetrics {
    pub total_pauses: AtomicU64,
    pub cooperative_pauses: AtomicU64,
    pub forced_pauses: AtomicU64,
    pub total_resumes: AtomicU64,
    pub pause_duration_histogram: Histogram,
}

pub struct AttachMetrics {
    pub total_attach_requests: AtomicU64,
    pub active_sessions: AtomicU64,
    pub tokens_generated: AtomicU64,
    pub tokens_revoked: AtomicU64,
    pub session_duration_histogram: Histogram,
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo test -p descartes_gui` passes
- [ ] GUI compiles with new buttons
- [ ] Telemetry events serialize correctly

#### Manual Verification:
- [ ] Open GUI, spawn agent, see "Pause" button
- [ ] Click Pause, verify button changes to "Resume" and "Attach"
- [ ] Click Attach, verify TUI launches
- [ ] Check telemetry output shows pause/attach events

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding.

---

## Testing Strategy

### Unit Tests

#### Core Pause/Resume Tests (`core/tests/pause_tests.rs`)
- Test cooperative pause sends correct stdin message
- Test forced pause sends SIGSTOP (Unix)
- Test resume sends SIGCONT after forced pause
- Test resume sends stdin message after cooperative pause
- Test state transitions: Running → Paused → Running
- Test invalid transitions rejected (e.g., Completed → Paused)

#### Token Tests (`core/tests/attach_token_tests.rs`)
- Test token generation creates valid UUID
- Test token expiration after TTL
- Test token revocation
- Test concurrent token operations
- Test cleanup removes expired tokens

#### ZMQ Tests (`core/tests/zmq_attach_tests.rs`)
- Test WriteStdin delivers data to agent stdin
- Test ReadStdout returns buffered data
- Test handshake protocol validation
- Test invalid token rejection

### Integration Tests

#### E2E Pause/Attach Tests (`tests/e2e/pause_attach_test.rs`)
1. Spawn agent with mock task
2. Pause agent
3. Request attach credentials
4. Connect via ZMQ with token
5. Send stdin data
6. Verify stdout received
7. Disconnect
8. Resume agent
9. Verify agent continues execution

### Manual Testing Steps

1. **Basic Pause/Resume**
   - `descartes spawn --task "echo test && sleep 60" --name test-agent`
   - `descartes ps` → verify running
   - `descartes pause --agent-id <id>`
   - `descartes ps` → verify paused
   - `descartes resume --agent-id <id>`
   - `descartes ps` → verify running

2. **Attach with Claude Code**
   - Pause agent
   - `descartes attach --agent-id <id> --client claude-code`
   - Verify Claude Code opens
   - Type command, verify response
   - Exit Claude Code
   - Resume agent

3. **Force Pause (Emergency Stop)**
   - Start long-running agent
   - `descartes pause --agent-id <id> --force`
   - Verify with `ps aux | grep <pid>` shows "T" (stopped)
   - `descartes resume --agent-id <id>`
   - Verify process continues

---

## Performance Considerations

1. **Buffer Management**: Stdout/stderr buffers use unbounded channels. For long pauses, add max buffer size with LRU eviction.

2. **Token Cleanup**: Run periodic cleanup of expired tokens (every 60s) to prevent memory growth.

3. **ZMQ Socket Lifecycle**: Create attach sockets lazily on first attach request, destroy after last client disconnects.

4. **Concurrency**: Paused agents consume memory but not CPU. Consider adding `--release-slot` flag to free concurrency slot while paused.

---

## Migration Notes

No database migrations required. All new state is runtime-only:
- Pause status stored in `AgentInfo` in-memory
- Attach tokens stored in `AttachTokenStore` in-memory
- Sessions tracked in `AttachSessionManager`

For persistence across daemon restarts, consider adding optional SQLite storage for:
- Active pause states (to allow resume after restart)
- Long-lived attach tokens

---

## References

- Subagent Pause PRD: `.taskmaster/docs/subagent_pause_and_connect_prd.md`
- Claude Code TUI PRD: `.taskmaster/docs/claude_code_tui_attach_prd.md`
- OpenCode TUI PRD: `.taskmaster/docs/opencode_tui_attach_prd.md`
- Existing ZMQ protocol: `descartes/core/src/zmq_agent_runner.rs:138-176`
- Agent runner: `descartes/core/src/agent_runner.rs`
- RPC server: `descartes/daemon/src/rpc_server.rs`
