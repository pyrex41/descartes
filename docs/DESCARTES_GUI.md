# Descartes GUI

Desktop interface for AI agent orchestration using the Ralph Wiggum loop pattern.

## Overview

The Descartes GUI provides visibility and control over AI agent execution, wrapping the v2 architecture with an Iced-based desktop application. It adapts the Ralph loop pattern for interactive use, giving users real-time control over agent behavior.

## Architecture

### Ralph Loop Adaptation for GUI

The original Ralph Wiggum loop operates in two modes:
- **Plan Mode**: Orchestrator decides what to do next
- **Build Mode**: Subagents execute specific tasks

The GUI adapts this by:

1. **Decoupling execution from UI thread**: Agent execution runs in background tasks via Tokio, with control channels for pause/resume/cancel
2. **Streaming output**: Agent responses stream to the GUI in real-time via messages
3. **Wave visualization**: SCUD task dependencies are computed and displayed as parallel execution waves
4. **Interactive control**: Users can intervene at any point during execution

```
┌─────────────────────────────────────────────────────────────┐
│                      Descartes GUI                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                     │
│  │  Waves  │  │ Agents  │  │ Output  │  ← View Tabs        │
│  └─────────┘  └─────────┘  └─────────┘                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Wave 1:  [Task A] [Start]  [Task B] [Start]               │
│  Wave 2:  [Task C] [Start]  (depends on A, B)              │
│  Wave 3:  [Task D] [Start]  (depends on C)                 │
│                                                             │
│  [Refresh]                                                  │
├─────────────────────────────────────────────────────────────┤
│  Status: Running │ Current: Task A                         │
│  [Pause] [Cancel]                                          │
└─────────────────────────────────────────────────────────────┘
```

## Features

### 1. Wave Visualization (Waves View)

Displays tasks organized by parallel execution waves, computed from SCUD's dependency DAG.

- **Wave grouping**: Tasks with satisfied dependencies are grouped into waves
- **Status display**: Shows task ID, title, and current status (Pending, InProgress, Done)
- **One-click start**: Start button on each task to begin agent execution
- **Refresh**: Reload waves from SCUD storage

### 2. Agent Control (Agents View)

Real-time control over running agents with immediate feedback.

| Control | Description |
|---------|-------------|
| **Pause** | Suspends agent execution, buffering incoming responses |
| **Resume** | Continues paused agent from where it left off |
| **Cancel** | Terminates agent execution, marks task for retry |

The control system uses Tokio channels:
```rust
enum AgentControl {
    Pause,
    Resume,
    Cancel,
    Interrupt { reason: String },
}
```

Control messages are processed via `tokio::select!` in the agent loop, allowing immediate response even during streaming.

### 3. Live Output (Output View)

Streaming display of agent output with automatic scrolling.

- **Real-time streaming**: Output appears as the agent generates it
- **Completion markers**: Clear indication when agent completes or errors
- **Scrollable history**: Review full agent output session

### 4. Error Handling

Dismissible error banners with context:
- SCUD connection errors
- Agent execution failures
- Configuration issues

## Planned Features

### Model Configuration

> **Status**: Planned

Per-agent-type model selection:
- **Orchestrator model**: For Plan mode decisions
- **Subagent model**: For Build mode execution
- **Research model**: For information gathering tasks

```
┌─────────────────────────────────────────┐
│ Model Configuration                     │
├─────────────────────────────────────────┤
│ Orchestrator: [claude-opus-4 ▼]         │
│ Subagent:     [claude-sonnet-4 ▼]       │
│ Research:     [perplexity-sonar ▼]      │
│                                         │
│ [Apply] [Reset to Defaults]             │
└─────────────────────────────────────────┘
```

### Context Supplementation

> **Status**: Planned

Ability to inject additional context during execution:
- **Pre-task context**: Add files, documentation, or instructions before task starts
- **In-flight context**: Supplement running agent with additional information
- **Guidance files**: Load project-specific guidance from `.scud/guidance/`

