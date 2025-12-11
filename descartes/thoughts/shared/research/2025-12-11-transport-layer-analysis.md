---
date: 2025-12-11T05:49:16Z
researcher: reuben
git_commit: 5fc83d472ca4774c018a21e3ba5c7e3d55db31d7
branch: backbone
repository: descartes
topic: "Transport Layer Analysis - Daemon Communication Mechanisms"
tags: [research, transport, zmq, rpc, http, websocket, ipc, daemon]
status: complete
last_updated: 2025-12-11
last_updated_by: reuben
---

# Research: Transport Layer Analysis

**Date**: 2025-12-11T05:49:16Z
**Researcher**: reuben
**Git Commit**: 5fc83d472ca4774c018a21e3ba5c7e3d55db31d7
**Branch**: backbone
**Repository**: descartes

## Research Question

Evaluate all the transport layer(s) in the Descartes codebase. The goal context is that the daemon should communicate to everything via ZMQ (agents/subagents, clients, etc).

## Summary

The Descartes daemon currently uses **four distinct transport mechanisms** for different communication needs:

| Transport | Port/Path | Used For | Direction |
|-----------|-----------|----------|-----------|
| **HTTP JSON-RPC** | 8080 | RPC requests from GUI | Request/Response |
| **Unix Socket JSON-RPC** | `/tmp/descartes-rpc.sock` | RPC from CLI, local IPC | Request/Response |
| **ZMQ PUB/SUB** | 19480 | Chat streaming to GUI | One-way publish |
| **WebSocket** | 8081 (reserved) | Event streaming (partially implemented) | Bidirectional |

Additionally, **agent communication** uses:
- **Local stdio** - Spawned CLI agents communicate via stdin/stdout pipes
- **ZMQ REQ/REP** - Remote agent servers use ZMQ for distributed agent management
- **ZMQ DEALER/ROUTER** - Async patterns for agent coordination

---

## Detailed Findings

