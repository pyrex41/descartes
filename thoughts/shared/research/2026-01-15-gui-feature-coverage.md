---
date: 2026-01-15T23:15:20Z
researcher: Claude
git_commit: 88826e40ef9896f7661a47902c361ec7c6a0a405
branch: master
repository: pyrex41/descartes
topic: "GUI Feature Coverage Analysis"
tags: [research, codebase, gui, feature-coverage, iced]
status: complete
last_updated: 2026-01-15
last_updated_by: Claude
---

# Research: GUI Feature Coverage Analysis

**Date**: 2026-01-15T23:15:20Z
**Researcher**: Claude
**Git Commit**: 88826e40ef9896f7661a47902c361ec7c6a0a405
**Branch**: master
**Repository**: pyrex41/descartes

## Research Question

Does the Descartes GUI provide a good interface for all the features documented in GitHub Pages (guidance system, backpressure configuration, skills, interactive mode, etc.)?

## Summary

The GUI provides a **minimal execution monitor** for swarm orchestration but does **not expose** the majority of features documented in GitHub Pages. The GUI focuses solely on:

1. Viewing task waves from SCUD
2. Starting individual agents on tasks
3. Pause/Resume/Cancel controls
4. Live output streaming

All configuration features (guidance, backpressure, harnesses, models) and workflow features (skills, interactive commands, transcripts) are **not accessible** through the GUI.

## Detailed Findings

### What the GUI Provides

#### 1. Three View Modes

**Location**: `descartes-gui/src/main.rs:47-52`

| View | Description |
|------|-------------|
| **Waves** | Displays tasks organized by parallel execution waves |
| **Agents** | Shows current agent status and control buttons |
| **Output** | Displays live streaming output from running agent |

#### 2. Wave Visualization

**Location**: `descartes-gui/src/main.rs:286-318`

The Waves view shows:
- Wave groupings computed from SCUD dependency DAG
- Task ID, title, and status for each task
- "Start" button to launch an agent for a specific task
- "Refresh" button to reload tasks from SCUD storage

```
Wave 1
├── TASK-001  Setup environment     Pending  [Start]
├── TASK-002  Create schema         Pending  [Start]

Wave 2
├── TASK-003  Implement API         Pending  [Start]
```

#### 3. Agent Controls

**Location**: `descartes-gui/src/main.rs:321-352`

The Agents view provides:
- Current agent status (Idle, Running, Paused)
- Current task ID being worked on
- Context-sensitive control buttons:
  - When Running: [Pause] [Cancel]
  - When Paused: [Resume] [Cancel]
  - When Idle: (no controls)

#### 4. Live Output

**Location**: `descartes-gui/src/main.rs:355-374`

The Output view displays:
- Scrollable output buffer
- Real-time streaming from running agent
- Error messages and completion status

#### 5. Error Handling

**Location**: `descartes-gui/src/main.rs:207-224`

- Error banner appears at top of screen
- Shows error message with [Dismiss] button
- Dismissable by clicking the button

---

### What the GUI Does NOT Provide

#### 1. Configuration UI - NOT IMPLEMENTED

The GUI has **no settings or preferences screen**. Users cannot:

| Feature | GUI Status | How to Configure |
|---------|-----------|------------------|
| Guidance (global/builder/review/validator) | Not in GUI | Edit `.descartes/config.toml` |
| Backpressure commands | Not in GUI | Edit `.descartes/config.toml` |
| Harness selection (claude-code, opencode, codex) | Not in GUI | Edit config or use CLI `--harness` |
| Model selection | Not in GUI | Edit config or use CLI `--model` |
| SCUD tag selection | Not in GUI | Must use CLI `scud tags` |
| Round size | Not in GUI | Use CLI `--round-size` |
| Validation toggle | Not in GUI | Use CLI `--verify` or `--no-validate` |

**Code Evidence**: `descartes-gui/src/main.rs:388` loads config but provides no UI to modify it:
```rust
let config = Config::load(None).map_err(|e| e.to_string())?;
```

#### 2. Skills System - NOT IMPLEMENTED

The GUI does **not expose the skills system**:

- No skills browser or selector
- No way to run `/skill research` or `/skill create_plan`
- No way to view or edit skill definitions
- No skill variable input

**Workaround**: Use CLI `descartes interactive` then `/skill <name>`

#### 3. Interactive Commands - NOT IMPLEMENTED

The documented interactive commands do not exist in the GUI:

| Command | CLI Support | GUI Support |
|---------|------------|-------------|
| `/help` | Yes | No |
| `/pause` | Yes | Partial (button only) |
| `/resume` | Yes | Partial (button only) |
| `/cancel` | Yes | Partial (button only) |
| `/scud` | Yes | No |
| `/waves` | Yes | Waves view (visual only) |
| `/diff` | Yes | No |
| `/context` | Yes | No |
| `/skill` | Yes | No |

