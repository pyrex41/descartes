# Implementation Plan: Descartes GUI Revival & Grounding Fixes

**Date:** 2026-01-14
**Status:** Ready for Implementation
**Estimated Phases:** 5

---

## Overview

This plan addresses the "not grounded right" issues in Descartes and revives the desktop GUI concept with v2's simpler architecture. The work is organized into 5 independent phases that can be executed in parallel or sequentially.

---

## Phase 1: Delete Orphaned Configuration

**Goal:** Remove orphaned `workflow-state.json` that has no code references.

### Changes

| File | Action |
|------|--------|
| `.scud/workflow-state.json` | DELETE |

### Implementation

```bash
rm /Users/reuben/projects/harnesses/descartes/.scud/workflow-state.json
```

### Success Criteria

- [x] **Automated:** `grep -r "workflow-state" --include="*.rs" .` returns no results
- [x] **Manual:** Descartes still builds and runs normally

---

## Phase 2: Wire Agent Control

**Goal:** Make `/pause`, `/resume`, `/cancel` commands actually work in interactive mode.

### Problem Analysis

The `AgentControl` channel is created but the receiver (`control_rx`) is immediately dropped:

```rust
// session.rs:496-527
let (control_tx, control_rx) = mpsc::channel::<AgentControl>(10);
let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(100);

self.control_tx = Some(control_tx);  // Stored - GOOD
self.event_rx = Some(event_rx);      // Stored - GOOD
// control_rx is NEVER passed to the spawned task - BUG!

tokio::spawn(async move {
    // control_rx not available here!
    match crate::agent::spawn_subagent(&*harness, category, prompt, None).await {
```

### Changes

| File | Lines | Change |
|------|-------|--------|
| `src/interactive/session.rs` | 496-527 | Pass `control_rx` into spawned task |
| `src/agent/subagent.rs` | ~45 | Add `control_rx` parameter to `spawn_subagent` |
| `src/agent/subagent.rs` | ~80-150 | Use `tokio::select!` to check for control messages |

### Implementation Details

#### 2.1 Modify `spawn_subagent` signature

**File:** `src/agent/subagent.rs`

```rust
// BEFORE (around line 45):
pub async fn spawn_subagent(
    harness: &dyn Harness,
    category: &str,
    prompt: &str,
    parent: Option<&SessionHandle>,
) -> Result<SubagentResult>

// AFTER:
pub async fn spawn_subagent(
    harness: &dyn Harness,
    category: &str,
    prompt: &str,
    parent: Option<&SessionHandle>,
    mut control_rx: Option<mpsc::Receiver<AgentControl>>,
) -> Result<SubagentResult>
```

#### 2.2 Add control checking loop

**File:** `src/agent/subagent.rs` (in the main loop, around line 80-150)

```rust
// Replace direct stream iteration with select!
loop {
    tokio::select! {
        // Check for control messages (non-blocking)
        Some(ctrl) = async {
            if let Some(ref mut rx) = control_rx {
                rx.recv().await
            } else {
                std::future::pending().await
            }
        } => {
            match ctrl {
                AgentControl::Pause => {
                    // Set paused flag, continue loop but don't process chunks
                    paused = true;
                    let _ = event_tx.send(AgentEvent::Paused).await;
                }
                AgentControl::Resume => {
                    paused = false;
                    let _ = event_tx.send(AgentEvent::Resumed).await;
                }
                AgentControl::Cancel => {
                    // Break out of loop, return partial result
                    let _ = event_tx.send(AgentEvent::Cancelled).await;
                    break;
                }
                AgentControl::Interrupt => {
                    // Same as cancel for now
                    break;
                }
            }
        }

        // Process next chunk from stream (if not paused)
        chunk = stream.next(), if !paused => {
            match chunk {
                Some(c) => { /* existing chunk processing */ }
                None => break,
            }
        }
    }
}
```

#### 2.3 Pass control_rx from session.rs

**File:** `src/interactive/session.rs` (around line 510-527)

```rust
// BEFORE:
tokio::spawn(async move {
    match crate::agent::spawn_subagent(&*harness, category, prompt, None).await {

// AFTER:
tokio::spawn(async move {
    match crate::agent::spawn_subagent(&*harness, category, prompt, None, Some(control_rx)).await {
```

#### 2.4 Update all other spawn_subagent call sites

Search for all usages and add `None` for the new parameter:

```bash
grep -rn "spawn_subagent" --include="*.rs" src/
```

Known call sites to update:
- `src/ralph_loop.rs` - pass `None`
- `src/proxy.rs` - pass `None`
- Any test files

### Success Criteria

