# Subagent Pause & Attach - Task Breakdown

## Summary

This document breaks down the implementation plan into discrete, actionable tasks organized by phase.

---

## Phase 1: Core Pause/Resume Mechanism

### Task 1.1: Extend AgentSignal Enum
**File**: `descartes/core/src/traits.rs`
**Effort**: Small
**Description**: Add `ForcePause` and `Resume` variants to `AgentSignal` enum for SIGSTOP/SIGCONT support.

### Task 1.2: Add PauseMode and AttachInfo Types
**File**: `descartes/core/src/traits.rs`
**Effort**: Small
**Description**: Define `PauseMode` enum (Cooperative/Forced) and `AttachInfo` struct for connection metadata.

### Task 1.3: Extend AgentInfo with Pause Fields
**File**: `descartes/core/src/traits.rs`
**Effort**: Small
**Description**: Add `paused_at`, `pause_mode`, and `attach_info` fields to `AgentInfo`.

### Task 1.4: Add pause/resume to AgentRunner Trait
**File**: `descartes/core/src/traits.rs`
**Effort**: Small
**Description**: Add `pause()` and `resume()` method signatures to the `AgentRunner` trait.

### Task 1.5: Implement Cooperative Pause in LocalProcessRunner
**File**: `descartes/core/src/agent_runner.rs`
**Effort**: Medium
**Description**: Implement `pause()` method that sends pause notification via stdin and updates status.

### Task 1.6: Implement Forced Pause (SIGSTOP) in LocalProcessRunner
**File**: `descartes/core/src/agent_runner.rs`
**Effort**: Medium
**Description**: Extend `pause()` to support `force=true` using SIGSTOP on Unix, with Windows fallback.

### Task 1.7: Implement Resume in LocalProcessRunner
**File**: `descartes/core/src/agent_runner.rs`
**Effort**: Medium
**Description**: Implement `resume()` method with SIGCONT for forced pauses and stdin notification.

### Task 1.8: Add Pause/Resume Unit Tests
**File**: `descartes/core/tests/pause_tests.rs` (new)
**Effort**: Medium
**Description**: Write unit tests for cooperative pause, forced pause, resume, and state transitions.

### Task 1.9: Update signal() Method for New Signals
**File**: `descartes/core/src/agent_runner.rs`
**Effort**: Small
**Description**: Update `signal()` to handle `ForcePause` and `Resume` variants.

---

## Phase 2: Attach Infrastructure

### Task 2.1: Create AttachToken Struct and Store
**File**: `descartes/core/src/attach.rs` (new)
**Effort**: Medium
**Description**: Implement `AttachToken` struct and `AttachTokenStore` with generation, validation, revocation.

### Task 2.2: Implement Token TTL and Cleanup
**File**: `descartes/core/src/attach.rs`
**Effort**: Small
**Description**: Add TTL enforcement and periodic cleanup of expired tokens.

### Task 2.3: Implement ZMQ WriteStdin Handler
**File**: `descartes/core/src/zmq_server.rs`
**Effort**: Medium
**Description**: Replace stub `WriteStdin` handler with actual implementation that writes to agent stdin.

### Task 2.4: Implement ZMQ ReadStdout Handler
**File**: `descartes/core/src/zmq_server.rs`
**Effort**: Medium
**Description**: Replace stub `ReadStdout` handler with implementation that returns buffered stdout.

### Task 2.5: Implement ZMQ ReadStderr Handler
**File**: `descartes/core/src/zmq_server.rs`
**Effort**: Small
**Description**: Replace stub `ReadStderr` handler similar to ReadStdout.

### Task 2.6: Create AttachSession Struct
**File**: `descartes/daemon/src/attach_session.rs` (new)
**Effort**: Medium
**Description**: Define `AttachSession` struct tracking session_id, agent_id, token, timestamps.

### Task 2.7: Implement AttachSessionManager
**File**: `descartes/daemon/src/attach_session.rs`
**Effort**: Medium
**Description**: Implement session lifecycle management (create, validate, terminate).

### Task 2.8: Add Attach Event Types to EventBus
**File**: `descartes/daemon/src/events.rs`
**Effort**: Small
**Description**: Add `Paused`, `Resumed`, `AttachRequested`, `AttachConnected`, `AttachDisconnected` to `AgentEventType`.

### Task 2.9: Wire AttachTokenStore into Daemon
**File**: `descartes/daemon/src/lib.rs` or `server.rs`
**Effort**: Small
**Description**: Initialize `AttachTokenStore` at daemon startup and make available to RPC handlers.

### Task 2.10: Add Attach Unit Tests
**File**: `descartes/core/tests/attach_tests.rs` (new)
**Effort**: Medium
**Description**: Test token generation, validation, expiration, revocation.

---

## Phase 3: RPC & CLI Integration

### Task 3.1: Add PauseResult/ResumeResult Types
**File**: `descartes/daemon/src/types.rs`
**Effort**: Small
**Description**: Define RPC response types for pause and resume operations.

### Task 3.2: Add AttachCredentials/AttachValidation Types
**File**: `descartes/daemon/src/types.rs`
**Effort**: Small
**Description**: Define RPC response types for attach operations.

