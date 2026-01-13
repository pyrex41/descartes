# ZMQ Chat Streaming Implementation Plan

## Overview

Transform the GUI chat from direct Claude CLI execution to a daemon-managed chat system using ZMQ for real-time streaming. The daemon spawns and manages CLI processes, streams output (including thinking) via ZMQ PUB/SUB, and supports upgrading chat sessions to full agent mode with sub-agents.

## Current State Analysis

### GUI Chat (`gui/src/main.rs:1782-1885`)
- Directly spawns `claude --print <prompt>` process
- Waits for completion before showing output (no streaming)
- No thinking output visible
- Bypasses daemon entirely

### Daemon
- Has attach session system for TUI clients (`daemon/src/claude_code_tui.rs`)
- Has ZMQ infrastructure in core (`core/src/zmq_*.rs`)
- Missing: PUB/SUB socket implementation (only REQ/REP exists)
- Missing: Chat session management

### ZMQ Infrastructure (`core/src/zmq_*.rs`)
- REQ/REP pattern implemented for agent control
- MessagePack serialization
- Message types defined but no chat-specific types
- PUB/SUB mentioned in docs but NOT implemented

### Claude Code CLI Options (from research)
- `--output-format stream-json`: NDJSON streaming with every token
- `--include-partial-messages`: Enable partial streaming events
- Extended thinking via trigger words ("think", "think hard", "ultrathink")
- `--betas interleaved-thinking`: Beta flag for thinking

## Desired End State

After implementation:
1. GUI chat sends prompts to daemon via RPC
2. Daemon spawns CLI process with `--output-format stream-json`
3. Daemon parses stream-json and publishes chunks via ZMQ PUB socket
4. GUI subscribes to ZMQ PUB and displays streaming output with thinking
5. Chat sessions can be "upgraded" to agent mode (same process, different consumption)
6. Architecture supports future SQLite persistence and OpenCode backend

### Key Discoveries
- `core/src/zmq_agent_runner.rs:536-580`: ZmqMessage enum needs chat message types
- `core/src/zmq_server.rs:701-734`: Status update task exists but doesn't use PUB socket
- `daemon/src/claude_code_tui.rs:350-464`: I/O loop pattern we can adapt for stream-json parsing
- Claude CLI `stream-json` outputs NDJSON with thinking blocks included

## What We're NOT Doing

- Full OpenCode implementation (trait only, Claude impl first)
- Session persistence to SQLite (ephemeral only, design for future)
- WebSocket transport (ZMQ only for now)
- MCP server integration in chat (future enhancement)
- Multi-user/auth (single user assumed)

## Implementation Approach

Single CLI process model: The daemon spawns one CLI process per chat session. The same process can be consumed as "chat mode" (streaming to GUI) or "agent mode" (full orchestration). The difference is in how the frontend consumes the output, not in the backend process.

ZMQ topology:
- Port 19280: HTTP RPC (existing)
- Port 19380: Reserved for WebSocket (unused for now)
- Port 19480: ZMQ PUB socket for streaming (NEW)

---

## Phase 1: Define Chat Protocol and CliBackend Trait

### Overview
Define the message types for chat streaming and create the `CliBackend` trait abstraction.

### Changes Required

#### 1. New file: `core/src/cli_backend.rs`