- [x] **Automated:** `cargo test` passes
- [x] **Automated:** `cargo clippy` passes
- [ ] **Manual:** In interactive mode, `/pause` visibly stops output, `/resume` continues, `/cancel` stops agent

---

## Phase 3: Complete OpenCode Harness

**Goal:** Fix JSON parsing to match OpenCode's actual Anthropic SSE format.

### Problem Analysis

Current implementation expects:
```json
{"type": "text", "text": "Hello"}
{"type": "tool_use", "id": "...", "name": "...", "input": {...}}
{"type": "done"}
```

OpenCode actually outputs Anthropic SSE format:
```json
{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}
{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}
{"type": "content_block_stop", "index": 0}
{"type": "message_stop"}
```

### Changes

| File | Lines | Change |
|------|-------|--------|
| `src/harness/opencode.rs` | 90-208 | Rewrite `parse_output_line` for Anthropic SSE |

### Implementation Details

**File:** `src/harness/opencode.rs`

Replace the `parse_output_line` method:

```rust
fn parse_output_line(&self, line: &str) -> Option<ResponseChunk> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Handle SSE "data: " prefix if present
    let json_str = line.strip_prefix("data: ").unwrap_or(line);
    if json_str == "[DONE]" {
        return Some(ResponseChunk::Done);
    }

    let json: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            debug!("Failed to parse JSON line: {} - {}", e, line);
            return None;
        }
    };

    let msg_type = json.get("type").and_then(|t| t.as_str())?;

    match msg_type {
        // Anthropic SSE: Content block with text
        "content_block_start" => {
            if let Some(block) = json.get("content_block") {
                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            return Some(ResponseChunk::Text(text.to_string()));
                        }
                    }
                }
                // Tool use block start
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    // Tool use starts here, but input comes in deltas
                    // We'll accumulate and emit on content_block_stop
                    // For now, store partial tool call state
                    return None; // Handle in accumulator
                }
            }
            None
        }

        // Anthropic SSE: Text delta (streaming text)
        "content_block_delta" => {
            if let Some(delta) = json.get("delta") {
                let delta_type = delta.get("type").and_then(|t| t.as_str());
                match delta_type {
                    Some("text_delta") => {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            return Some(ResponseChunk::Text(text.to_string()));
                        }
                    }
                    Some("input_json_delta") => {
                        // Tool input accumulation - handled by caller
                        if let Some(partial) = delta.get("partial_json").and_then(|p| p.as_str()) {
                            debug!("Tool input delta: {}", partial);
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        // Anthropic SSE: Block finished
        "content_block_stop" => {
            // Could emit accumulated tool call here
            None
        }

        // Anthropic SSE: Message complete
        "message_stop" => Some(ResponseChunk::Done),

        // Anthropic SSE: Message start (metadata)
        "message_start" => {
            // Contains model info, usage stats - log but don't emit
            debug!("Message start: {:?}", json);
            None
        }

        // Anthropic SSE: Message delta (stop reason)
        "message_delta" => {
            if let Some(delta) = json.get("delta") {
                if delta.get("stop_reason").is_some() {
                    // Message finishing, done will come in message_stop
                }
            }
            None
        }

        // Legacy format support (keep for compatibility)
        "text" | "assistant" | "content" => {
            json.get("text")
                .or_else(|| json.get("content"))
                .and_then(|t| t.as_str())
                .map(|s| ResponseChunk::Text(s.to_string()))
        }

        "tool_use" | "tool_call" => {
            let name = json.get("name").and_then(|n| n.as_str())?;
            let id = json.get("id").and_then(|i| i.as_str())?;
            let args = json
                .get("input")
                .or_else(|| json.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            if self.is_subagent_tool(name) {
                if let Some(req) = self.extract_subagent_request(name, &args) {
                    return Some(ResponseChunk::SubagentSpawn(req));
                }
            }

            Some(ResponseChunk::ToolCall(ToolCall {
                name: name.to_string(),
                arguments: args,
                id: id.to_string(),
            }))
        }

        "done" | "complete" | "end" => Some(ResponseChunk::Done),

        "error" => {
            let msg = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .or_else(|| json.get("message").and_then(|m| m.as_str()))
                .unwrap_or("Unknown error");
            Some(ResponseChunk::Error(msg.to_string()))
        }

        _ => {
            debug!("Unknown message type: {} - {:?}", msg_type, json);
            None
        }
    }
}
```

### Add Tool Accumulation State

For proper tool use parsing, add state tracking to accumulate tool input across deltas:

```rust
// Add to OpenCodeSession struct
struct OpenCodeSession {
    session_id: Option<String>,
    messages: Vec<ConversationMessage>,
    // NEW: Accumulate tool input across deltas
    pending_tool: Option<PendingToolCall>,
}

#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    input_json: String,
}
```