### Task 3.3: Implement agent.pause RPC Method
**File**: `descartes/daemon/src/rpc_server.rs`
**Effort**: Medium
**Description**: Add `pause_agent` method to RPC trait and implementation.

### Task 3.4: Implement agent.resume RPC Method
**File**: `descartes/daemon/src/rpc_server.rs`
**Effort**: Medium
**Description**: Add `resume_agent` method to RPC trait and implementation.

### Task 3.5: Implement agent.attach.request RPC Method
**File**: `descartes/daemon/src/rpc_server.rs`
**Effort**: Medium
**Description**: Add method to generate attach credentials for paused agents.

### Task 3.6: Implement agent.attach.validate RPC Method
**File**: `descartes/daemon/src/rpc_server.rs`
**Effort**: Small
**Description**: Add method to validate attach tokens.

### Task 3.7: Implement agent.attach.revoke RPC Method
**File**: `descartes/daemon/src/rpc_server.rs`
**Effort**: Small
**Description**: Add method to revoke attach tokens.

### Task 3.8: Update process_single_request for New Methods
**File**: `descartes/daemon/src/rpc_server.rs`
**Effort**: Small
**Description**: Add dispatch cases for new RPC methods in `process_single_request`.

### Task 3.9: Create pause CLI Command
**File**: `descartes/cli/src/commands/pause.rs` (new)
**Effort**: Medium
**Description**: Implement `descartes pause` command with `--force` flag.

### Task 3.10: Create resume CLI Command
**File**: `descartes/cli/src/commands/resume.rs` (new)
**Effort**: Small
**Description**: Implement `descartes resume` command.

### Task 3.11: Create attach CLI Command
**File**: `descartes/cli/src/commands/attach.rs` (new)
**Effort**: Medium
**Description**: Implement `descartes attach` with `--client`, `--launch`, `--info-only` flags.

### Task 3.12: Register New Commands in CLI
**File**: `descartes/cli/src/commands/mod.rs`, `main.rs`
**Effort**: Small
**Description**: Add pause, resume, attach modules and wire into command dispatch.

### Task 3.13: Add CLI RPC Client Methods
**File**: `descartes/daemon/src/client.rs`
**Effort**: Small
**Description**: Add `pause_agent`, `resume_agent`, `request_attach` helper methods to RpcClient.

### Task 3.14: Add RPC Integration Tests
**File**: `descartes/daemon/tests/rpc_pause_attach_tests.rs` (new)
**Effort**: Medium
**Description**: Integration tests for pause/resume/attach RPC methods.

---

## Phase 4: Claude Code TUI Attachment

### Task 4.1: Create TUI Launchers Module
**File**: `descartes/cli/src/tui_launchers/mod.rs` (new)
**Effort**: Small
**Description**: Create module structure for TUI launcher implementations.

### Task 4.2: Implement Claude Code Launcher
**File**: `descartes/cli/src/tui_launchers/claude_code.rs` (new)
**Effort**: Medium
**Description**: Implement `launch()` and `is_available()` for Claude Code.

### Task 4.3: Define Attach Protocol Messages
**File**: `descartes/core/src/attach_protocol.rs` (new)
**Effort**: Medium
**Description**: Define `AttachHandshake`, `AttachHandshakeResponse`, `HistoricalOutput` types.

### Task 4.4: Create AttachEndpoint Struct
**File**: `descartes/daemon/src/attach_endpoint.rs` (new)
**Effort**: Medium
**Description**: Define ZMQ ROUTER-based endpoint for attach sessions.

### Task 4.5: Implement Handshake Handler
**File**: `descartes/daemon/src/attach_endpoint.rs`
**Effort**: Medium
**Description**: Implement token validation and session creation in handshake.

### Task 4.6: Implement Stdin/Stdout Proxying in AttachEndpoint
**File**: `descartes/daemon/src/attach_endpoint.rs`
**Effort**: Medium
**Description**: Proxy stdin writes and stdout reads through ZMQ.

### Task 4.7: Wire AttachEndpoint into Daemon
**File**: `descartes/daemon/src/server.rs`
**Effort**: Small
**Description**: Start attach endpoint when daemon starts, on configurable port/socket.

### Task 4.8: Integrate Launcher into attach Command
**File**: `descartes/cli/src/commands/attach.rs`
**Effort**: Small
**Description**: Call `tui_launchers::claude_code::launch()` when `--client claude-code`.

### Task 4.9: Add Attach Endpoint Tests
**File**: `descartes/daemon/tests/attach_endpoint_tests.rs` (new)
**Effort**: Medium
**Description**: Test handshake, stdin/stdout proxying, disconnect handling.

---

## Phase 5: OpenCode TUI Attachment

### Task 5.1: Implement OpenCode Launcher
**File**: `descartes/cli/src/tui_launchers/opencode.rs` (new)
**Effort**: Medium
**Description**: Implement `launch()` and `is_available()` for OpenCode.

### Task 5.2: Integrate OpenCode Launcher into attach Command
**File**: `descartes/cli/src/commands/attach.rs`
**Effort**: Small
**Description**: Call `tui_launchers::opencode::launch()` when `--client opencode`.