```rust
//! CLI Backend abstraction for Claude Code, OpenCode, etc.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Stream chunk from CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    /// Text content (assistant response)
    Text { content: String },
    /// Thinking/reasoning block
    Thinking { content: String },
    /// Tool use started
    ToolUseStart { tool_name: String, tool_id: String },
    /// Tool use input
    ToolUseInput { tool_id: String, input: serde_json::Value },
    /// Tool result
    ToolResult { tool_id: String, result: String, is_error: bool },
    /// Turn complete
    TurnComplete { turn_number: u32 },
    /// Session complete
    Complete { exit_code: i32 },
    /// Error
    Error { message: String },
}

/// Configuration for a chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionConfig {
    /// Working directory for the CLI
    pub working_dir: String,
    /// Enable extended thinking
    pub enable_thinking: bool,
    /// Thinking budget level: "normal", "hard", "harder", "ultra"
    pub thinking_level: String,
    /// Maximum turns (0 = unlimited)
    pub max_turns: u32,
    /// Additional CLI flags
    pub extra_flags: Vec<String>,
}

impl Default for ChatSessionConfig {
    fn default() -> Self {
        Self {
            working_dir: ".".to_string(),
            enable_thinking: true,
            thinking_level: "normal".to_string(),
            max_turns: 0,
            extra_flags: vec![],
        }
    }
}

/// Result of starting a chat session
#[derive(Debug, Clone)]
pub struct ChatSessionHandle {
    pub session_id: Uuid,
    pub stream_rx: mpsc::UnboundedReceiver<StreamChunk>,
}

/// Trait for CLI backends (Claude Code, OpenCode, etc.)
#[async_trait]
pub trait CliBackend: Send + Sync {
    /// Get the backend name (e.g., "claude", "opencode")
    fn name(&self) -> &str;

    /// Check if the CLI is available on the system
    async fn is_available(&self) -> bool;

    /// Get the CLI version
    async fn version(&self) -> Result<String, String>;

    /// Start a new chat session with the given config
    /// Returns a handle with the session ID and stream receiver
    async fn start_session(
        &self,
        config: ChatSessionConfig,
    ) -> Result<ChatSessionHandle, String>;

    /// Send a prompt to an existing session
    async fn send_prompt(
        &self,
        session_id: Uuid,
        prompt: String,
    ) -> Result<(), String>;

    /// Stop a session gracefully
    async fn stop_session(&self, session_id: Uuid) -> Result<(), String>;

    /// Kill a session forcefully
    async fn kill_session(&self, session_id: Uuid) -> Result<(), String>;

    /// Check if a session is active
    fn is_session_active(&self, session_id: Uuid) -> bool;
}
```

#### 2. New file: `core/src/claude_backend.rs`

```rust
//! Claude Code CLI backend implementation

use crate::cli_backend::{ChatSessionConfig, ChatSessionHandle, CliBackend, StreamChunk};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Claude Code stream-json message types
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeStreamMessage {
    #[serde(rename = "assistant")]
    Assistant { message: AssistantMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart { content_block: ContentBlock },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_start")]
    MessageStart { message: MessageInfo },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaInfo },
    #[serde(rename = "message_stop")]
    MessageStop {},
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Vec<ContentItem>,
}

#[derive(Debug, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    item_type: String,
    text: Option<String>,
    thinking: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
}

#[derive(Debug, Deserialize)]
struct ContentDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
    thinking: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageInfo {
    id: String,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaInfo {
    stop_reason: Option<String>,
}

/// Active chat session
struct ActiveSession {
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
}

/// Claude Code backend
pub struct ClaudeBackend {
    sessions: Arc<DashMap<Uuid, ActiveSession>>,
}

impl ClaudeBackend {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    fn build_command(&self, config: &ChatSessionConfig) -> Command {
        let mut cmd = Command::new("claude");

        // Core streaming flags
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--include-partial-messages");

        // Thinking flags
        if config.enable_thinking {
            cmd.arg("--betas").arg("interleaved-thinking");
        }

        // Max turns
        if config.max_turns > 0 {
            cmd.arg("--max-turns").arg(config.max_turns.to_string());
        }

        // Extra flags
        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        // Working directory
        cmd.current_dir(&config.working_dir);

        // I/O setup
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd
    }

    fn parse_stream_line(&self, line: &str) -> Option<StreamChunk> {
        let msg: ClaudeStreamMessage = serde_json::from_str(line).ok()?;

        match msg {
            ClaudeStreamMessage::ContentBlockDelta { delta } => {
                if let Some(text) = delta.text {
                    Some(StreamChunk::Text { content: text })
                } else if let Some(thinking) = delta.thinking {
                    Some(StreamChunk::Thinking { content: thinking })
                } else {
                    None
                }
            }
            ClaudeStreamMessage::MessageStop {} => {
                Some(StreamChunk::TurnComplete { turn_number: 0 })
            }
            _ => None,
        }
    }
}

impl Default for ClaudeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CliBackend for ClaudeBackend {
    fn name(&self) -> &str {
        "claude"
    }

    async fn is_available(&self) -> bool {
        which::which("claude").is_ok()
    }

    async fn version(&self) -> Result<String, String> {
        let output = Command::new("claude")
            .arg("--version")
            .output()
            .await
            .map_err(|e| e.to_string())?;

        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_string())
            .map_err(|e| e.to_string())
    }

    async fn start_session(
        &self,
        config: ChatSessionConfig,
    ) -> Result<ChatSessionHandle, String> {
        let session_id = Uuid::new_v4();
        let mut cmd = self.build_command(&config);

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn claude: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();

        // Stdin forwarding task
        let mut stdin = child.stdin.take().ok_or("Failed to capture stdin")?;
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(input) = stdin_rx.recv().await {
                if stdin.write_all(input.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        // Stdout parsing task
        let sessions = self.sessions.clone();
        let sid = session_id;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                // Try to parse as stream-json
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    // Check for thinking content
                    if let Some(delta) = msg.get("delta") {
                        if let Some(thinking) = delta.get("thinking").and_then(|t| t.as_str()) {
                            let _ = stream_tx.send(StreamChunk::Thinking {
                                content: thinking.to_string(),
                            });
                        } else if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            let _ = stream_tx.send(StreamChunk::Text {
                                content: text.to_string(),
                            });
                        }
                    }
                    // Check for message stop
                    if msg.get("type").and_then(|t| t.as_str()) == Some("message_stop") {
                        let _ = stream_tx.send(StreamChunk::TurnComplete { turn_number: 0 });
                    }
                }
            }

            // Process exited
            let _ = stream_tx.send(StreamChunk::Complete { exit_code: 0 });
            sessions.remove(&sid);
        });

        self.sessions.insert(session_id, ActiveSession {
            child,
            stdin_tx,
        });

        Ok(ChatSessionHandle {
            session_id,
            stream_rx,
        })
    }

    async fn send_prompt(&self, session_id: Uuid, prompt: String) -> Result<(), String> {
        let session = self.sessions.get(&session_id)
            .ok_or("Session not found")?;

        session.stdin_tx.send(prompt)
            .map_err(|e| format!("Failed to send prompt: {}", e))
    }

    async fn stop_session(&self, session_id: Uuid) -> Result<(), String> {
        if let Some((_, mut session)) = self.sessions.remove(&session_id) {
            // Send SIGTERM
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                if let Some(pid) = session.child.id() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                }
            }
            let _ = session.child.wait().await;
        }
        Ok(())
    }

    async fn kill_session(&self, session_id: Uuid) -> Result<(), String> {
        if let Some((_, mut session)) = self.sessions.remove(&session_id) {
            let _ = session.child.kill().await;
        }
        Ok(())
    }

    fn is_session_active(&self, session_id: Uuid) -> bool {
        self.sessions.contains_key(&session_id)
    }
}
```

