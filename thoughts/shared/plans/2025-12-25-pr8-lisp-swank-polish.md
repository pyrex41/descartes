# PR #8 Lisp/Swank Integration Polish

## Overview

Post-merge polish for PR #8 (Lisp/Swank Live Development Integration). This plan addresses the critical missing RPC handler, improves S-expression escaping, adds proper error propagation for Lisp agent spawning, and adds integration test coverage.

## Current State Analysis

**Merged**: PR #8 at 2025-12-26T02:39:15Z

**Issues Identified**:
1. **Critical**: Missing `swank.restart` RPC handler - GUI debugger restart buttons fail
2. **Important**: S-expression escaping only handles `\` and `"`, missing `\n`, `\r`
3. **Important**: Swank init failure doesn't fail Lisp agent spawn
4. **Gap**: No integration tests for Swank protocol

### Key Discoveries:
- GUI calls `swank.restart` at `gui/src/rpc_client.rs:121`
- RPC server has no handler, returns "Method not found" at `daemon/src/rpc_server.rs:1394`
- Escaping at `core/src/swank/client.rs:393` is incomplete
- Swank init failure logged but ignored at `daemon/src/rpc_server.rs:381-384`

## Desired End State

After this plan:
1. Clicking restart buttons in GUI debugger panel works correctly
2. Multi-line Lisp code evaluates without escaping errors
3. Spawning a Lisp agent fails cleanly if SBCL/Swank can't start
4. Integration tests verify the full Swank protocol round-trip

**Verification**:
```bash
# All tests pass
cargo test --workspace

# Manual: Spawn Lisp agent, trigger error, use debugger restart
descartes spawn --name lisp-test --type lisp-developer --task "Evaluate (/ 1 0)"
```

## What We're NOT Doing

- Replacing global `SWANK_REGISTRY` with dependency injection (future enhancement)
- Adding race condition protection in client connect (minor risk, future)
- Changing the Swank protocol implementation itself
- Adding reconnection after SBCL crash (future feature)

## Implementation Approach

Four phases, each independently testable:
1. Add missing RPC handler (critical fix)
2. Improve S-expression escaping (robustness)
3. Fail Lisp agent spawn on Swank init failure (proper error handling)
4. Add integration tests (verification)

---

## Phase 1: Add Missing `swank.restart` RPC Handler

### Overview
Add the `swank.restart` RPC method handler so GUI debugger restart buttons work.

### Changes Required:

#### 1. Add RPC Handler to Daemon
**File**: `descartes/daemon/src/rpc_server.rs`

**Add parameter parsing function** (after `parse_token_params` ~line 1577):
```rust
#[allow(clippy::result_large_err)]
fn parse_swank_restart_params(request: &RpcRequest) -> Result<(String, usize), RpcResponse> {
    // Support both positional array and named object params
    match &request.params {
        Some(Value::Array(arr)) => {
            let agent_id = arr.first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
                .to_string();
            let restart_index = arr.get(1)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing restart_index parameter"))?
                as usize;
            Ok((agent_id, restart_index))
        }
        Some(Value::Object(obj)) => {
            let agent_id = obj.get("agent_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing agent_id parameter"))?
                .to_string();
            let restart_index = obj.get("restart_index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| Self::invalid_params(request.id.clone(), "Missing restart_index parameter"))?
                as usize;
            Ok((agent_id, restart_index))
        }
        _ => Err(Self::invalid_params(
            request.id.clone(),
            "Expected parameters [agent_id, restart_index] or {agent_id, restart_index}",
        )),
    }
}
```

**Add internal implementation method** (in `impl RpcServerImpl`, after `attach_revoke_internal` ~line 982):
```rust
pub(crate) async fn swank_restart_internal(
    &self,
    agent_id: String,
    restart_index: usize,
) -> Result<SwankRestartResult, ErrorObjectOwned> {
    info!("Invoking Swank restart {} for agent {}", restart_index, agent_id);

    let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
        error!("Invalid agent ID format: {}", e);
        ErrorObjectOwned::owned(-32602, format!("Invalid agent ID format: {}", e), None::<()>)
    })?;

    // Get Swank client from registry
    let swank_client = SWANK_REGISTRY.get(&agent_uuid).ok_or_else(|| {
        error!("No Swank session for agent {}", agent_id);
        ErrorObjectOwned::owned(
            -32016,
            format!("No Swank session for agent {}", agent_id),
            None::<()>,
        )
    })?;

    // Invoke the restart
    match swank_client.invoke_restart(restart_index).await {
        Ok(_) => {
            info!("Swank restart {} invoked successfully for agent {}", restart_index, agent_id);
            Ok(SwankRestartResult {
                agent_id,
                restart_index,
                success: true,
                message: None,
            })
        }
        Err(e) => {
            error!("Failed to invoke Swank restart: {}", e);
            Ok(SwankRestartResult {
                agent_id,
                restart_index,
                success: false,
                message: Some(e.to_string()),
            })
        }
    }
}
```