### Success Criteria

- [x] **Automated:** `cargo test opencode` passes
- [x] **Automated:** New test case parses actual OpenCode output sample
- [ ] **Manual:** `descartes ralph --harness opencode` successfully runs a task

### New Test Case

Add to `src/harness/opencode.rs` tests:

```rust
#[test]
fn test_parse_anthropic_sse_format() {
    let harness = create_test_harness();

    // Text delta
    let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
    let chunk = harness.parse_output_line(line);
    assert!(matches!(chunk, Some(ResponseChunk::Text(t)) if t == "Hello"));

    // Message stop
    let line = r#"{"type":"message_stop"}"#;
    let chunk = harness.parse_output_line(line);
    assert!(matches!(chunk, Some(ResponseChunk::Done)));

    // SSE with data prefix
    let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"World"}}"#;
    let chunk = harness.parse_output_line(line);
    assert!(matches!(chunk, Some(ResponseChunk::Text(t)) if t == "World"));
}
```

---

## Phase 4: Document claude-proxy

**Goal:** Create documentation explaining the claude-proxy binary's purpose and usage.

### Analysis

`claude-proxy` is an OpenAI-compatible HTTP proxy that wraps `claude -p`:
- Runs on `localhost:8765`
- Accepts `/v1/chat/completions` requests
- Converts OpenAI chat format to Claude Code CLI calls
- Returns streaming responses in OpenAI format

**Use case:** Allows BAML or other OpenAI-compatible tools to use Claude Code as a backend.

### Changes

| File | Action |
|------|--------|
| `docs/CLAUDE_PROXY.md` | CREATE |
| `src/bin/claude-proxy.rs` | ADD header doc comment |

### Implementation Details

#### 4.1 Create documentation file

**File:** `docs/CLAUDE_PROXY.md`

```markdown
# claude-proxy

An OpenAI-compatible HTTP proxy that wraps `claude -p` for integration with tools expecting the OpenAI API.

## Purpose

Allows BAML, LangChain, or other OpenAI-compatible tools to use Claude Code as a backend without modifying their code.

## Usage

```bash
# Start the proxy server
claude-proxy

# Or with custom port
CLAUDE_PROXY_PORT=9000 claude-proxy
```

The server runs on `localhost:8765` by default.

## API

### POST /v1/chat/completions

Accepts OpenAI chat completion requests:

```json
{
  "model": "claude-3-sonnet",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "stream": true
}
```

Returns streaming SSE responses in OpenAI format.

### GET /v1/models

Lists available models (returns Claude Code's default model).

## Integration with BAML

In your `baml_src/clients.baml`:

```baml
client ClaudeCodeProxy {
  provider openai
  options {
    base_url "http://localhost:8765/v1"
    model "claude-3-sonnet"
  }
}
```

## Limitations

- Only supports streaming responses
- Does not support function calling (tool use)
- Runs `claude -p` subprocess for each request
- Single-threaded request handling

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CLAUDE_PROXY_PORT` | `8765` | Port to listen on |
| `CLAUDE_PROXY_HOST` | `127.0.0.1` | Host to bind to |
```

#### 4.2 Add header doc to source

**File:** `src/bin/claude-proxy.rs` (line 1)

```rust
//! # claude-proxy
//!
//! OpenAI-compatible HTTP proxy wrapping `claude -p`.
//!
//! This binary provides an OpenAI API facade over Claude Code CLI,
//! allowing tools like BAML to use Claude Code as a backend.
//!
//! ## Usage
//!
//! ```bash
//! claude-proxy  # Starts server on localhost:8765
//! ```
//!
//! ## API Endpoints
//!
//! - `POST /v1/chat/completions` - Chat completion (streaming)
//! - `GET /v1/models` - List models
//!
//! See `docs/CLAUDE_PROXY.md` for full documentation.
```

### Success Criteria

- [x] **Automated:** `cargo doc` generates docs without warnings
- [ ] **Manual:** Documentation clearly explains purpose and usage

---

## Phase 5: Create Minimal Iced GUI

**Goal:** Build a new desktop GUI using Iced that wraps v2's simple architecture.

### Architecture

```
descartes-gui (new)
       │
       ├── RalphExecutor (direct Rust call)
       ├── AgentRegistry (for status display)
       ├── SCUD storage (for waves/tasks)
       └── Harness output capture (for live stream)