#### 3. Update `core/src/lib.rs`

Add exports:
```rust
pub mod cli_backend;
pub mod claude_backend;

pub use cli_backend::{ChatSessionConfig, ChatSessionHandle, CliBackend, StreamChunk};
pub use claude_backend::ClaudeBackend;
```

### Success Criteria

#### Automated Verification:
- [x] `cargo build -p descartes-core` compiles without errors
- [x] `cargo test -p descartes-core` passes
- [ ] `cargo clippy -p descartes-core` has no warnings

#### Manual Verification:
- [ ] Unit test confirms ClaudeBackend can detect `claude` CLI availability

---

## Phase 2: Add ZMQ PUB Socket to Daemon

### Overview
Add a ZMQ PUB socket to the daemon for broadcasting stream chunks to subscribers.

### Changes Required

#### 1. Update `daemon/src/config.rs`

Add PUB port configuration:
```rust
// In ServerConfig struct, add:
/// ZMQ PUB socket port for streaming
pub pub_port: u16,

// In Default impl:
pub_port: 19480,
```

#### 2. Update `daemon/src/main.rs`

Add CLI flag:
```rust
/// ZMQ PUB socket port
#[arg(long, help = "ZMQ PUB socket port (default: 19480)")]
pub_port: Option<u16>,

// In config setup:
if let Some(port) = args.pub_port {
    config.server.pub_port = port;
}
```

#### 3. New file: `daemon/src/zmq_publisher.rs`

```rust
//! ZMQ PUB socket for streaming chat output

use descartes_core::StreamChunk;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use zeromq::{PubSocket, Socket, SocketRecv, SocketSend};

/// ZMQ Publisher for streaming chat chunks
pub struct ZmqPublisher {
    socket: Arc<RwLock<PubSocket>>,
    endpoint: String,
}

impl ZmqPublisher {
    pub async fn new(port: u16) -> Result<Self, String> {
        let endpoint = format!("tcp://0.0.0.0:{}", port);
        let mut socket = PubSocket::new();

        socket.bind(&endpoint)
            .await
            .map_err(|e| format!("Failed to bind PUB socket: {}", e))?;

        tracing::info!("ZMQ PUB socket listening on {}", endpoint);

        Ok(Self {
            socket: Arc::new(RwLock::new(socket)),
            endpoint,
        })
    }

    /// Publish a stream chunk for a session
    /// Topic format: "chat/{session_id}"
    pub async fn publish(&self, session_id: Uuid, chunk: &StreamChunk) -> Result<(), String> {
        let topic = format!("chat/{}", session_id);
        let payload = serde_json::to_vec(chunk)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // ZMQ PUB message format: [topic, payload]
        let mut socket = self.socket.write().await;

        // Send topic frame
        socket.send(topic.as_bytes().into())
            .await
            .map_err(|e| format!("Failed to send topic: {}", e))?;

        // Send payload frame
        socket.send(payload.into())
            .await
            .map_err(|e| format!("Failed to send payload: {}", e))?;

        Ok(())
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
```

