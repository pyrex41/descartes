# Daemon Compilation Fixes - November 26, 2025

## Context

The Descartes daemon had compilation errors blocking the GUI build. The errors stemmed from API mismatches between the `AttachSessionManager` implementation and its usage in `rpc_server.rs` and the TUI handlers.

## Root Cause Analysis

### 1. AttachSessionManager API Evolution

The `AttachSessionManager::new` signature changed from taking just a config to requiring three arguments:
```rust
// Old API (expected by rpc_server.rs)
AttachSessionManager::new(config)

// New API (actual implementation)
AttachSessionManager::new(token_store: Arc<AttachTokenStore>, event_bus: Arc<EventBus>, config: AttachSessionConfig)
```

This change was part of Phase 5/6 work on pause/attach integration, which added proper token management and event emission.

### 2. Session vs Credentials Flow

The code was conflating two different flows:
- **Credentials Request**: Client requests credentials to attach → returns `AttachCredentials` (token, URL, expiry)
- **Session Creation**: After successful handshake, session is created → returns `AttachSession`

The `rpc_server.rs` was incorrectly calling `create_session` when it should have called `request_attach`.

### 3. Borrow Checker Conflicts in TUI Handlers

The `tokio::select!` macro in `run_io_loop` had borrow conflicts:
```rust
tokio::select! {
    result = self.read_message(reader) => { ... }  // borrows self
    result = self.stdout_rx.recv() => { ... }      // mutably borrows self.stdout_rx
}
```

The `read_message` method took `&self` even though it didn't use `self`, causing the borrow checker to hold an immutable borrow across all select branches.

## Fixes Applied

### 1. RpcServerImpl Constructor (rpc_server.rs:214-234)

```rust
pub fn new(...) -> Self {
    let attach_config = AttachSessionConfig::default();
    let token_store = Arc::new(descartes_core::AttachTokenStore::new());
    let event_bus = Arc::new(crate::events::EventBus::new());
    let attach_manager = Arc::new(AttachSessionManager::new(
        Arc::clone(&token_store),
        Arc::clone(&event_bus),
        attach_config,
    ));
    Self {
        agent_runner,
        state_store,
        agent_ids: Arc::new(dashmap::DashMap::new()),
        attach_manager,
        event_bus,
    }
}
```

### 2. Credentials Flow (rpc_server.rs:623-640)

Changed from `create_session` to `request_attach`:
```rust
let credentials = self
    .attach_manager
    .request_attach(agent_uuid, parsed_client_type)
    .await?;

Ok(AttachCredentialsResult {
    agent_id,
    token: credentials.token,
    connect_url: credentials.connect_url,
    expires_at: credentials.expires_at,
})
```

### 3. Token Validation (rpc_server.rs:649-668)

Updated to handle `Option<Uuid>` return type instead of session info:
```rust
match self.attach_manager.validate_token(&token).await {
    Some(agent_id) => {
        Ok(AttachValidateResult {
            valid: true,
            agent_id: Some(agent_id.to_string()),
            expires_at: None, // Not exposed by validate_token
        })
    }
    None => { ... }
}
```

### 4. Token Revocation (rpc_server.rs:677-687)

Use token store directly instead of non-existent `revoke_session`:
```rust
let token_store = self.attach_manager.token_store();
let revoked = token_store.revoke(&token).await;
```

### 5. TUI Handler Borrow Fixes (claude_code_tui.rs, opencode_tui.rs)

Added static helper methods that don't take `&self`:
```rust
async fn read_message_static<R>(reader: &mut BufReader<R>) -> DaemonResult<AttachMessage>
async fn send_message_static<W>(writer: &mut W, msg: &AttachMessage) -> DaemonResult<()>
async fn handle_client_message_static<W>(msg: AttachMessage, writer: &mut W, stdin_tx: &mpsc::Sender<Vec<u8>>) -> DaemonResult<bool>
```

Then extracted mutable fields before the select loop:
```rust
let stdout_rx = &mut self.stdout_rx;
let stderr_rx = &mut self.stderr_rx;
let output_buffer = &self.output_buffer;
let stdin_tx = &self.stdin_tx;

loop {
    tokio::select! {
        result = Self::read_message_static(reader) => { ... }
        result = stdout_rx.recv() => { ... }
        result = stderr_rx.recv() => { ... }
    }
}
```

### 6. Minor Fixes

- Added `base64 = "0.21"` to daemon's Cargo.toml
- Fixed `StdinData.decode_data()` → `to_bytes()`
- Fixed `AttachMessage::error(msg, None)` → `AttachMessage::error(&msg)`
- Updated TUI handler function signatures from `AttachSessionInfo` to `Uuid`
- Added `event_bus` field to `RpcServerImpl::Clone` impl

## Current Build Status

| Package | Status | Warnings |
|---------|--------|----------|
| descartes-core | Compiles | 51 |
| descartes-daemon | Compiles | 28 |
| descartes-gui | Compiles | 107 |

Most warnings are:
- Unused imports (can be cleaned with `cargo fix`)
- Lifetime elision hints in GUI
- Dead code (methods defined but not yet called)

## Files Modified

1. `descartes/daemon/src/rpc_server.rs` - Constructor, attach methods, Clone impl
2. `descartes/daemon/src/attach_session.rs` - Token borrow fix in request_attach
3. `descartes/daemon/src/claude_code_tui.rs` - Static methods, borrow fixes
4. `descartes/daemon/src/opencode_tui.rs` - Static methods, borrow fixes
5. `descartes/daemon/Cargo.toml` - Added base64 dependency

## Architectural Notes

The attach session system has a clean separation:
1. **AttachTokenStore** (core) - Token generation, validation, revocation
2. **AttachSessionManager** (daemon) - Session lifecycle, event emission
3. **TUI Handlers** (daemon) - Protocol handling, I/O forwarding

The flow is:
```
Client Request → request_attach() → AttachCredentials (token, URL)
                     ↓
Client Connect → Handshake with token
                     ↓
               validate_token() → Uuid (if valid)
                     ↓
               create_session() → AttachSession
                     ↓
               run_io_loop() → stdin/stdout forwarding
                     ↓
               terminate_session() → cleanup
```

## Next Steps

Per project assessment, the next work should focus on:
1. **Phase 3.9.5+**: Interactive Context Browser, Visual DAG Editor
2. **Integration Testing**: End-to-end flows between CLI, Daemon, GUI
3. **Warning Cleanup**: Run `cargo fix` to address unused imports