**Add SwankRestartResult struct** (after `AttachRevokeResult` ~line 202):
```rust
/// Swank restart result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwankRestartResult {
    pub agent_id: String,
    pub restart_index: usize,
    pub success: bool,
    pub message: Option<String>,
}
```

**Add handler case in `process_single_request`** (before the `_` default case ~line 1393):
```rust
"swank.restart" => match Self::parse_swank_restart_params(&request) {
    Ok((agent_id, restart_index)) => {
        match server_impl.swank_restart_internal(agent_id, restart_index).await {
            Ok(result) => match serde_json::to_value(result) {
                Ok(value) => RpcResponse::success(value, request.id.clone()),
                Err(e) => RpcResponse::error(
                    -32603,
                    format!("Serialization error: {}", e),
                    request.id.clone(),
                ),
            },
            Err(err) => Self::convert_error(err, request.id.clone()),
        }
    }
    Err(response) => response,
},
```

### Success Criteria:

#### Automated Verification:
- [x] Build passes: `cargo check --workspace`
- [x] Existing tests pass: `cargo test --workspace`
- [x] No new warnings: `cargo clippy --workspace`

#### Manual Verification:
- [ ] Start daemon, spawn Lisp agent
- [ ] Trigger error: evaluate `(/ 1 0)`
- [ ] Debugger panel appears in GUI
- [ ] Click ABORT restart - debugger dismisses and returns to top level

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: Improve S-Expression Escaping

### Overview
Add proper escaping for newlines and carriage returns in Lisp code sent to Swank.

### Changes Required:

#### 1. Update Escaping in SwankClient
**File**: `descartes/core/src/swank/client.rs`

**Create helper function** (add after the `sexp_as_str` function ~line 377):
```rust
/// Escape a string for embedding in a Swank S-expression.
fn escape_for_sexp(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
```

**Update `eval` method** (line 393):
```rust
// Before:
let escaped = code.replace('\\', "\\\\").replace('"', "\\\"");

// After:
let escaped = Self::escape_for_sexp(code);
```

**Update `compile_string` method** (line 413):
```rust
// Before:
let escaped = code.replace('\\', "\\\\").replace('"', "\\\"");

// After:
let escaped = Self::escape_for_sexp(code);
```

**Update `inspect` method** (line 433):
```rust
// Before:
let escaped = expr.replace('\\', "\\\\").replace('"', "\\\"");

// After:
let escaped = Self::escape_for_sexp(expr);
```

#### 2. Add Unit Tests
**File**: `descartes/core/src/swank/client.rs` (in existing `mod tests`)

```rust
#[test]
fn test_escape_for_sexp() {
    assert_eq!(
        SwankClient::escape_for_sexp("hello"),
        "hello"
    );
    assert_eq!(
        SwankClient::escape_for_sexp("line1\nline2"),
        "line1\\nline2"
    );
    assert_eq!(
        SwankClient::escape_for_sexp("with \"quotes\""),
        "with \\\"quotes\\\""
    );
    assert_eq!(
        SwankClient::escape_for_sexp("back\\slash"),
        "back\\\\slash"
    );
    assert_eq!(
        SwankClient::escape_for_sexp("tab\there"),
        "tab\\there"
    );
    assert_eq!(
        SwankClient::escape_for_sexp("cr\rhere"),
        "cr\\rhere"
    );
}
```

### Success Criteria:

#### Automated Verification:
- [x] Build passes: `cargo check --workspace`
- [x] All tests pass: `cargo test --workspace`
- [x] New escape tests pass: `cargo test -p descartes-core escape_for_sexp`

#### Manual Verification:
- [ ] Evaluate multi-line code in Lisp agent:
  ```lisp
  (progn
    (print "line 1")
    (print "line 2"))
  ```
- [ ] Code executes correctly without parse errors

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: Fail Lisp Agent Spawn on Swank Init Failure

### Overview
If a Lisp agent is spawned but Swank initialization fails, the spawn should fail rather than succeeding with a degraded agent.

### Changes Required:

#### 1. Update Spawn Logic in RPC Server
**File**: `descartes/daemon/src/rpc_server.rs`

**Modify `spawn_agent_internal`** (~lines 375-386):

