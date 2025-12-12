---
date: 2025-12-11T20:21:58-0600
researcher: Claude
git_commit: 6c8e6674c283f014881c3e148269e9c6c4cbe7b8
branch: backbone
repository: descartes
topic: "Chat and Agent Feature Analysis for Testing Plan"
tags: [research, codebase, chat, agent, testing, integration]
status: complete
last_updated: 2025-12-11
last_updated_by: Claude
---

# Research: Chat and Agent Feature Analysis for Testing Plan

**Date**: 2025-12-11T20:21:58-0600
**Researcher**: Claude
**Git Commit**: 6c8e6674c283f014881c3e148269e9c6c4cbe7b8
**Branch**: backbone
**Repository**: descartes

## Research Question

Analyze the chat and agent features in the descartes codebase to understand:
1. How these features are implemented
2. What tests currently exist
3. What integration points need testing
4. How to develop a testing plan to verify everything works

## Summary

Descartes is a composable AI agent orchestration system with four main packages:
- **core**: Core library with backends, session management, state machines
- **daemon**: HTTP/WebSocket/ZMQ server for RPC and streaming
- **gui**: Iced-based GUI with chat interface and monitoring
- **cli**: Command-line interface for agent management

The chat feature uses a Claude CLI backend that spawns `claude -p --output-format stream-json` processes and streams NDJSON output via ZMQ PUB/SUB to clients. The agent feature provides process spawning, lifecycle management, and monitoring via RPC.

## Detailed Findings

### 1. Chat Feature Implementation

#### Core Chat Components

**`descartes/core/src/cli_backend.rs`** (lines 1-147)
- Defines `CliBackend` trait (lines 78-106) - interface for CLI backends
- Defines `StreamChunk` enum (lines 12-38) - message types for streaming:
  - `Text` - assistant response content
  - `Thinking` - reasoning/thinking blocks
  - `ToolUseStart`, `ToolUseInput`, `ToolResult` - tool execution events
  - `TurnComplete`, `Complete`, `Error` - lifecycle events
- `ChatSessionConfig` struct (lines 40-69) - session configuration with:
  - `working_dir`, `initial_prompt`, `enable_thinking`, `thinking_level`, `max_turns`
- `ChatSessionHandle` (lines 71-76) - returns `session_id` and `stream_rx` channel

**`descartes/core/src/claude_backend.rs`** (lines 1-401)
- `ClaudeBackend` implements `CliBackend` (lines 180-369)
- `build_command()` (lines 120-162):
  - Uses `claude -p --output-format stream-json`
  - Adds thinking prefixes based on level ("think hard:", "ultrathink:", etc.)
  - Sets working directory and max turns
- `start_session()` (lines 202-332):
  - Spawns Claude CLI process
  - Parses NDJSON stream from stdout
  - Handles message types: `assistant`, `result`, `content_block_delta`, `message_stop`
  - Emits `StreamChunk` messages to channel
- Tests (lines 371-400): `test_thinking_prefix`, `test_claude_backend_availability`, `test_stream_message_parsing`

#### Daemon Chat Handling

**`descartes/daemon/src/chat_manager.rs`** (lines 1-322)
- `ChatManager` struct (lines 40-46):
  - Holds `CliBackend` (ClaudeBackend)
  - Holds `ZmqPublisher` for streaming
  - Uses `DashMap<Uuid, SessionTracker>` for session tracking
- `start_session()` (lines 62-120):
  - Starts CLI immediately with initial prompt
  - Spawns task to forward `StreamChunk` to ZMQ publisher
- `create_session()` (lines 122-145):
  - Creates session WITHOUT starting CLI (for race-free subscription)
  - Stores config for deferred CLI start
- `send_prompt()` (lines 147-226):
  - If CLI not started, starts it with the prompt
  - Otherwise attempts to send to running session (limited in single-shot mode)
- `upgrade_to_agent()` (lines 274-281): Changes session mode flag