#### 4. Update `daemon/src/server.rs`

Add publisher to RpcServer:
```rust
// In RpcServer struct:
publisher: Option<Arc<ZmqPublisher>>,

// In RpcServer::new():
let publisher = ZmqPublisher::new(config.server.pub_port)
    .await
    .map_err(|e| DaemonError::ServerError(e))?;

// Store as Some(Arc::new(publisher))
```

### Success Criteria

#### Automated Verification:
- [x] `cargo build -p descartes-daemon` compiles
- [ ] Daemon starts and logs "ZMQ PUB socket listening on tcp://0.0.0.0:19480"

#### Manual Verification:
- [ ] Can connect to PUB socket with `zmq` CLI tool

---

## Phase 3: Daemon Chat Session Manager

### Overview
Create a chat session manager that uses the CliBackend to spawn sessions and streams output via ZMQ.

### Changes Required

#### 1. New file: `daemon/src/chat_manager.rs`

```rust
//! Chat session management

use crate::zmq_publisher::ZmqPublisher;
use descartes_core::{ChatSessionConfig, CliBackend, ClaudeBackend, StreamChunk};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Active chat session info
#[derive(Debug, Clone)]
pub struct ChatSessionInfo {
    pub session_id: Uuid,
    pub working_dir: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub turn_count: u32,
    /// Mode: "chat" or "agent"
    pub mode: String,
}

/// Chat session manager
pub struct ChatManager {
    backend: Arc<dyn CliBackend>,
    publisher: Arc<ZmqPublisher>,
    sessions: Arc<DashMap<Uuid, ChatSessionInfo>>,
}

impl ChatManager {
    pub fn new(publisher: Arc<ZmqPublisher>) -> Self {
        Self {
            backend: Arc::new(ClaudeBackend::new()),
            publisher,
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Start a new chat session
    pub async fn start_session(
        &self,
        config: ChatSessionConfig,
    ) -> Result<Uuid, String> {
        // Start the backend session
        let handle = self.backend.start_session(config.clone()).await?;
        let session_id = handle.session_id;

        // Store session info
        self.sessions.insert(session_id, ChatSessionInfo {
            session_id,
            working_dir: config.working_dir.clone(),
            created_at: chrono::Utc::now(),
            is_active: true,
            turn_count: 0,
            mode: "chat".to_string(),
        });

        // Spawn task to forward stream chunks to ZMQ publisher
        let publisher = self.publisher.clone();
        let sessions = self.sessions.clone();
        let mut stream_rx = handle.stream_rx;

        tokio::spawn(async move {
            while let Some(chunk) = stream_rx.recv().await {
                // Update turn count on turn complete
                if matches!(chunk, StreamChunk::TurnComplete { .. }) {
                    if let Some(mut info) = sessions.get_mut(&session_id) {
                        info.turn_count += 1;
                    }
                }

                // Check for completion
                let is_complete = matches!(chunk, StreamChunk::Complete { .. });

                // Publish to ZMQ
                if let Err(e) = publisher.publish(session_id, &chunk).await {
                    tracing::error!("Failed to publish chunk: {}", e);
                }

                if is_complete {
                    if let Some(mut info) = sessions.get_mut(&session_id) {
                        info.is_active = false;
                    }
                    break;
                }
            }
        });

        Ok(session_id)
    }

    /// Send a prompt to an existing session
    pub async fn send_prompt(&self, session_id: Uuid, prompt: String) -> Result<(), String> {
        self.backend.send_prompt(session_id, prompt).await
    }

    /// Get session info
    pub fn get_session(&self, session_id: Uuid) -> Option<ChatSessionInfo> {
        self.sessions.get(&session_id).map(|r| r.clone())
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<ChatSessionInfo> {
        self.sessions.iter().map(|r| r.clone()).collect()
    }

    /// Stop a session
    pub async fn stop_session(&self, session_id: Uuid) -> Result<(), String> {
        self.backend.stop_session(session_id).await?;
        if let Some(mut info) = self.sessions.get_mut(&session_id) {
            info.is_active = false;
        }
        Ok(())
    }

    /// Upgrade session to agent mode
    pub fn upgrade_to_agent(&self, session_id: Uuid) -> Result<(), String> {
        if let Some(mut info) = self.sessions.get_mut(&session_id) {
            info.mode = "agent".to_string();
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }
}
```