### 1. Current Transport Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        DAEMON SERVER                            │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ HTTP Server  │  │ Unix Socket  │  │ ZMQ PUB      │          │
│  │ Port 8080    │  │ /tmp/*.sock  │  │ Port 19480   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                 │                   │
│         └────────┬────────┘                 │                   │
│                  ▼                          │                   │
│         ┌────────────────┐                  │                   │
│         │ JsonRpcServer  │                  │                   │
│         │ (rpc.rs)       │                  │                   │
│         └───────┬────────┘                  │                   │
│                 │                           │                   │
│         ┌───────▼────────┐         ┌───────▼────────┐          │
│         │ RpcHandlers    │         │ ZmqPublisher   │          │
│         │ (handlers.rs)  │         │ (zmq_pub.rs)   │          │
│         └───────┬────────┘         └───────┬────────┘          │
│                 │                          │                   │
│         ┌───────▼────────┐         ┌───────▼────────┐          │
│         │ ChatManager    │◄────────│ Stream Chunks  │          │
│         │ (chat_mgr.rs)  │         │ to SUB clients │          │
│         └────────────────┘         └────────────────┘          │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   GUI Client    │     │   CLI Client    │     │ Remote Agents   │
│                 │     │                 │     │                 │
│ HTTP RPC ───────┼────►│ Unix Socket ────┼────►│ ZMQ REQ/REP ────┼──►
│ ZMQ SUB ◄───────┼─────│                 │     │ (Port 5555)     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### 2. HTTP JSON-RPC Transport

**Location**: `daemon/src/server.rs:78-320`

**Implementation Details**:
- Uses `hyper` HTTP server
- Default endpoint: `http://127.0.0.1:8080`
- JSON-RPC 2.0 protocol
- Bearer token authentication support

**Configuration** (`daemon/src/config.rs:42-44`):
```rust
pub http_addr: String,  // Default: "127.0.0.1"
pub http_port: u16,     // Default: 8080
```

**Request Flow**:
1. HTTP POST received at `server.rs:209`
2. Body parsed as JSON-RPC request at `server.rs:229`
3. Routed to `JsonRpcServer::process_request()` at `rpc.rs:46`
4. Method dispatched via pattern matching at `rpc.rs:69-88`
5. Handler executes and returns response

**Available RPC Methods**:
- `agent.spawn`, `agent.list`, `agent.kill`, `agent.logs`
- `workflow.execute`
- `state.query`
- `system.health`, `system.metrics`
- `chat.start`, `chat.prompt`, `chat.stop`, `chat.list`, `chat.upgrade_to_agent`

**Client**: `daemon/src/client.rs:81` - `RpcClient` using `reqwest`

### 3. Unix Socket JSON-RPC Transport

**Location**: `daemon/src/rpc_server.rs:862-987`

**Implementation Details**:
- Uses `tokio::net::UnixListener`
- Default socket: `/tmp/descartes-rpc.sock`
- Line-delimited JSON framing
- New connection per request (no pooling)

**Request Flow**:
1. Unix stream accepted at `rpc_server.rs:930`
2. Line read from stream at `rpc_server.rs:995-997`
3. Parsed as JSON-RPC at `rpc_server.rs:1004`
4. Routed via `process_single_request()` at `rpc_server.rs:1085`

**Client**: `daemon/src/rpc_client.rs:84` - `UnixSocketRpcClient`

**Used By**:
- CLI commands (`cli/src/rpc.rs:63`)
- GUI alternative transport (`gui/src/rpc_unix_client.rs:12`)

### 4. ZMQ PUB/SUB Transport (Chat Streaming)

**Location**: `daemon/src/zmq_publisher.rs:18-75`

**Implementation Details**:
- Uses `zeromq::PubSocket`
- Default endpoint: `tcp://0.0.0.0:19480`
- Topic-based routing: `chat/{session_id}`
- One-way streaming (daemon → clients)

**Configuration** (`daemon/src/config.rs:47-48`):
```rust
pub pub_addr: String,  // Default: "0.0.0.0"
pub pub_port: u16,     // Default: 19480
```

**Message Flow**:
1. `ChatManager` receives stream chunks from CLI backend
2. Chunks forwarded to `ZmqPublisher::publish()` at `chat_manager.rs:95`
3. Message formatted as `"{topic} {json}"` at `zmq_publisher.rs:48`
4. Sent via PubSocket at `zmq_publisher.rs:50-58`

**Client**: `gui/src/zmq_subscriber.rs:24` - Uses `zeromq::SubSocket`

**StreamChunk Types** (`core/src/cli_backend.rs`):
- `Text { content }` - Assistant response text
- `Thinking { content }` - Reasoning/thinking output
- `ToolUseStart { tool_name, tool_id }` - Tool invocation start
- `ToolUseInput { content }` - Tool input chunk
- `ToolResult { tool_id, result }` - Tool execution result
- `TurnComplete { turn_number }` - Turn boundary marker
- `Complete { session_id }` - Session end marker
- `Error { message }` - Error notification

### 5. WebSocket Transport (Partially Implemented)

**Location**: `daemon/src/event_stream.rs`

**Implementation Details**:
- Uses `tokio-tungstenite`
- Default port: 8081 (configured but not fully wired)
- Intended for real-time event streaming

**Configuration** (`daemon/src/config.rs:45-46`):
```rust
pub ws_addr: String,   // Default: "127.0.0.1"
pub ws_port: u16,      // Default: 8081
```

**Current Status**:
- Event types defined (`ServerMessage`, `ClientMessage`)
- Event bus infrastructure exists (`daemon/src/events.rs`)
- Client handler exists (`daemon/src/event_client.rs`)
- **Not started by default daemon binary** - ZMQ PUB/SUB used instead

### 6. ZMQ Agent Communication (Remote Agents)

**Location**: `core/src/zmq_server.rs`, `core/src/zmq_client.rs`

**Implementation Details**:
- Server uses REP socket (request/reply pattern)
- Client uses REQ socket
- Default endpoint: `tcp://0.0.0.0:5555`
- MessagePack serialization

**Message Protocol** (`core/src/zmq_agent_runner.rs:536-580`):
```rust
pub enum ZmqMessage {
    SpawnRequest { request_id, config },
    SpawnResponse { request_id, success, agent_info, error },
    ControlCommand { request_id, agent_id, command_type },
    CommandResponse { request_id, success, result, error },
    StatusUpdate { agent_id, status, timestamp },
    ListAgentsRequest { request_id },
    ListAgentsResponse { request_id, agents },
    HealthCheckRequest { request_id },
    HealthCheckResponse { request_id, healthy, details },
    // ... batch operations
}
```

**Agent Lifecycle via ZMQ**:
1. Client calls `ZmqClient::spawn_remote(config)` at `zmq_client.rs:609`
2. Server receives `SpawnRequest` at `zmq_server.rs:352`
3. Server spawns local process via `LocalProcessRunner` at `zmq_server.rs:403`
4. Server returns `SpawnResponse` with agent info
5. Control commands (stop, pause, resume) sent same way

### 7. Local Agent Communication (Stdio)

**Location**: `core/src/agent_runner.rs:278-347`

**Implementation Details**:
- Spawns CLI as child process with piped stdio
- stdin for sending commands/prompts
- stdout/stderr for receiving output
- Background tasks read output asynchronously

**Agent Backends Supported**:
- `claude-code-cli` / `claude` → spawns `claude` command
- `opencode` → spawns `opencode --headless` command

**I/O Streaming** (`agent_runner.rs:673-767`):
- Unbounded mpsc channels for buffering
- Broadcast channels for TUI attachment (1024 msg buffer)
- Line-by-line reading with BufReader

**Signal-Based Control**:
- `SIGINT` - Interrupt
- `SIGTERM` - Terminate
- `SIGKILL` - Force kill
- `SIGSTOP` - Force pause
- `SIGCONT` - Resume
- Cooperative pause via JSON stdin message

---

## Code References

### Core ZMQ Implementation
- `core/src/zmq_communication.rs` - Socket wrappers, connection management
- `core/src/zmq_server.rs` - Agent server implementation
- `core/src/zmq_client.rs` - Agent client implementation
- `core/src/zmq_agent_runner.rs` - Protocol definitions, message types
- `core/src/channel_bridge.rs` - In-process coordination bridge

### Daemon Transport Layer
- `daemon/src/server.rs:78-320` - HTTP server, request routing
- `daemon/src/rpc.rs:46-117` - JSON-RPC processing, method dispatch
- `daemon/src/rpc_server.rs:862-987` - Unix socket server
- `daemon/src/zmq_publisher.rs:18-75` - ZMQ PUB socket
- `daemon/src/event_stream.rs` - WebSocket server (partial)

### Client Implementations
- `daemon/src/client.rs:81` - HTTP RPC client
- `daemon/src/rpc_client.rs:84` - Unix socket RPC client
- `gui/src/rpc_client.rs:13` - GUI HTTP client wrapper
- `gui/src/rpc_unix_client.rs:12` - GUI Unix socket wrapper
- `gui/src/zmq_subscriber.rs:24` - ZMQ SUB client
- `cli/src/rpc.rs:63` - CLI connection helper

### Agent Runner
- `core/src/agent_runner.rs:278` - LocalProcessRunner::spawn()
- `core/src/agent_runner.rs:398-520` - Signal handling
- `core/src/agent_runner.rs:673-767` - Stdio streaming

---

## Architecture Documentation

### Transport Selection Matrix

| Component | To Daemon | From Daemon | Why |
|-----------|-----------|-------------|-----|
| GUI RPC | HTTP (8080) | HTTP Response | Cross-network compatible |
| GUI Streaming | - | ZMQ SUB (19480) | High-performance pub/sub |
| CLI Commands | Unix Socket | Unix Socket | Low latency local IPC |
| Remote Agents | ZMQ REQ (5555) | ZMQ REP | Distributed, async |
| Local Agents | stdin | stdout/stderr | Process pipes |

### Port Assignments

| Port | Protocol | Purpose | Configurable |
|------|----------|---------|--------------|
| 8080 | HTTP | JSON-RPC API | `--http-port` |
| 8081 | WebSocket | Events (reserved) | `--ws-port` |
| 9090 | HTTP | Prometheus metrics | `metrics_port` |
| 19480 | ZMQ PUB | Chat streaming | `--pub-port` |
| 5555 | ZMQ REQ/REP | Agent management | In config |

### Authentication

- HTTP: Bearer token in `Authorization` header
- Unix Socket: No auth (local trust)
- ZMQ PUB/SUB: No auth (topic-based isolation)
- ZMQ Agent: No auth (network isolation assumed)

### Error Handling

All transports use `DaemonError` enum:
```rust
pub enum DaemonError {
    ConnectionError(String),
    RpcError { code: i32, message: String },
    SerializationError(String),
    TimeoutError,
    // ...
}
```

---

## Transport Inventory Summary

### Currently Active Transports

1. **HTTP JSON-RPC** (`hyper`)
   - Files: `server.rs`, `client.rs`, `rpc_client.rs`
   - Purpose: Primary GUI RPC, external API

2. **Unix Socket JSON-RPC** (`tokio::net::UnixStream`)
   - Files: `rpc_server.rs`, `rpc_client.rs`
   - Purpose: CLI commands, local IPC

3. **ZMQ PUB/SUB** (`zeromq::PubSocket/SubSocket`)
   - Files: `zmq_publisher.rs`, `zmq_subscriber.rs`
   - Purpose: Chat streaming to GUI

4. **ZMQ REQ/REP** (`zeromq::ReqSocket/RepSocket`)
   - Files: `zmq_server.rs`, `zmq_client.rs`, `zmq_communication.rs`
   - Purpose: Remote agent management

5. **Process Stdio** (`tokio::process::Command`)
   - Files: `agent_runner.rs`
   - Purpose: Local agent I/O

### Partially Implemented

6. **WebSocket** (`tokio-tungstenite`)
   - Files: `event_stream.rs`, `event_client.rs`
   - Purpose: Event streaming (not started by daemon)

### Dependencies

```toml
# core/Cargo.toml
zeromq = "0.4"

# daemon/Cargo.toml
zeromq = "0.4"
hyper = "1.0"
tokio-tungstenite = "0.21"
jsonrpsee = { features = ["ws-client"] }

# gui/Cargo.toml
zeromq = "0.4"
reqwest = "0.11"
```

---

## Related Research

- `thoughts/shared/plans/2025-12-11-zmq-chat-streaming.md` - ZMQ streaming implementation plan

## Open Questions

1. **WebSocket vs ZMQ**: The WebSocket transport (`event_stream.rs`) is implemented but not started. Should it be removed in favor of ZMQ PUB/SUB, or kept for browser clients?

2. **HTTP vs ZMQ for RPC**: Currently GUI uses HTTP for RPC. Could this be consolidated to ZMQ REQ/REP for consistency?

3. **Unix Socket Scope**: Unix socket is used for CLI. Should remote CLI scenarios use ZMQ instead?

4. **Agent Communication**: Local agents use stdio pipes. Should they also communicate via ZMQ for consistency with remote agents?

5. **Authentication**: ZMQ transports have no authentication. How should this be addressed for security?