**`descartes/daemon/src/rpc.rs`** (lines 1-603)
- Chat RPC methods (lines 258-465):
  - `chat.create` (lines 260-310): Creates session, returns `{session_id, pub_endpoint, topic}`
  - `chat.start` (lines 312-364): Legacy - starts CLI immediately
  - `chat.prompt` (lines 366-395): Sends prompt to session
  - `chat.stop` (lines 397-421): Stops session
  - `chat.list` (lines 423-439): Lists all sessions
  - `chat.upgrade_to_agent` (lines 441-465): Upgrades to agent mode
- Tests (lines 480-602): RPC protocol tests

#### GUI Chat Components

**`descartes/gui/src/chat_state.rs`** (lines 1-266)
- `ChatState` struct (lines 11-41):
  - Message history, streaming state, session tracking
  - `daemon_session_id`, `pub_endpoint` for ZMQ subscription
  - `pending_prompt` for race-free flow
- `ChatMessage` enum (lines 63-102): UI update messages
  - `SessionCreated`, `SessionStarted`, `StreamChunk`, `SendPendingPrompt`
- `update()` function (lines 115-265): State machine for UI updates
  - Handles streaming chunks, turn completion, errors

**`descartes/gui/src/zmq_subscriber.rs`** (lines 1-109)
- `subscribe_to_session()`: Connects to ZMQ PUB socket
- Topic format: `"chat/{session_id}"`
- Parses `StreamChunk` from JSON messages

#### Chat Data Flow

```
GUI                     Daemon                    Claude CLI
 │                        │                          │
 ├─── chat.create ───────>│                          │
 │<── {session_id, pub_endpoint, topic} ─┤          │
 │                        │                          │
 ├─── ZMQ SUB "chat/{id}" │                          │
 │                        │                          │
 ├─── chat.prompt ───────>│                          │
 │                        ├── spawn claude -p ──────>│
 │                        │<── NDJSON stream ────────┤
 │<── ZMQ: StreamChunk ───┤                          │
 │<── ZMQ: StreamChunk ───┤                          │
 │<── ZMQ: Complete ──────┤                          │
```

### 2. Agent Feature Implementation

#### Core Agent Components

**`descartes/core/src/agent_runner.rs`** (lines 1-500+)
- `LocalProcessRunner` struct (lines 41-46):
  - Registry: `DashMap<Uuid, Arc<RwLock<LocalAgentHandle>>>`
  - Configuration: `ProcessRunnerConfig`
- `ProcessRunnerConfig` (lines 49-73):
  - `working_dir`, `enable_json_streaming`, `enable_health_checks`
  - `health_check_interval_secs`, `max_concurrent_agents`
- `build_command()` (lines 115-176):
  - Supports `claude-code-cli`/`claude`, `opencode`, generic CLI backends
  - Sets stdio pipes, environment variables, working directory
- Implements `AgentRunner` trait for spawning, listing, killing agents
- Health checker spawning (lines 178-200+)

**`descartes/core/src/agent_state.rs`** (lines 1-500+)
- `AgentRuntimeState`: Current agent state
- `AgentStateCollection`: Collection of agent states
- `AgentStreamMessage`: Parsed stream messages
- `AgentStatus`: Running, Paused, Completed, Failed, etc.

**`descartes/core/src/agent_stream_parser.rs`** (lines 1-800+)
- `AgentStreamParser`: Parses NDJSON from agent processes
- `StreamHandler` trait: Callback interface for parsed events
- `ParserConfig`: Configuration for parsing behavior

**`descartes/daemon/src/agent_monitor.rs`** (lines 1-500+)
- `AgentMonitor` struct (lines 122-137):
  - Integrates stream parser with RPC/event bus
  - Tracks agents via `HashMap<Uuid, AgentRuntimeState>`
- `AgentMonitorConfig` (lines 79-99):
  - `auto_discover`, `max_agents`, `stale_threshold_secs`