#### 4. Transcript Browser - NOT IMPLEMENTED

- No way to browse historical transcripts
- No transcript search or filtering
- No transcript viewer beyond current session output

**Workaround**: Use CLI `descartes transcripts --today` and `descartes show <id>`

#### 5. SCUD Management - NOT IMPLEMENTED

The GUI cannot:
- Switch SCUD tags
- View task details
- Edit task status
- Add/remove dependencies
- Run `scud generate` or `scud expand`

**Workaround**: Use CLI `scud` commands directly

#### 6. Validation Results - NOT DISPLAYED

The GUI does not show:
- Backpressure validation results (pass/fail)
- Which commands were run
- Which tasks failed validation

**Note**: The TUI (`descartes swarm`) does show validation results.

---

### Implementation Status

**Location**: `descartes-gui/src/main.rs:127`

The GUI has a TODO indicating agent spawning is not complete:
```rust
Message::StartAgent(task_id) => {
    // ...
    // TODO: Actually spawn the agent via RalphExecutor
```

This means clicking "Start" currently:
1. Sets status to Running
2. Adds a message to output buffer
3. Does NOT actually spawn an agent

---

### GUI vs TUI Comparison

| Feature | GUI (descartes-gui) | TUI (descartes swarm) |
|---------|--------------------|-----------------------|
| **Wave visualization** | Yes (Waves view) | Yes (progress bar) |
| **Agent status** | Yes (Agents view) | Yes (agent list) |
| **Live output** | Yes (Output view) | No (attach required) |
| **Pause/Resume/Cancel** | Yes (buttons) | No |
| **Validation results** | No | Yes (PASS/FAIL display) |
| **Keyboard shortcuts** | No | Yes ([1-9], [v], [q]) |
| **Multi-agent display** | No (single agent) | Yes (numbered list) |
| **Auto-execution** | No (manual start) | Yes (wave orchestration) |

---

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ descartes-gui (Iced 0.14)                                   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Views (no configuration UI)                          │   │
│  │  ├── Waves    → Task list with Start buttons        │   │
│  │  ├── Agents   → Status + Pause/Resume/Cancel        │   │
│  │  └── Output   → Live output buffer                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Data Layer (read-only)                               │   │
│  │  ├── Config::load() - loads config, no edit UI      │   │
│  │  ├── scud::list_tasks() - reads task list           │   │
│  │  └── scud::waves() - computes wave structure        │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

Features NOT in GUI:
❌ Guidance configuration
❌ Backpressure configuration
❌ Harness/model selection
❌ Skills system
❌ Interactive commands
❌ Transcript browser
❌ SCUD tag management
❌ Validation display
```

---

## Code References

### GUI Implementation
- `descartes-gui/src/main.rs:1-932` - Main application with all views
- `descartes-gui/src/state.rs` - AppState, AgentStatus, TaskInfo structs
- `descartes-gui/src/theme.rs` - Dark theme color constants
- `descartes-gui/src/views/` - View module stubs (logic in main.rs)

### Backend Configuration (not exposed in GUI)
- `descartes/src/config.rs:596-612` - GuidanceConfig struct
- `descartes/src/config.rs:296-312` - HarnessConfig struct
- `descartes/src/config.rs:444-446` - CategoryConfig with backpressure

### Skills System (not in GUI)
- `descartes/src/interactive/skills.rs:14-38` - Skill struct
- `descartes/src/interactive/skills.rs:200-315` - Built-in skills
- `descartes/src/interactive/session.rs:396-431` - Skill execution

### TUI (has more features than GUI)
- `descartes/src/swarm_tui.rs` - Terminal UI with validation display

---

## Feature Coverage Summary

| GitHub Pages Feature | GUI Coverage |
|---------------------|--------------|
| Wave-based execution | Partial (view only, no auto-execution) |
| Fresh context per task | Not visible |
| Backpressure validation | Not visible |
| Guidance system | Not configurable |
| Skills system | Not accessible |
| Interactive commands | Not available |
| Agent categories | Not selectable |
| Harness selection | Not available |
| Model selection | Not available |
| Transcript management | Not available |
| SCUD integration | Read-only (list/waves) |

---

## Related Research

- `thoughts/shared/research/2026-01-15-review-guidance-planning-workflows.md` - Documents the features that should be in the GUI

---

## Open Questions

1. Is the GUI intended to be feature-complete or a minimal monitor?
2. Should the GUI add configuration screens for guidance/backpressure?
3. Should the GUI integrate the skills system?
4. Is the StartAgent TODO blocking real usage of the GUI?