#### 2. Add RPC methods in `daemon/src/handlers.rs`

```rust
// New RPC methods:

/// Start a chat session
/// Method: "chat.start"
/// Params: { "working_dir": string, "enable_thinking": bool, "thinking_level": string }
/// Returns: { "session_id": string, "pub_endpoint": string }
pub async fn chat_start(&self, params: Value) -> RpcResult {
    let working_dir = params.get("working_dir")
        .and_then(|v| v.as_str())
        .unwrap_or(".")
        .to_string();

    let enable_thinking = params.get("enable_thinking")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let thinking_level = params.get("thinking_level")
        .and_then(|v| v.as_str())
        .unwrap_or("normal")
        .to_string();

    let config = ChatSessionConfig {
        working_dir,
        enable_thinking,
        thinking_level,
        ..Default::default()
    };

    let session_id = self.chat_manager.start_session(config).await
        .map_err(|e| RpcError::internal(e))?;

    Ok(json!({
        "session_id": session_id.to_string(),
        "pub_endpoint": format!("tcp://127.0.0.1:{}", self.config.server.pub_port),
        "topic": format!("chat/{}", session_id),
    }))
}

/// Send prompt to chat session
/// Method: "chat.prompt"
/// Params: { "session_id": string, "prompt": string }
pub async fn chat_prompt(&self, params: Value) -> RpcResult {
    let session_id: Uuid = params.get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("session_id required"))?
        .parse()
        .map_err(|_| RpcError::invalid_params("invalid session_id"))?;

    let prompt = params.get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("prompt required"))?
        .to_string();

    self.chat_manager.send_prompt(session_id, prompt).await
        .map_err(|e| RpcError::internal(e))?;

    Ok(json!({"success": true}))
}

/// Stop chat session
/// Method: "chat.stop"
/// Params: { "session_id": string }
pub async fn chat_stop(&self, params: Value) -> RpcResult {
    let session_id: Uuid = params.get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("session_id required"))?
        .parse()
        .map_err(|_| RpcError::invalid_params("invalid session_id"))?;

    self.chat_manager.stop_session(session_id).await
        .map_err(|e| RpcError::internal(e))?;

    Ok(json!({"success": true}))
}

/// List chat sessions
/// Method: "chat.list"
pub async fn chat_list(&self, _params: Value) -> RpcResult {
    let sessions = self.chat_manager.list_sessions();
    Ok(json!({
        "sessions": sessions.iter().map(|s| json!({
            "session_id": s.session_id.to_string(),
            "working_dir": s.working_dir,
            "created_at": s.created_at.to_rfc3339(),
            "is_active": s.is_active,
            "turn_count": s.turn_count,
            "mode": s.mode,
        })).collect::<Vec<_>>()
    }))
}

/// Upgrade chat to agent mode
/// Method: "chat.upgrade_to_agent"
/// Params: { "session_id": string }
pub async fn chat_upgrade_to_agent(&self, params: Value) -> RpcResult {
    let session_id: Uuid = params.get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("session_id required"))?
        .parse()
        .map_err(|_| RpcError::invalid_params("invalid session_id"))?;

    self.chat_manager.upgrade_to_agent(session_id)
        .map_err(|e| RpcError::internal(e))?;

    Ok(json!({"success": true, "mode": "agent"}))
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-daemon` compiles
- [ ] `cargo test -p descartes-daemon` passes

#### Manual Verification:
- [ ] RPC call `chat.start` returns session_id and pub_endpoint
- [ ] RPC call `chat.prompt` sends prompt successfully
- [ ] Stream chunks appear on ZMQ PUB socket

---

## Phase 4: GUI ZMQ Client Integration

### Overview
Replace direct CLI execution in GUI with daemon RPC calls and ZMQ subscription.

### Changes Required

#### 1. New file: `gui/src/zmq_subscriber.rs`