```
┌─────────────────────────────────────────┐
│ Add Context                             │
├─────────────────────────────────────────┤
│ [Drop files here or click to browse]    │
│                                         │
│ Loaded:                                 │
│ ├── API_DOCS.md                         │
│ ├── ARCHITECTURE.md                     │
│ └── schema.sql                          │
│                                         │
│ [Clear All] [Apply to Current Agent]    │
└─────────────────────────────────────────┘
```

### In-Flight Monitoring

> **Status**: Planned

Real-time visibility into agent decision-making:
- **Tool calls**: Display tool invocations with arguments and results
- **Subagent spawning**: Show when orchestrator delegates to subagents
- **Token usage**: Track context consumption and warn before overflow
- **Backpressure signals**: Surface validation failures and blocked states

```
┌─────────────────────────────────────────────────────────────┐
│ Agent Monitor                                               │
├─────────────────────────────────────────────────────────────┤
│ ● Running task-1: "Setup environment"                       │
│                                                             │
│ 14:32:01 [Tool] Read("src/main.rs")                        │
│ 14:32:02 [Tool] Bash("cargo build")                        │
│ 14:32:05 [Subagent] Spawning "fix-compilation-errors"      │
│ 14:32:10 [Tool] Edit("src/lib.rs", lines 45-52)            │
│                                                             │
│ Tokens: 12,450 / 200,000 (6.2%)  ████░░░░░░░░░░░░░░░░      │
└─────────────────────────────────────────────────────────────┘
```

### Agent Registry Dashboard

> **Status**: Planned

Overview of all agent activity across the system:
- **Active agents**: List of currently running agents with status
- **Historical runs**: Past agent executions with outcomes
- **Resource usage**: CPU, memory, API call statistics

## Testing

The GUI includes comprehensive headless testing using `iced_test`:

### Test Categories

| Category | Count | Description |
|----------|-------|-------------|
| Model tests | 8 | Direct state manipulation tests |
| UI interaction | 8 | Simulator-based click tests |
| Full workflow | 3 | End-to-end scenario tests |
| Snapshot | 2 | Visual regression tests |

### Running Tests

```bash
cd descartes-gui
cargo test
```

### Headless Testing Pattern

```rust
use iced_test::simulator;

#[test]
fn test_workflow() {
    let mut app = test_app();

    // Simulate user clicking "Start"
    let mut ui = simulator(app.view());
    let _ = ui.click("Start");

    // Process generated messages
    for msg in ui.into_messages() {
        let _ = app.update(msg);
    }

    // Assert state changed
    assert_eq!(app.state.agent_status, AgentStatus::Running);
}
```

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | API key for Claude models |
| `XAI_API_KEY` | API key for xAI models (SCUD) |
| `RUST_LOG` | Logging level (e.g., `descartes_gui=debug`) |

### SCUD Integration

The GUI reads from SCUD storage at `.scud/tasks/tasks.scg`:

```bash
# Ensure SCUD is configured
scud warmup

# View available tasks
scud list

# Start GUI
cargo run --bin descartes-gui
```

## Building

### Dependencies

- Rust 1.75+
- System dependencies for Iced (varies by platform)

### macOS

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cd descartes-gui
cargo build --release
```

### Linux

```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt install libxkbcommon-dev libwayland-dev

# Build
cargo build --release
```

## File Structure

```
descartes-gui/
├── Cargo.toml           # Dependencies including iced 0.14
├── src/
│   ├── main.rs          # Application entry, Elm architecture
│   ├── state.rs         # AppState, AgentStatus, TaskInfo
│   ├── theme.rs         # Color constants (dark theme)
│   └── views/
│       ├── mod.rs       # View module exports
│       ├── waves.rs     # Wave visualization (placeholder)
│       ├── agents.rs    # Agent control (placeholder)
│       └── output.rs    # Output display (placeholder)
└── target/              # Build artifacts (gitignored)
```

## Roadmap

1. **v0.1** (Current): Basic wave display, agent start/pause/cancel, output streaming
2. **v0.2**: Model configuration UI, per-task model selection
3. **v0.3**: Context supplementation, file drop support
4. **v0.4**: In-flight monitoring, tool call visualization
5. **v0.5**: Agent registry dashboard, historical runs
6. **v1.0**: Full Ralph loop integration with RalphExecutor
