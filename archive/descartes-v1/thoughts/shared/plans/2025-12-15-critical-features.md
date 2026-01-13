# Plan 2: Critical Features Implementation

## Implementation Status: ✅ COMPLETED (2025-12-15)

### Summary of Changes

**New Files:**
- `core/src/expression_eval.rs` - Complete expression evaluator with AST parsing, JSON path access, comparisons, boolean/arithmetic operators

**Modified Files:**
- `core/Cargo.toml` - Added `async-stream = "0.3"` dependency
- `core/src/lib.rs` - Added expression_eval module export
- `core/src/traits.rs` - Added `get_agent_pid()` to AgentRunner trait, added `Streaming` variant to FinishReason
- `core/src/agent_runner.rs` - Implemented `get_agent_pid()` for LocalProcessRunner
- `core/src/zmq_server.rs` - Implemented Signal, Pause, Resume commands
- `core/src/providers.rs` - Implemented streaming for OpenAI, Anthropic, Grok, Ollama providers
- `core/src/debugger.rs` - Integrated expression evaluator for conditional breakpoints and Evaluate command
- `core/src/time_travel_integration.rs` - Fixed agent_id tracking in RewindPoint
- `daemon/src/handlers.rs` - Added runner/state_store integration for actual log/state fetching
- `core/tests/time_travel_integration_tests.rs` - Updated test RewindPoint structs with agent_id field

### Deferred Items
- WriteStdin/ReadStdout/ReadStderr - Requires significant AgentHandle interface changes
- StreamLogs - Requires separate ZMQ PUB/SUB architecture
- Full time travel + debugger integration - Complex state restoration needs more design

---

## Overview

This plan covers implementation of critical missing features identified during the codebase audit:
1. **ZMQ Control Commands** (9 unimplemented commands)
2. **Provider Streaming** (4 providers with unimplemented streaming)
3. **Debugger Expression Evaluation** (2 TODOs)
4. **Debugger Integration TODOs** (4 additional TODOs)

---

## Part A: ZMQ Control Commands

### Location
`core/src/zmq_server.rs:494-576`

### Commands by Priority

| Command | Lines | Complexity | Priority |
|---------|-------|------------|----------|
| Pause | 495-500 | Medium | High |
| Resume | 501-506 | Medium | High |
| Signal | 553-558 | Low | Medium |
| WriteStdin | 534-540 | High | Low |
| ReadStdout | 541-546 | High | Low |
| ReadStderr | 547-552 | High | Low |
| CustomAction | 559-564 | Medium | Low |
| QueryOutput | 565-570 | Medium | Low |
| StreamLogs | 571-576 | Very High | Medium |

### Implementation Details

#### A1. Pause/Resume Commands (SIGTSTP/SIGCONT)

```rust
// core/src/zmq_server.rs

ControlCommandType::Pause => {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Get PID from agent handle
        if let Some(pid) = self.runner.get_agent_pid(&agent_id).await? {
            kill(Pid::from_raw(pid as i32), Signal::SIGTSTP)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to pause: {}", e)))?;
            Ok(None)
        } else {
            Err(AgentError::NotFound("Agent process not found".to_string()))
        }
    }
    #[cfg(not(unix))]
    {
        Err(AgentError::UnsupportedPlatform("Pause only supported on Unix".to_string()))
    }
}
```

**Required Changes**:
1. Add `nix` crate dependency to `core/Cargo.toml`
2. Extend `AgentRunner` trait with `get_agent_pid()` method
3. Implement `get_agent_pid()` in all AgentRunner implementations

#### A2. Signal Command

```rust
ControlCommandType::Signal => {
    let signal_num = command.payload
        .get("signal")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AgentError::InvalidInput("Missing signal number".to_string()))?;

    self.runner
        .signal(&agent_id, AgentSignal::Custom(signal_num as i32))
        .await
        .map(|_| None)
}
```

**Required Changes**:
1. Add `AgentSignal::Custom(i32)` variant to signal enum
2. Handle custom signals in runner implementations

#### A3. WriteStdin/ReadStdout/ReadStderr