- Publishes updates to event bus for GUI consumption

#### Agent RPC Methods

**`descartes/daemon/src/rpc.rs`** (lines 70-73, 174-210)
- `agent.spawn`: Spawns new agent with configuration
- `agent.list`: Lists all active agents
- `agent.kill`: Terminates an agent
- `agent.logs`: Retrieves agent logs

### 3. Existing Test Coverage

#### Test Statistics
- **Total test files**: 30 dedicated test files in `/tests/` directories
- **Total test functions**: ~1,210 across all files
- **Test types**: Mix of `#[test]` and `#[tokio::test]`

#### CLI Tests (50 tests)
| File | Tests | Coverage |
|------|-------|----------|
| `cli/tests/spawn_tests.rs` | 27 | Provider config, backend creation |
| `cli/tests/init_tests.rs` | 5 | Project initialization |
| `cli/tests/kill_tests.rs` | 5 | Agent termination |
| `cli/tests/logs_tests.rs` | 8 | Log querying |
| `cli/tests/ps_tests.rs` | 4 | Agent listing |
| `cli/tests/plugins_tests.rs` | 1 | Plugin management |

#### Core Tests (264 tests)
| File | Tests | Coverage |
|------|-------|----------|
| `core/tests/debugger_tests.rs` | 70 | Debugger functionality |
| `core/tests/dag_tests.rs` | 70 | DAG operations |
| `core/tests/dag_swarm_conversion_tests.rs` | 26 | DAG/Swarm conversion |
| `core/tests/swarm_parser_tests.rs` | 27 | Swarm config parsing |
| `core/tests/agent_runner_tests.rs` | 19 | Agent execution |
| `core/tests/time_travel_integration_tests.rs` | 23 | State replay |
| `core/tests/zmq_integration_tests.rs` | 23 | ZMQ communication |

#### Daemon Tests (93 tests)
| File | Tests | Coverage |
|------|-------|----------|
| `daemon/tests/rpc_server_tests.rs` | 40 | RPC server |
| `daemon/tests/agent_monitor_integration_tests.rs` | 15 | Agent monitoring |
| `daemon/tests/task_board_realtime_tests.rs` | 14 | Task board |
| `daemon/tests/client_integration_test.rs` | 11 | Client communication |

#### GUI Tests (251 tests)
| File | Tests | Coverage |
|------|-------|----------|
| `gui/tests/debugger_ui_tests.rs` | 51 | Debugger UI |
| `gui/tests/dag_editor_tests.rs` | 50 | DAG editor |
| `gui/tests/time_travel_tests.rs` | 42 | Time travel UI |
| `gui/tests/swarm_monitor_tests.rs` | 35 | Swarm monitoring |

#### Inline Tests
Key source files with inline `#[cfg(test)]` modules:
- `core/src/providers_test.rs` - 32 tests
- `core/src/debugger.rs` - 45 tests
- `core/src/dag.rs` - 26 tests
- `core/src/cli_backend.rs` - 4 tests
- `core/src/claude_backend.rs` - 4 tests
- `daemon/src/rpc.rs` - 6 tests
- `daemon/src/chat_manager.rs` - 1 test

### 4. Testing Gaps Identified

#### Chat Feature - Limited Test Coverage
1. **No dedicated chat integration tests**: `chat_manager.rs` has only 1 unit test
2. **No ZMQ streaming tests for chat**: Need to test full pub/sub flow
3. **No GUI chat state tests**: `chat_state.rs` has no tests
4. **No end-to-end chat tests**: GUI -> daemon -> Claude CLI -> ZMQ -> GUI

#### Agent Feature - Good Coverage But Gaps
1. **Agent runner tests limited**: Only 19 tests, mostly edge cases
2. **No daemon agent RPC integration tests**: Need to test spawn/kill/list via RPC
3. **Agent monitor tests exist but may need expansion**