```rust
//! ZMQ SUB client for receiving stream chunks

use descartes_core::StreamChunk;
use iced::futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;
use uuid::Uuid;
use zeromq::{Socket, SocketRecv, SubSocket};

/// Subscribe to a chat session's stream
pub async fn subscribe_to_session(
    pub_endpoint: &str,
    session_id: Uuid,
) -> Result<impl Stream<Item = StreamChunk>, String> {
    let topic = format!("chat/{}", session_id);

    let mut socket = SubSocket::new();
    socket.connect(pub_endpoint)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    socket.subscribe(&topic)
        .await
        .map_err(|e| format!("Failed to subscribe: {}", e))?;

    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            match socket.recv().await {
                Ok(msg) => {
                    // First frame is topic, second is payload
                    if msg.len() >= 2 {
                        if let Ok(chunk) = serde_json::from_slice::<StreamChunk>(&msg[1]) {
                            if tx.send(chunk).is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("ZMQ recv error: {}", e);
                    break;
                }
            }
        }
    });

    Ok(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
}
```

#### 2. Update `gui/src/chat_state.rs`

Add streaming-related state:
```rust
/// State for the chat interface
#[derive(Debug, Clone, Default)]
pub struct ChatState {
    // ... existing fields ...

    /// Current streaming text (accumulated)
    pub streaming_text: String,
    /// Current thinking text (accumulated)
    pub streaming_thinking: String,
    /// Is currently streaming
    pub is_streaming: bool,
    /// Active session ID (from daemon)
    pub daemon_session_id: Option<Uuid>,
    /// ZMQ PUB endpoint
    pub pub_endpoint: Option<String>,
}

/// Messages for chat operations
#[derive(Debug, Clone)]
pub enum ChatMessage {
    // ... existing variants ...

    /// Session started (from daemon)
    SessionStarted { session_id: Uuid, pub_endpoint: String },
    /// Stream chunk received
    StreamChunk(StreamChunk),
    /// Stream ended
    StreamEnded,
}
```

#### 3. Update `gui/src/main.rs`

Replace direct CLI execution with daemon RPC:
```rust
// In Message::Chat handling for ChatMsg::SubmitPrompt:

ChatMsg::SubmitPrompt => {
    let prompt = self.chat_state.prompt_input.clone();
    if prompt.trim().is_empty() {
        return iced::Task::none();
    }

    let working_dir = self.session_state
        .active_session()
        .map(|s| s.path.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    // If no active daemon session, start one
    if self.chat_state.daemon_session_id.is_none() {
        let client = self.rpc_client.clone();
        return iced::Task::perform(
            async move {
                let response = client.as_ref()
                    .ok_or("Not connected")?
                    .call("chat.start", json!({
                        "working_dir": working_dir,
                        "enable_thinking": true,
                        "thinking_level": "normal",
                    }))
                    .await
                    .map_err(|e| e.to_string())?;

                let session_id: Uuid = response["session_id"]
                    .as_str()
                    .ok_or("Missing session_id")?
                    .parse()
                    .map_err(|_| "Invalid session_id")?;

                let pub_endpoint = response["pub_endpoint"]
                    .as_str()
                    .ok_or("Missing pub_endpoint")?
                    .to_string();

                Ok((session_id, pub_endpoint, prompt))
            },
            |result: Result<(Uuid, String, String), String>| {
                match result {
                    Ok((session_id, pub_endpoint, prompt)) => {
                        Message::Chat(ChatMsg::SessionStartedWithPrompt {
                            session_id,
                            pub_endpoint,
                            initial_prompt: prompt,
                        })
                    }
                    Err(e) => Message::Chat(ChatMsg::Error(e)),
                }
            },
        );
    }

    // Send prompt to existing session
    let session_id = self.chat_state.daemon_session_id.unwrap();
    let client = self.rpc_client.clone();
    iced::Task::perform(
        async move {
            client.as_ref()
                .ok_or("Not connected")?
                .call("chat.prompt", json!({
                    "session_id": session_id.to_string(),
                    "prompt": prompt,
                }))
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        },
        |result: Result<(), String>| {
            match result {
                Ok(()) => Message::Chat(ChatMsg::PromptSent),
                Err(e) => Message::Chat(ChatMsg::Error(e)),
            }
        },
    )
}
```

#### 4. Add ZMQ subscription in `subscription()`