These require extending the `AgentHandle` interface to expose stdio streams.

```rust
// core/src/agent_runner.rs - new trait methods
pub trait AgentRunner {
    // ... existing methods ...

    async fn write_stdin(&self, agent_id: &Uuid, data: &[u8]) -> AgentResult<()>;
    async fn read_stdout(&self, agent_id: &Uuid, max_bytes: usize) -> AgentResult<Vec<u8>>;
    async fn read_stderr(&self, agent_id: &Uuid, max_bytes: usize) -> AgentResult<Vec<u8>>;
}
```

**Note**: This is a significant interface change affecting all runner implementations.

#### A4. StreamLogs Command

Most complex - requires:
1. Subscribe to log stream for agent
2. Return streaming response over ZMQ
3. Handle backpressure and cancellation

Consider implementing as a separate ZMQ PUB socket that clients SUB to.

---

## Part B: Provider Streaming

### Location
`core/src/providers.rs`

### Providers Needing Streaming

| Provider | Lines | API Type | Streaming Method |
|----------|-------|----------|------------------|
| OpenAiProvider | 125-131 | HTTP SSE | `stream: true` |
| AnthropicProvider | 249-255 | HTTP SSE | `stream: true` |
| GrokProvider | 383-389 | HTTP SSE | Similar to OpenAI |
| OllamaProvider | 717-723 | HTTP JSONL | Line-by-line |

### Implementation Pattern (OpenAI example)

```rust
async fn stream(
    &self,
    request: ModelRequest,
) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<ModelResponse>> + Unpin + Send>> {
    let client = self.client.as_ref()
        .ok_or(ProviderError::NotInitialized)?;

    let response = client
        .post(&format!("{}/chat/completions", self.endpoint))
        .json(&serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "stream": true,
        }))
        .send()
        .await?;

    let stream = response.bytes_stream()
        .map(|chunk| {
            // Parse SSE format: data: {...}\n\n
            let text = String::from_utf8_lossy(&chunk?);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return Ok(ModelResponse::Done);
                    }
                    let parsed: serde_json::Value = serde_json::from_str(data)?;
                    // Extract delta content
                    let content = parsed["choices"][0]["delta"]["content"]
                        .as_str()
                        .unwrap_or("");
                    return Ok(ModelResponse::Chunk { content: content.to_string() });
                }
            }
            Ok(ModelResponse::Empty)
        });

    Ok(Box::new(stream))
}
```

### Implementation Steps

1. **Add dependencies** to `core/Cargo.toml`:
   ```toml
   futures = "0.3"
   async-stream = "0.3"  # Optional, for async_stream! macro
   ```

2. **Implement OpenAI streaming** (reference implementation)

3. **Implement Anthropic streaming** (uses different SSE format)

4. **Implement Grok streaming** (follows OpenAI format)

5. **Implement Ollama streaming** (JSONL format, not SSE)

---

## Part C: Debugger Expression Evaluation

### Location
`core/src/debugger.rs`

### TODOs

1. **Line 488**: Conditional breakpoint evaluation
   ```rust
   // TODO: Evaluate condition if present
   if self.condition.is_some() {
       // Would need an expression evaluator here
   }
   ```

2. **Line 1361**: Evaluate command
   ```rust
   DebugCommand::Evaluate { expression } => {
       // TODO: Implement expression evaluation
       Ok(CommandResult::EvaluationResult {
           expression: expression.clone(),
           result: serde_json::json!({
               "error": "Expression evaluation not yet implemented",
           }),
       })
   }
   ```

### Implementation Approach

Create a simple expression evaluator for JSON paths and basic operations:

```rust
// core/src/expression_eval.rs (new file)

pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate an expression against the current context
    pub fn evaluate(
        expression: &str,
        context: &DebugContext,
    ) -> Result<serde_json::Value, EvalError> {
        // Support:
        // 1. JSON path: "state.agent.status"
        // 2. Comparisons: "state.count > 5"
        // 3. Boolean: "state.running && state.healthy"

        let parsed = Self::parse(expression)?;
        Self::eval_ast(&parsed, context)
    }

    fn parse(expression: &str) -> Result<Ast, EvalError> {
        // Simple recursive descent parser
        // ...
    }

    fn eval_ast(ast: &Ast, context: &DebugContext) -> Result<serde_json::Value, EvalError> {
        // Evaluate AST node
        // ...
    }
}
```