### Task 5.3: Document SSH Tunneling for Remote Attach
**File**: `descartes/docs/SSH_ATTACHMENT.md` (new)
**Effort**: Small
**Description**: Write documentation for future SSH tunnel support.

### Task 5.4: Add OpenCode Launcher Tests
**File**: `descartes/cli/tests/opencode_launcher_tests.rs` (new)
**Effort**: Small
**Description**: Test launcher behavior when OpenCode is/isn't installed.

---

## Phase 6: GUI & Telemetry Integration

### Task 6.1: Add Pause Button to Agent Panel
**File**: `descartes/gui/src/agent_panel.rs` (or equivalent)
**Effort**: Medium
**Description**: Add conditional Pause button for running agents.

### Task 6.2: Add Resume/Attach Buttons to Agent Panel
**File**: `descartes/gui/src/agent_panel.rs`
**Effort**: Medium
**Description**: Add Resume and Attach buttons for paused agents.

### Task 6.3: Add GUI Message Types for Pause/Attach
**File**: `descartes/gui/src/event_handler.rs`
**Effort**: Small
**Description**: Add `PauseAgent`, `ResumeAgent`, `AttachToAgent`, `AttachCredentialsReceived` messages.

### Task 6.4: Implement Pause/Resume Message Handlers
**File**: `descartes/gui/src/event_handler.rs`
**Effort**: Medium
**Description**: Handle pause/resume messages by calling RPC and updating UI.

### Task 6.5: Implement Attach Message Handler
**File**: `descartes/gui/src/event_handler.rs`
**Effort**: Medium
**Description**: Request credentials then launch TUI on attach button click.

### Task 6.6: Define Telemetry Event Types
**File**: `descartes/daemon/src/telemetry.rs` (new or extend)
**Effort**: Small
**Description**: Define `AgentPaused`, `AgentResumed`, `AttachSessionStarted`, `AttachSessionEnded`.

### Task 6.7: Emit Telemetry Events
**File**: Various (pause/resume/attach handlers)
**Effort**: Small
**Description**: Emit telemetry events at appropriate points in pause/attach flow.

### Task 6.8: Add Pause/Attach Metrics
**File**: `descartes/daemon/src/metrics.rs`
**Effort**: Small
**Description**: Add counters and histograms for pause/attach operations.

### Task 6.9: Add GUI Tests for Pause/Attach UI
**File**: `descartes/gui/tests/pause_attach_ui_tests.rs` (new)
**Effort**: Medium
**Description**: Test button visibility, message handling, state transitions.

---

## Task Summary by Phase

| Phase | Task Count | Estimated Effort |
|-------|------------|------------------|
| Phase 1: Core Pause/Resume | 9 tasks | Medium |
| Phase 2: Attach Infrastructure | 10 tasks | Medium-Large |
| Phase 3: RPC & CLI | 14 tasks | Medium |
| Phase 4: Claude Code | 9 tasks | Medium |
| Phase 5: OpenCode | 4 tasks | Small |
| Phase 6: GUI & Telemetry | 9 tasks | Medium |
| **Total** | **55 tasks** | |

---

## Dependency Graph

```
Phase 1 (Core)
    ├── Task 1.1-1.4 (Types) ─────────────────────┐
    └── Task 1.5-1.9 (Implementation) ────────────┤
                                                   ↓
Phase 2 (Attach Infra)                            │
    ├── Task 2.1-2.2 (Tokens) ───────────────────┤
    ├── Task 2.3-2.5 (ZMQ Handlers) ─────────────┤
    └── Task 2.6-2.10 (Sessions/Events) ─────────┤
                                                   ↓
Phase 3 (RPC/CLI)                                 │
    ├── Task 3.1-3.8 (RPC Methods) ──────────────┤
    └── Task 3.9-3.14 (CLI Commands) ────────────┤
                                                   ↓
Phase 4 (Claude Code) ────────────────────────────┤
                                                   ↓
Phase 5 (OpenCode) ───────────────────────────────┤
                                                   ↓
Phase 6 (GUI/Telemetry) ──────────────────────────┘
```

---

## Implementation Order Recommendation

For a single developer, work through phases sequentially. For parallel development:

**Track A (Core/Backend)**:
- Phase 1 → Phase 2 (Tasks 2.1-2.5) → Phase 3 (Tasks 3.1-3.8)

**Track B (CLI/TUI)**:
- Phase 3 (Tasks 3.9-3.14) → Phase 4 → Phase 5

**Track C (GUI)**:
- Phase 6 (after Phase 3 RPC is complete)

---

## Risk Areas

1. **SIGSTOP on Unix**: Test thoroughly on Linux/macOS. May behave differently with container runtimes.

2. **ZMQ Socket Lifecycle**: Ensure sockets are properly cleaned up on disconnect to avoid resource leaks.

3. **Claude Code/OpenCode Integration**: Depends on external tools accepting connection parameters. May need coordination with tool maintainers.

4. **Token Security**: Tokens are simple UUIDs without cryptographic signing. Acceptable for initial implementation but flag for security review.