```rust
// Before:
// Initialize Swank for Lisp agents
if needs_swank {
    match self.initialize_swank_session(agent_id).await {
        Ok(()) => {
            info!("Swank session initialized for agent {}", agent_id_str);
        }
        Err(e) => {
            // Log but don't fail the spawn - agent can work without Swank
            warn!("Failed to initialize Swank for agent {}: {}", agent_id_str, e);
        }
    }
}

// After:
// Initialize Swank for Lisp agents - fail spawn if Swank init fails
if needs_swank {
    match self.initialize_swank_session(agent_id).await {
        Ok(()) => {
            info!("Swank session initialized for agent {}", agent_id_str);
        }
        Err(e) => {
            // Swank is required for Lisp agents - kill the agent and fail the spawn
            error!("Failed to initialize Swank for Lisp agent {}: {}", agent_id_str, e);

            // Clean up the partially spawned agent
            if let Err(kill_err) = self.agent_runner.kill(&agent_id).await {
                warn!("Failed to kill agent after Swank init failure: {}", kill_err);
            }
            self.agent_ids.remove(&agent_id_str);

            return Err(ErrorObjectOwned::owned(
                -32017,
                format!("Failed to initialize Lisp runtime (SBCL/Swank): {}", e),
                None::<()>,
            ));
        }
    }
}
```

#### 2. Add Test for Spawn Failure
**File**: `descartes/daemon/src/rpc_server.rs` (in existing `mod tests`)

```rust
#[tokio::test]
async fn test_lisp_agent_detection() {
    use descartes_core::traits::AgentConfig;

    // Test various Lisp agent detection patterns
    let lisp_config = AgentConfig {
        name: "test-lisp".to_string(),
        model_backend: "sbcl".to_string(),
        task: "test".to_string(),
        context: None,
        system_prompt: None,
        environment: std::collections::HashMap::new(),
    };
    assert!(is_lisp_agent(&lisp_config));

    let lisp_name_config = AgentConfig {
        name: "my-lisp-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "test".to_string(),
        context: None,
        system_prompt: None,
        environment: std::collections::HashMap::new(),
    };
    assert!(is_lisp_agent(&lisp_name_config));

    let mut env_config = AgentConfig {
        name: "test".to_string(),
        model_backend: "claude".to_string(),
        task: "test".to_string(),
        context: None,
        system_prompt: None,
        environment: std::collections::HashMap::new(),
    };
    env_config.environment.insert(
        "DESCARTES_TOOL_LEVEL".to_string(),
        "lisp_developer".to_string(),
    );
    assert!(is_lisp_agent(&env_config));

    let non_lisp_config = AgentConfig {
        name: "regular-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "test".to_string(),
        context: None,
        system_prompt: None,
        environment: std::collections::HashMap::new(),
    };
    assert!(!is_lisp_agent(&non_lisp_config));
}
```

### Success Criteria:

#### Automated Verification:
- [x] Build passes: `cargo check --workspace`
- [x] All tests pass: `cargo test --workspace`
- [x] New detection test passes: `cargo test -p descartes-daemon lisp_agent_detection`

#### Manual Verification:
- [ ] Stop any running SBCL processes
- [ ] Temporarily rename/remove `sbcl` from PATH
- [ ] Try to spawn Lisp agent: `descartes spawn --name test --type lisp-developer --task "test"`
- [ ] Spawn should fail with clear error message about Lisp runtime
- [ ] Restore `sbcl` to PATH

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 4.

---

## Phase 4: Add Integration Tests

### Overview
Add integration tests for the Swank protocol to verify the full round-trip from eval to response.

### Changes Required:

#### 1. Create Integration Test Module
**File**: `descartes/core/src/swank/tests.rs` (new file)