### 5. Integration Points Requiring Testing

#### HTTP/RPC Layer (Port 19280)
- JSON-RPC 2.0 request/response
- Batch request handling
- Authentication (JWT, API key)
- Error responses

#### ZMQ PUB/SUB Layer (Port 19480)
- Publisher initialization
- Topic-based subscription
- StreamChunk serialization/deserialization
- Multi-client scenarios
- Connection handling

#### Claude CLI Integration
- Process spawning with correct arguments
- NDJSON stream parsing
- Error handling (CLI not found, API errors)
- Process termination

#### SQLite State Store
- Agent state persistence
- Session tracking
- Migration handling

## Architecture Documentation

```
┌─────────────┐     ┌─────────────┐
│     CLI     │     │     GUI     │
└──────┬──────┘     └──────┬──────┘
       │                   │
       │ Unix Socket       │ HTTP + ZMQ SUB
       │                   │
       └────────┬──────────┘
                │
        ┌───────▼───────┐
        │    Daemon     │
        │  HTTP :19280  │
        │  WS   :19380  │
        │  ZMQ  :19480  │
        └───────┬───────┘
                │
    ┌───────────┼───────────┐
    │           │           │
┌───▼───┐  ┌───▼───┐  ┌────▼────┐
│Claude │  │SQLite │  │ZMQ PUB  │
│ CLI   │  │Store  │  │Publisher│
└───────┘  └───────┘  └─────────┘
```

## Recommended Testing Plan

### Phase 1: Unit Tests (Add Missing)
1. Add tests for `ChatState` update logic in `gui/src/chat_state.rs`
2. Add tests for `ChatManager` session lifecycle in `daemon/src/chat_manager.rs`
3. Expand `claude_backend.rs` tests for error scenarios

### Phase 2: Integration Tests
1. **Chat RPC Integration**: Test `chat.create` -> `chat.prompt` -> `chat.stop` flow
2. **ZMQ Streaming**: Test publisher/subscriber with mock data
3. **Agent RPC Integration**: Test `agent.spawn` -> `agent.list` -> `agent.kill` flow

### Phase 3: End-to-End Tests
1. **Chat E2E**: Start daemon, create session, subscribe ZMQ, send prompt, verify chunks
2. **Agent E2E**: Start daemon, spawn agent, monitor output, terminate
3. **GUI Integration**: Test chat view with daemon connection

### Test Commands for Manual Verification
```bash
# Build
cargo build -p descartes-daemon -p descartes-gui

# Start daemon
RUST_LOG=info ./target/debug/descartes-daemon

# Test chat.create
curl -s -X POST http://127.0.0.1:19280/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"chat.create","params":{"working_dir":"/tmp","enable_thinking":false}}'

# Test system.health
curl -s -X POST http://127.0.0.1:19280/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"system.health"}'

# Run existing tests
cargo test -p descartes-core -p descartes-daemon --lib
```

## Code References

- `descartes/core/src/cli_backend.rs:12-106` - CliBackend trait and StreamChunk
- `descartes/core/src/claude_backend.rs:109-369` - ClaudeBackend implementation
- `descartes/daemon/src/chat_manager.rs:40-297` - ChatManager
- `descartes/daemon/src/rpc.rs:258-465` - Chat RPC methods
- `descartes/gui/src/chat_state.rs:11-265` - GUI chat state
- `descartes/core/src/agent_runner.rs:41-200` - LocalProcessRunner
- `descartes/daemon/src/agent_monitor.rs:122-137` - AgentMonitor

## Open Questions

1. **Multi-turn chat**: The current `ClaudeBackend` uses single-shot mode (`-p`). How should multi-turn conversations be handled?
2. **Session persistence**: Are chat sessions persisted to SQLite, or only kept in memory?
3. **ZMQ reconnection**: What happens if ZMQ subscriber disconnects mid-stream?
4. **Agent coordination**: How do multiple agents communicate in swarm mode?