```

No daemon, no ZMQ, no gRPC - just direct library calls.

### Reusable Components from v1

From `git show fbc532f:archive/descartes-v1/gui/`:

| Component | Reuse Strategy |
|-----------|----------------|
| `ViewMode` enum pattern | Copy - clean state machine |
| Theme constants | Copy - consistent styling |
| `SwarmMonitorState` | Adapt - remove RPC, use AgentRegistry |
| Animation system | Optional - nice-to-have |

### New Crate Structure

```
descartes/
├── descartes/           # Existing CLI + lib
└── descartes-gui/       # NEW
    ├── Cargo.toml
    └── src/
        ├── main.rs      # Iced app entry
        ├── theme.rs     # Colors, fonts (from v1)
        ├── views/
        │   ├── mod.rs
        │   ├── waves.rs     # Task waves display
        │   ├── agents.rs    # Agent status panel
        │   └── output.rs    # Live output stream
        └── state.rs     # App state, messages
```

### Cargo.toml

```toml
[package]
name = "descartes-gui"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "descartes-gui"
path = "src/main.rs"

[dependencies]
# GUI
iced = { version = "0.13", features = ["tokio", "advanced"] }

# Descartes core
descartes = { path = "../descartes" }

# Async
tokio = { version = "1", features = ["full", "sync"] }

# Utilities
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Minimal main.rs

```rust
use iced::{Application, Command, Element, Settings, Theme};
use descartes::{RalphExecutor, scud};
use tokio::sync::mpsc;

fn main() -> iced::Result {
    DescartesGui::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    // Navigation
    SwitchView(ViewMode),

    // Task management
    LoadWaves(String),  // tag
    WavesLoaded(Result<Vec<Vec<scud::Task>>, String>),

    // Agent management
    StartAgent(String),  // task_id
    AgentOutput(String),
    AgentComplete(Result<(), String>),

    // Control
    PauseAgent,
    ResumeAgent,
    CancelAgent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Waves,
    Agents,
    Output,
}

struct DescartesGui {
    view: ViewMode,
    waves: Vec<Vec<scud::Task>>,
    output_buffer: String,
    agent_running: bool,
    // Channel to send control messages
    control_tx: Option<mpsc::Sender<descartes::interactive::AgentControl>>,
}

impl Application for DescartesGui {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                view: ViewMode::Waves,
                waves: Vec::new(),
                output_buffer: String::new(),
                agent_running: false,
                control_tx: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Descartes".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchView(view) => {
                self.view = view;
                Command::none()
            }
            Message::LoadWaves(tag) => {
                Command::perform(
                    async move {
                        // Load from SCUD storage
                        let storage = scud::Storage::open(".").map_err(|e| e.to_string())?;
                        let waves = storage.compute_waves(&tag).map_err(|e| e.to_string())?;
                        Ok(waves)
                    },
                    Message::WavesLoaded,
                )
            }
            Message::WavesLoaded(result) => {
                match result {
                    Ok(waves) => self.waves = waves,
                    Err(e) => eprintln!("Failed to load waves: {}", e),
                }
                Command::none()
            }
            // ... other message handlers
            _ => Command::none()
        }
    }

    fn view(&self) -> Element<Message> {
        // Build UI based on current view
        match self.view {
            ViewMode::Waves => self.view_waves(),
            ViewMode::Agents => self.view_agents(),
            ViewMode::Output => self.view_output(),
        }
    }
}

impl DescartesGui {
    fn view_waves(&self) -> Element<Message> {
        // Render task waves
        todo!()
    }

    fn view_agents(&self) -> Element<Message> {
        // Render agent status
        todo!()
    }

    fn view_output(&self) -> Element<Message> {
        // Render live output
        todo!()
    }
}
```

### Success Criteria

- [x] **Automated:** `cargo build -p descartes-gui` succeeds
- [ ] **Manual:** GUI launches and displays task waves
- [ ] **Manual:** Can start an agent and see live output
- [ ] **Manual:** Pause/Resume/Cancel buttons work

---

## Execution Order

Phases can be executed in parallel, but recommended order:

1. **Phase 1** (5 min) - Quick cleanup
2. **Phase 2** (1-2 hours) - Agent control wiring
3. **Phase 3** (1-2 hours) - OpenCode harness
4. **Phase 4** (30 min) - Documentation
5. **Phase 5** (4-8 hours) - GUI implementation

Phases 2, 3, and 4 are independent and can run in parallel.

---

## Open Questions

None - all requirements are clear from the research phase.

---

## References

- Research document: `docs/CODEBASE_RESEARCH_GUI_STATUS.md`
- Archived v1 GUI: `git show fbc532f:archive/descartes-v1/gui/`
- SCUD TUI reference: `/Users/reuben/projects/harnesses/scud/scud-cli/src/commands/spawn/tui/`
