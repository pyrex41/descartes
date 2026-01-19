# Descartes Refactoring Plan: SCUD Integration & GUI Focus

**Date**: 2026-01-19
**Status**: Planning
**Goal**: Transform Descartes from a full orchestration platform into a focused GUI + spec-building layer on top of SCUD

---

## Executive Summary

SCUD CLI (v1.40.1) now provides complete swarm orchestration:
- Wave computation (Kahn's algorithm for DAG)
- Task execution with fresh context per task
- TUI monitoring with tmux/terminal integration
- Backpressure validation between rounds
- Multiple harness support (claude, opencode)

**Descartes should become**:
1. **Iced GUI** for visualizing and controlling SCUD execution
2. **Spec builder** for constructing rich task prompts
3. **Thin CLI wrapper** that delegates to SCUD

**Code reduction**: ~2,500 lines removed (19% of codebase)

---

## Current State Analysis

### What SCUD Provides (Don't Reimplement)

| Feature | SCUD Command | Notes |
|---------|--------------|-------|
| Task storage | `.scud/tasks/*.scg` | SCG compact format |
| Wave computation | `scud waves` | Topological sort |
| Task execution | `scud swarm` | Fresh context per task |
| Agent spawning | `scud spawn` | Visible terminal windows |
| Progress monitoring | `scud monitor` | TUI with attach/validate |
| Backpressure | `scud validate` | Configurable commands |
| PRD parsing | `scud parse-prd` | AI-powered task extraction |
| Task expansion | `scud expand` | Break down complex tasks |

### What Descartes Adds (Keep)

| Feature | Location | Purpose |
|---------|----------|---------|
| Spec building | `spec.rs` | Rich prompt construction from plans |
| Guidance integration | `.scud/guidance/` | Context injection for agents |
| Harness abstraction | `harness/` | Backend adapters (claude-code, opencode, codex) |
| Agent definitions | `agent/` | Named agent templates |
| Interactive session | `interactive/` | CLI session with commands |
| Transcript system | `transcript/` | Execution recording |
| **Iced GUI** | `descartes-gui/` | Visual orchestration interface |

---

## Phase 1: Code Removal (~2,500 lines)

### Files to DELETE

| File | Lines | Reason |
|------|-------|--------|
| `swarm_executor.rs` | 1,340 | SCUD handles orchestration |
| `swarm_tui.rs` | 520 | SCUD provides TUI |
| `context_handoff.rs` | 472 | Only used by deprecated executor |
| `handoff/mod.rs` | 357 | Appears unused (verify first) |

### Code to REMOVE from kept files

**main.rs** (~50 lines):
- Remove `--no-use-scud` flag and fallback path (lines 587-606)
- Remove `SwarmExecutor` instantiation
- Keep SCUD delegation path only

**lib.rs** (~10 lines):
- Remove `SwarmExecutor` re-export
- Remove `SwarmTui` re-export

**agent/registry.rs** (~50 lines):
- Remove TUI-specific integration
- Keep core registry for interactive mode

### Verification Steps

```bash
# 1. Ensure SCUD delegation works
descartes swarm --scud-tag test --plan docs/plan.md

# 2. Remove files
rm descartes/src/swarm_executor.rs
rm descartes/src/swarm_tui.rs
rm descartes/src/context_handoff.rs
rm -rf descartes/src/handoff/

# 3. Update lib.rs
# Remove: pub mod swarm_executor; pub mod swarm_tui; pub mod context_handoff;

# 4. Update main.rs
# Remove SwarmExecutor fallback, keep only SCUD delegation

# 5. Build and test
cargo build --release
cargo test
```

---

## Phase 2: GUI Architecture Redesign

### Current GUI Issues

1. **Agent spawning is TODO** - `main.rs:127` just adds stub text
2. **`scud::list_tasks()` doesn't exist** - referenced but never implemented
3. **Control channel always None** - pause/resume are no-ops
4. **No connection to SCUD** - pure UI mockup

### New GUI Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Iced Application                         │
├─────────────────────────────────────────────────────────────┤
│  ViewMode: Waves | Agents | Output | Settings | SCUD        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │ SCUD Bridge │────▶│ Event Bus   │────▶│ View State  │   │
│  │ (subprocess)│     │ (channels)  │     │ (AppState)  │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
│         │                   │                   │           │
│         ▼                   ▼                   ▼           │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │ scud swarm  │     │ Message::*  │     │ view_*()    │   │
│  │ scud spawn  │     │ dispatch    │     │ render      │   │
│  │ scud list   │     │             │     │             │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### SCUD Bridge Module

Create `descartes-gui/src/scud_bridge.rs`:

```rust
use std::process::{Command, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// Events from SCUD execution
pub enum ScudEvent {
    TasksLoaded(Vec<TaskInfo>),
    WavesComputed(Vec<Vec<String>>),
    SwarmStarted { tag: String },
    TaskStarted { task_id: String },
    TaskCompleted { task_id: String, success: bool },
    ValidationResult { passed: bool, output: String },
    SwarmCompleted { success: bool },
    Output(String),
    Error(String),
}

/// Commands to SCUD
pub enum ScudCommand {
    LoadTasks { tag: Option<String> },
    ComputeWaves { tag: String },
    StartSwarm { tag: String, harness: String, round_size: usize },
    StopSwarm,
}

pub struct ScudBridge {
    event_tx: mpsc::Sender<ScudEvent>,
    command_rx: mpsc::Receiver<ScudCommand>,
}

impl ScudBridge {
    pub async fn run(mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                ScudCommand::LoadTasks { tag } => {
                    self.load_tasks(tag).await;
                }
                ScudCommand::StartSwarm { tag, harness, round_size } => {
                    self.run_swarm(tag, harness, round_size).await;
                }
                // ... other commands
            }
        }
    }

    async fn load_tasks(&self, tag: Option<String>) {
        // Call: scud list --json [--tag <tag>]
        let output = Command::new("scud")
            .args(&["list", "--json"])
            .output();

        match output {
            Ok(out) => {
                let tasks: Vec<TaskInfo> = serde_json::from_slice(&out.stdout).unwrap_or_default();
                let _ = self.event_tx.send(ScudEvent::TasksLoaded(tasks)).await;
            }
            Err(e) => {
                let _ = self.event_tx.send(ScudEvent::Error(e.to_string())).await;
            }
        }
    }

    async fn run_swarm(&self, tag: String, harness: String, round_size: usize) {
        // Spawn: scud swarm --tag <tag> --harness <harness> --json-events
        // Stream stdout for JSON event lines
        let mut child = Command::new("scud")
            .args(&[
                "swarm",
                "--tag", &tag,
                "--harness", &harness,
                "--round-size", &round_size.to_string(),
                "--json-events",  // Hypothetical flag for event streaming
            ])
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn scud");

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<SwarmEvent>(&line) {
                // Convert to ScudEvent and send
                let _ = self.event_tx.send(event.into()).await;
            }
        }
    }
}
```

### Updated Message Enum

```rust
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SwitchView(ViewMode),

    // SCUD Bridge Events (from ScudBridge)
    ScudEvent(ScudEvent),

    // User Actions
    LoadTasks,
    StartSwarm { tag: String },
    StopSwarm,
    RefreshWaves,

    // Task Actions
    StartTask(String),      // task_id
    MarkTaskDone(String),
    MarkTaskBlocked(String),

    // UI
    DismissError,
    Tick,
}
```

### View Implementations

**Waves View** - Shows task DAG organized by execution waves:
```rust
fn view_waves(&self) -> Element<Message> {
    let waves_content = self.state.waves.iter().enumerate().map(|(wave_idx, wave)| {
        let wave_header = text(format!("Wave {}", wave_idx + 1))
            .size(16)
            .style(theme::text::HEADER);

        let tasks: Vec<_> = wave.iter().map(|task| {
            row![
                text(&task.id).width(60),
                text(&task.title).width(Length::Fill),
                status_badge(&task.status),
                button("Start").on_press(Message::StartTask(task.id.clone())),
            ]
            .spacing(10)
            .into()
        }).collect();

        column![wave_header, Column::with_children(tasks).spacing(5)]
            .spacing(10)
            .into()
    }).collect();

    scrollable(Column::with_children(waves_content).spacing(20))
        .into()
}
```

**Output View** - Real-time streaming from SCUD:
```rust
fn view_output(&self) -> Element<Message> {
    let output_text = text(&self.state.output_buffer)
        .font(Font::MONOSPACE)
        .size(12);

    let status_bar = row![
        text(format!("Wave {}/{}", self.state.current_wave, self.state.total_waves)),
        text(format!("Tasks: {}/{}", self.state.completed_tasks, self.state.total_tasks)),
        if self.state.swarm_running {
            button("Stop").on_press(Message::StopSwarm)
        } else {
            button("Start Swarm").on_press(Message::StartSwarm {
                tag: self.state.current_tag.clone()
            })
        }
    ].spacing(20);

    column![status_bar, scrollable(output_text)]
        .spacing(10)
        .into()
}
```

---

## Phase 3: SCUD CLI Enhancements Needed

For the GUI to work optimally, SCUD should support:

### 1. JSON Output Mode

```bash
# List tasks as JSON
scud list --json --tag feature

# Output:
[
  {"id": "1", "title": "Implement auth", "status": "pending", "deps": []},
  {"id": "2", "title": "Add tests", "status": "pending", "deps": ["1"]}
]
```

### 2. Event Streaming Mode

```bash
# Stream events during swarm execution
scud swarm --tag feature --json-events

# Output (one JSON object per line):
{"event": "swarm_started", "tag": "feature", "total_waves": 3}
{"event": "wave_started", "wave": 0, "tasks": ["1", "2"]}
{"event": "task_started", "task_id": "1"}
{"event": "task_output", "task_id": "1", "text": "Starting..."}
{"event": "task_completed", "task_id": "1", "success": true}
{"event": "validation_started"}
{"event": "validation_completed", "passed": true}
{"event": "wave_completed", "wave": 0}
{"event": "swarm_completed", "success": true}
```

### 3. ZMQ PUB/SUB (Future)

For real-time streaming without polling, SCUD could expose a ZMQ PUB socket:

```rust
// SCUD side (in swarm command)
let publisher = zmq::Context::new().socket(zmq::PUB)?;
publisher.bind("tcp://*:5555")?;

// On each event:
publisher.send(&serde_json::to_vec(&event)?, 0)?;

// GUI side
let subscriber = zmq::Context::new().socket(zmq::SUB)?;
subscriber.connect("tcp://localhost:5555")?;
subscriber.set_subscribe(b"")?;  // Subscribe to all

loop {
    let msg = subscriber.recv_bytes(0)?;
    let event: ScudEvent = serde_json::from_slice(&msg)?;
    // Handle event
}
```

This is documented in the `backbone.md` PRD but not yet implemented.

---

## Phase 4: Spec Building Enhancement

The spec builder (`spec.rs`) is Descartes's unique value. Enhance it:

### Current Flow
```
Plan.md + Task → build_task_spec() → Write to .scud/guidance/
```

### Enhanced Flow
```
Plan.md + Task + Context → build_rich_spec() → .scud/guidance/descartes-spec.md
                                             → .scud/guidance/task-{id}-context.md
```

### New Spec Features

1. **Codebase Context Injection**
   - Auto-include relevant file snippets based on task description
   - Use semantic search (if available) or keyword matching

2. **Dependency Context**
   - Include outputs/transcripts from completed dependency tasks
   - "Task 1 completed with: [summary]"

3. **Verification Commands**
   - Include explicit verification steps in spec
   - "Run `cargo test` after implementation"

4. **Template System**
   - Per-project spec templates
   - Customizable prompt structure

---

## Phase 5: Final Architecture

### Directory Structure (After Cleanup)

```
descartes/
├── descartes/                    # CLI library (trimmed)
│   ├── src/
│   │   ├── main.rs              # CLI entry (SCUD delegation only)
│   │   ├── lib.rs               # Library exports
│   │   ├── config.rs            # Configuration (keep)
│   │   ├── spec.rs              # Spec building (keep, enhance)
│   │   ├── scud/
│   │   │   └── mod.rs           # SCUD wrappers (keep)
│   │   ├── harness/             # Agent backends (keep)
│   │   │   ├── mod.rs
│   │   │   ├── claude_code.rs
│   │   │   ├── opencode.rs
│   │   │   └── codex.rs
│   │   ├── agent/               # Agent definitions (keep)
│   │   │   ├── mod.rs
│   │   │   ├── definition.rs
│   │   │   ├── category.rs
│   │   │   └── tools.rs
│   │   ├── interactive/         # CLI session (keep)
│   │   │   ├── mod.rs
│   │   │   ├── session.rs
│   │   │   ├── commands.rs
│   │   │   └── skills.rs
│   │   └── transcript/          # Recording (keep)
│   │       ├── mod.rs
│   │       └── scg.rs
│   └── Cargo.toml
│
├── descartes-gui/                # Iced GUI (expand)
│   ├── src/
│   │   ├── main.rs              # Iced application entry
│   │   ├── state.rs             # AppState struct
│   │   ├── theme.rs             # Visual styling
│   │   ├── scud_bridge.rs       # NEW: SCUD subprocess communication
│   │   └── views/
│   │       ├── mod.rs
│   │       ├── waves.rs         # Wave visualization
│   │       ├── agents.rs        # Agent status
│   │       ├── output.rs        # Streaming output
│   │       ├── settings.rs      # Configuration UI
│   │       └── scud.rs          # Task management
│   └── Cargo.toml
│
├── .scud/                        # SCUD workspace
│   ├── tasks/                    # Task files
│   ├── guidance/                 # Descartes-generated specs
│   ├── transcripts/              # Execution records
│   └── config.toml               # SCUD configuration
│
└── docs/                         # Documentation
```

### Line Count Comparison

| Component | Before | After | Change |
|-----------|--------|-------|--------|
| descartes CLI | 12,983 | ~10,500 | -2,500 (-19%) |
| descartes-gui | 1,050 | ~2,500 | +1,450 (for real functionality) |
| **Total** | 14,033 | ~13,000 | -1,000 (-7%) |

The net reduction is smaller because we're adding real GUI functionality, but the complexity reduction is significant - no more duplicated orchestration logic.

---

## Implementation Checklist

### Phase 1: Code Removal
- [ ] Verify `handoff/mod.rs` is unused
- [ ] Remove `swarm_executor.rs`
- [ ] Remove `swarm_tui.rs`
- [ ] Remove `context_handoff.rs`
- [ ] Remove `handoff/` directory
- [ ] Update `lib.rs` exports
- [ ] Update `main.rs` to remove fallback path
- [ ] Run full test suite
- [ ] Update documentation

### Phase 2: GUI SCUD Bridge
- [ ] Create `scud_bridge.rs` module
- [ ] Implement `ScudEvent` enum
- [ ] Implement `ScudCommand` enum
- [ ] Add JSON parsing for SCUD output
- [ ] Connect bridge to Iced subscription
- [ ] Update `Message` enum
- [ ] Test with `scud list --json`

### Phase 3: GUI Views
- [ ] Fix `view_waves()` to use ScudBridge
- [ ] Implement real task loading
- [ ] Add swarm start/stop controls
- [ ] Implement output streaming
- [ ] Add settings view
- [ ] Add SCUD task management view

### Phase 4: SCUD Enhancements (Coordinate with SCUD repo)
- [ ] Add `--json` flag to `scud list`
- [ ] Add `--json-events` flag to `scud swarm`
- [ ] Document event format
- [ ] (Future) Add ZMQ PUB socket option

### Phase 5: Spec Building
- [ ] Add codebase context injection
- [ ] Add dependency context
- [ ] Add template system
- [ ] Update guidance writer

---

## Migration Path

### For Existing Users

1. **Update SCUD**: `cargo install scud-cli` (get v1.40+)
2. **Update Descartes**: Pull latest, rebuild
3. **No workflow changes**: `descartes swarm` still works (delegates to SCUD)
4. **New GUI**: `descartes-gui` for visual interface

### Backwards Compatibility

- Keep `descartes swarm` command (thin wrapper to `scud swarm`)
- Keep `descartes next/complete/waves` commands (wrappers)
- Remove only internal implementation, not CLI interface

---

## Open Questions

1. **SCUD JSON Output**: Does SCUD already support `--json` flags? Need to verify.

2. **ZMQ Timeline**: Should we wait for SCUD ZMQ support or use subprocess + JSON events now?

3. **Harness Location**: Should harnesses move to SCUD or stay in Descartes?
   - Pro SCUD: Single binary, simpler deployment
   - Pro Descartes: GUI can use harnesses directly for interactive mode

4. **GUI Distribution**: Ship as separate binary or integrated with CLI?
   - Separate: Smaller CLI binary, optional GUI
   - Integrated: Single install, feature flag

---

## References

- SCUD CLI: https://crates.io/crates/scud-cli
- Iced GUI Framework: https://iced.rs/
- ZMQ Backbone PRD: `.scud/docs/prd/backbone.md`
- GUI Implementation Plan: `thoughts/shared/plans/2026-01-15-gui-full-feature-implementation.md`
- Architecture Analysis: `thoughts/DESCARTES_ARCHITECTURE_ANALYSIS.md`