### Expressions to Support

1. **JSON Path Access**: `state.agent.name`, `context.variables.foo`
2. **Comparisons**: `==`, `!=`, `>`, `<`, `>=`, `<=`
3. **Boolean Operators**: `&&`, `||`, `!`
4. **Numeric Operations**: `+`, `-`, `*`, `/`

---

## Part D: Debugger Integration TODOs

### D1. Time Travel Integration

**File**: `core/src/time_travel_integration.rs`

| Line | TODO | Description |
|------|------|-------------|
| 766 | Get agent_id from context | Currently hardcoded as "agent-1" |
| 899 | Integrate with debugger | Hook debugger instance into restored state |
| 914 | Agent runtime re-initialization | Re-init AgentRunner from restored state |

### D2. Handler TODOs

**File**: `daemon/src/handlers.rs`

| Line | TODO | Description |
|------|------|-------------|
| 117 | Fetch actual logs | Currently returns placeholder LogEntry |
| 180 | Fetch actual state | Currently returns empty HashMap |

### Implementation Notes

**Time Travel + Debugger Integration**:
```rust
// After restoring state, create debugger with restored context
let debugger = Debugger::new();
debugger.restore_from_state(&restored_state)?;
debugger.restore_breakpoints(&breakpoints)?;
```

**Handler Log Fetching**:
```rust
// Connect to SQLite log storage
let logs = self.state_store
    .get_logs_for_agent(&agent_id, limit, since)
    .await?;
```

---

## Implementation Order (Recommended)

### Phase 1: Foundation (Prerequisite for others)
1. ✅ Expression evaluator module (needed for debugger features)
2. ✅ AgentRunner trait extensions (needed for ZMQ commands)

### Phase 2: Quick Wins
1. ✅ ZMQ Signal command (builds on existing signal handling)
2. ✅ Handler log/state fetching (simple state store queries)
3. ✅ Time travel agent_id fix (one-line change)

### Phase 3: Streaming
1. ✅ OpenAI streaming (reference implementation)
2. ✅ Anthropic streaming
3. ✅ Grok streaming
4. ✅ Ollama streaming

### Phase 4: Process Control
1. ✅ Pause/Resume (Unix signals via runner methods)
2. ⏳ WriteStdin/ReadStdout/ReadStderr (stdio access) - Deferred, significant interface change

### Phase 5: Advanced
1. ⏳ StreamLogs (ZMQ streaming architecture) - Deferred, requires separate ZMQ architecture
2. ✅ Debugger expression eval integration
3. ⏳ Full debugger integration with time travel - Deferred, complex state restoration

---

## Dependencies

| Feature | External Crates |
|---------|-----------------|
| Pause/Resume | `nix` (Unix signals) |
| Streaming | `futures`, `async-stream` |
| Expression Eval | None (pure Rust) |

## Risk Assessment

| Feature | Risk | Notes |
|---------|------|-------|
| ZMQ Pause/Resume | Medium | Platform-specific (Unix only) |
| Provider Streaming | Medium | API-specific parsing |
| Expression Eval | Low | Self-contained module |
| Handler TODOs | Low | Simple state store queries |
| Time Travel Integration | High | Complex state restoration |

---

## Files to Create/Modify

### New Files
- `core/src/expression_eval.rs` - Expression evaluator

### Modified Files
- `core/Cargo.toml` - Add dependencies
- `core/src/lib.rs` - Export expression_eval
- `core/src/agent_runner.rs` - Extended trait
- `core/src/zmq_server.rs` - Implement commands
- `core/src/providers.rs` - Add streaming
- `core/src/debugger.rs` - Expression evaluation
- `core/src/time_travel_integration.rs` - Integration fixes
- `daemon/src/handlers.rs` - Log/state fetching