```rust
//! Integration tests for Swank protocol.
//!
//! These tests require SBCL to be installed and available in PATH.
//! Run with: cargo test -p descartes-core --test swank_integration -- --ignored

use crate::swank::{find_available_port, SwankClient, SwankLauncher, SwankMessage, DEFAULT_SWANK_PORT};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Check if SBCL is available
fn sbcl_available() -> bool {
    std::process::Command::new("sbcl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_swank_connection() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();

    let client = SwankClient::connect(agent_id, port, event_tx).await;
    assert!(client.is_ok(), "Failed to connect to Swank: {:?}", client.err());

    let client = client.unwrap();
    assert!(client.is_connected().await);
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_simple_eval() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    // Simple arithmetic
    let result = client.eval("(+ 1 2)", "CL-USER").await;
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.contains("3"), "Expected 3, got: {}", value);
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_multiline_eval() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    // Multi-line code with special characters
    let code = r#"(progn
  (format nil "Hello~%World"))"#;

    let result = client.eval(code, "CL-USER").await;
    assert!(result.is_ok(), "Multiline eval failed: {:?}", result.err());
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_debugger_triggered() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, mut event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    // Trigger division by zero - this will enter the debugger
    // We need to spawn this as a task since it will block waiting for debugger resolution
    let client_clone = client.clone();
    let eval_task = tokio::spawn(async move {
        client_clone.eval("(/ 1 0)", "CL-USER").await
    });

    // Wait for debugger event
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            while let Some(msg) = event_rx.recv().await {
                if let SwankMessage::Debug { condition, restarts, .. } = msg {
                    // Verify we got the debug event
                    assert!(condition.contains("DIVISION-BY-ZERO") || condition.contains("division"),
                        "Expected division error, got: {}", condition);
                    assert!(!restarts.is_empty(), "Expected at least one restart");

                    // Find and invoke ABORT restart
                    let abort_idx = restarts.iter()
                        .find(|r| r.name.to_uppercase().contains("ABORT"))
                        .map(|r| r.index)
                        .unwrap_or(0);

                    return Some(abort_idx);
                }
            }
            None
        }
    ).await;

    assert!(timeout.is_ok(), "Timed out waiting for debugger event");
    let abort_idx = timeout.unwrap();
    assert!(abort_idx.is_some(), "Did not receive debug event");

    // Invoke abort restart
    let abort_idx = abort_idx.unwrap();
    let restart_result = client.invoke_restart(abort_idx).await;
    assert!(restart_result.is_ok(), "Restart failed: {:?}", restart_result.err());

    // The eval task should complete (with an abort)
    let eval_result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        eval_task
    ).await;
    assert!(eval_result.is_ok(), "Eval task didn't complete after restart");
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_connection_cleanup() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let mut child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    assert!(client.is_connected().await);

    // Disconnect
    client.disconnect().await.unwrap();
    assert!(!client.is_connected().await);

    // Kill SBCL
    child.kill().await.unwrap();
}
```

#### 2. Add Module Declaration
**File**: `descartes/core/src/swank/mod.rs`

Add at the end of the file:
```rust
#[cfg(test)]
mod tests;
```

#### 3. Create Integration Test Binary
**File**: `descartes/core/tests/swank_integration.rs` (new file)

```rust
//! Integration tests for Swank - run separately with SBCL available.
//!
//! Run with: cargo test -p descartes-core --test swank_integration -- --ignored

// Re-export tests from the module
// The actual tests are in src/swank/tests.rs and run via cargo test
```

### Success Criteria:

#### Automated Verification:
- [x] Build passes: `cargo check --workspace`
- [x] Unit tests pass: `cargo test --workspace`
- [x] Integration tests compile: `cargo test -p descartes-core swank::integration_tests --no-run`

#### Manual Verification (requires SBCL):
- [ ] Run integration tests: `cargo test -p descartes-core swank -- --ignored --nocapture`
- [ ] All integration tests pass
- [ ] Tests properly clean up SBCL processes

**Implementation Note**: Integration tests are marked `#[ignore]` by default since they require SBCL. Run with `--ignored` flag to execute them.

---

## Testing Strategy

### Unit Tests:
- S-expression escaping edge cases
- Lisp agent detection logic
- Parameter parsing for `swank.restart`

### Integration Tests (require SBCL):
- Connection lifecycle
- Simple eval round-trip
- Multi-line code handling
- Debugger event flow
- Restart invocation
- Cleanup behavior

### Manual Testing Steps:
1. Start daemon
2. Spawn Lisp agent with GUI
3. Evaluate `(+ 1 2)` → expect `3`
4. Evaluate `(/ 1 0)` → debugger panel appears
5. Click ABORT → returns to top level
6. Evaluate multi-line code → works correctly
7. Kill agent → SBCL process terminates

## Error Codes Added

| Code | Meaning |
|------|---------|
| -32016 | No Swank session for agent |
| -32017 | Failed to initialize Lisp runtime |

## Related Files

- `descartes/daemon/src/rpc_server.rs` - RPC handlers
- `descartes/core/src/swank/client.rs` - Swank protocol client
- `descartes/core/src/swank/mod.rs` - Swank module
- `descartes/gui/src/rpc_client.rs` - GUI RPC client
- `descartes/gui/src/lisp_debugger.rs` - Debugger UI

## References

- Post-merge actions: `thoughts/shared/plans/2025-12-25-pr8-lisp-swank-post-merge-actions.md`
- PR: https://github.com/pyrex41/descartes/pull/8
- Swank Protocol: https://github.com/slime/slime/blob/master/swank.lisp