```rust
// In subscription method, add ZMQ stream subscription:

let zmq_sub = if let (Some(endpoint), Some(session_id)) =
    (&self.chat_state.pub_endpoint, self.chat_state.daemon_session_id)
{
    let endpoint = endpoint.clone();
    let session_id = session_id;

    iced::Subscription::run_with_id(
        format!("zmq_chat_{}", session_id),
        iced::stream::channel(100, move |mut output| async move {
            match zmq_subscriber::subscribe_to_session(&endpoint, session_id).await {
                Ok(mut stream) => {
                    use futures::StreamExt;
                    while let Some(chunk) = stream.next().await {
                        let _ = output.send(Message::Chat(ChatMsg::StreamChunk(chunk))).await;
                    }
                    let _ = output.send(Message::Chat(ChatMsg::StreamEnded)).await;
                }
                Err(e) => {
                    let _ = output.send(Message::Chat(ChatMsg::Error(e))).await;
                }
            }

            // Keep subscription alive
            futures::future::pending::<()>().await
        }),
    )
} else {
    iced::Subscription::none()
};
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-gui` compiles
- [ ] No clippy warnings

#### Manual Verification:
- [ ] Submitting prompt starts daemon session
- [ ] Stream chunks appear in real-time
- [ ] Thinking content visible in UI

---

## Phase 5: Thinking Output Display

### Overview
Update chat view to display thinking blocks distinctively.

### Changes Required

#### 1. Update `gui/src/chat_state.rs`

Add thinking accumulation:
```rust
// In update() for StreamChunk:
ChatMessage::StreamChunk(chunk) => {
    match chunk {
        StreamChunk::Text { content } => {
            state.streaming_text.push_str(&content);
        }
        StreamChunk::Thinking { content } => {
            state.streaming_thinking.push_str(&content);
        }
        StreamChunk::TurnComplete { .. } => {
            // Finalize the message
            if !state.streaming_text.is_empty() || !state.streaming_thinking.is_empty() {
                state.messages.push(ChatMessageEntry {
                    id: Uuid::new_v4(),
                    role: ChatRole::Assistant,
                    content: state.streaming_text.clone(),
                    thinking: if state.streaming_thinking.is_empty() {
                        None
                    } else {
                        Some(state.streaming_thinking.clone())
                    },
                    timestamp: chrono::Utc::now(),
                });
                state.streaming_text.clear();
                state.streaming_thinking.clear();
            }
        }
        StreamChunk::Complete { .. } => {
            state.is_streaming = false;
            state.loading = false;
        }
        StreamChunk::Error { message } => {
            state.error = Some(message);
            state.is_streaming = false;
            state.loading = false;
        }
        _ => {}
    }
}
```

#### 2. Update `ChatMessageEntry` struct

```rust
#[derive(Debug, Clone)]
pub struct ChatMessageEntry {
    pub id: Uuid,
    pub role: ChatRole,
    pub content: String,
    pub thinking: Option<String>,  // NEW
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

#### 3. Update `gui/src/chat_view.rs`

Add thinking display in `view_message()`:
```rust
fn view_message(msg: &ChatMessageEntry) -> Element<ChatMessage> {
    // ... existing icon/name/role_color setup ...

    // Thinking block (collapsible)
    let thinking_section = if let Some(ref thinking) = msg.thinking {
        container(
            column![
                row![
                    text("💭").size(12),
                    Space::with_width(6),
                    text("Thinking").size(11).color(colors::TEXT_MUTED),
                ]
                .align_y(Vertical::Center),
                Space::with_height(4),
                text(thinking)
                    .size(12)
                    .font(fonts::MONO)
                    .color(colors::TEXT_SECONDARY),
            ]
        )
        .padding(8)
        .style(container_styles::thinking_block)  // Semi-transparent, italic style
    } else {
        container(Space::with_height(0))
    };

    let content = column![
        // Header row
        row![
            text(icon).size(12).color(role_color),
            Space::with_width(8),
            text(name).size(12).font(fonts::MONO_MEDIUM).color(role_color),
            Space::with_width(Length::Fill),
            text(timestamp).size(10).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center),
        Space::with_height(4),
        thinking_section,  // Show thinking first
        Space::with_height(4),
        content_text,  // Then response
    ];

    // ... rest of styling ...
}
```

#### 4. Add streaming indicator view

```rust
// In view(), show streaming content in real-time:

let streaming_section = if state.is_streaming {
    let mut content = column![];

    if !state.streaming_thinking.is_empty() {
        content = content.push(
            container(
                column![
                    row![
                        text("💭").size(12),
                        text(" Thinking...").size(11).color(colors::TEXT_MUTED),
                    ],
                    text(&state.streaming_thinking)
                        .size(12)
                        .font(fonts::MONO)
                        .color(colors::TEXT_SECONDARY),
                ]
            )
            .padding(8)
            .style(container_styles::thinking_block)
        );
    }

    if !state.streaming_text.is_empty() {
        content = content.push(
            text(&state.streaming_text)
                .size(14)
                .font(fonts::MONO)
                .color(colors::TEXT_PRIMARY)
        );
    }

    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(container_styles::panel)
} else {
    container(Space::with_height(0))
};
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-gui` compiles

#### Manual Verification:
- [ ] Thinking blocks display with distinct styling
- [ ] Streaming text updates in real-time
- [ ] Complete messages show thinking collapsed or expandable

---

## Phase 6: Agent Upgrade Path

### Overview
Enable upgrading chat sessions to agent mode with sub-agent support.

### Changes Required

#### 1. Add upgrade button in chat UI

```rust
// In chat_view.rs, add to controls_row:

let upgrade_btn = if state.daemon_session_id.is_some() && state.mode == "chat" {
    button(
        text("⚡ Upgrade to Agent")
            .size(11)
            .color(colors::PRIMARY)
    )
    .on_press(ChatMessage::UpgradeToAgent)
    .padding([4, 8])
    .style(button_styles::secondary)
} else if state.mode == "agent" {
    container(
        row![
            text("⚡").size(10).color(colors::SUCCESS),
            Space::with_width(4),
            text("Agent Mode").size(11).color(colors::SUCCESS),
        ]
    )
    .padding([4, 8])
} else {
    container(Space::with_width(0))
};
```

#### 2. Handle upgrade message

```rust
// In main.rs Message::Chat handling:

ChatMsg::UpgradeToAgent => {
    if let Some(session_id) = self.chat_state.daemon_session_id {
        let client = self.rpc_client.clone();
        return iced::Task::perform(
            async move {
                client.as_ref()
                    .ok_or("Not connected")?
                    .call("chat.upgrade_to_agent", json!({
                        "session_id": session_id.to_string(),
                    }))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            },
            |result| match result {
                Ok(()) => Message::Chat(ChatMsg::UpgradedToAgent),
                Err(e) => Message::Chat(ChatMsg::Error(e)),
            },
        );
    }
    iced::Task::none()
}

ChatMsg::UpgradedToAgent => {
    self.chat_state.mode = "agent".to_string();
    self.status_message = Some("Session upgraded to Agent mode".to_string());
    iced::Task::none()
}
```

#### 3. Add mode to ChatState

```rust
pub struct ChatState {
    // ... existing fields ...

    /// Current mode: "chat" or "agent"
    pub mode: String,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            mode: "chat".to_string(),
        }
    }
}
```

### Success Criteria

#### Automated Verification:
- [ ] `cargo build -p descartes-gui` compiles

#### Manual Verification:
- [ ] Upgrade button visible in chat mode
- [ ] Clicking upgrade calls daemon RPC
- [ ] UI reflects agent mode status

---

## Future Considerations (Not In Scope)

### SQLite Session Persistence

When implementing persistence:
- Store in `~/.descartes/sessions.db`
- Schema: sessions, messages, thinking_blocks tables
- Load sessions on daemon start
- Save incrementally as messages arrive

### OpenCode Backend

When adding OpenCode:
- Implement `CliBackend` trait for OpenCode
- Add CLI detection in `ClaudeBackend::is_available()`
- Add backend selection in chat config
- Parse OpenCode's stream format (may differ from Claude)

### Sub-Agent Orchestration

When implementing sub-agents:
- Use existing `ZmqAgentRunner` for spawning
- Parent session tracks child session IDs
- Stream child output through parent's ZMQ topic
- Add UI for sub-agent tree view

---

## Testing Strategy

### Unit Tests
- `CliBackend` trait mock for testing without CLI
- Stream chunk parsing edge cases
- ZMQ message serialization

### Integration Tests
- Daemon chat.* RPC methods
- ZMQ PUB/SUB round-trip
- Multi-session management

### Manual Testing Steps
1. Start daemon with `--pub-port 19480`
2. Start GUI, connect to daemon
3. Submit prompt in chat
4. Verify thinking output appears
5. Verify streaming updates in real-time
6. Test upgrade to agent mode
7. Stop session and verify cleanup

---

## References

- ZMQ infrastructure: `core/src/zmq_*.rs`
- Daemon attach system: `daemon/src/claude_code_tui.rs`
- GUI chat: `gui/src/chat_state.rs`, `gui/src/chat_view.rs`
- Claude CLI docs: https://code.claude.com/docs/en/cli-reference
