---
date: 2025-12-29T21:39:07-06:00
researcher: Claude
git_commit: fb9b5c431505d633e421000d78cf0da18ea04dff
branch: master
repository: descartes
topic: "Blog Documentation Gap Analysis for New Features"
tags: [research, documentation, blog, keyboard, flow, loop, scud]
status: complete
last_updated: 2025-12-29
last_updated_by: Claude
---

# Research: Blog Documentation Gap Analysis for New Features

**Date**: 2025-12-29T21:39:07-06:00
**Researcher**: Claude
**Git Commit**: fb9b5c431505d633e421000d78cf0da18ea04dff
**Branch**: master
**Repository**: descartes

## Research Question

What blog posts should be edited and/or added to document the new features (hotkeys, flow, iterative loops, etc.)?

## Summary

The current blog documentation has significant gaps compared to features that exist in the codebase. Three major areas require documentation updates:

1. **Keyboard/Hotkeys** - The blog documents ~15 shortcuts, but the GUI has 50+ keyboard bindings including comprehensive vim-like navigation
2. **Iterative Loop System** - Completely undocumented (`descartes loop` commands, Ralph-style execution)
3. **SCUD Loop Integration** - Not documented (wave-based execution, task tracking)

## Detailed Findings

### Current Blog Structure

The blog series exists at `docs/blog/` with 11 articles + README:

| File | Topic | Status |
|------|-------|--------|
| 01-introduction-the-pi-philosophy.md | Core philosophy | Up to date |
| 02-getting-started.md | Installation/setup | Up to date |
| 03-cli-commands.md | CLI reference | **Needs update** (missing loop commands) |
| 04-providers-configuration.md | Provider config | Up to date |
| 05-session-management.md | Sessions | Up to date |
| 06-agent-types.md | Agent types/tool levels | Up to date |
| 07-flow-workflow.md | Flow workflow | **Needs update** (missing SCUD integration details) |
| 08-skills-system.md | Skills | Up to date |
| 09-gui-features.md | GUI features | **Major update needed** (keyboard shortcuts) |
| 10-subagent-tracking.md | Sub-agent tracking | Up to date |
| 11-advanced-features.md | Advanced features | **Needs update** (missing loop system) |

### Gap 1: Keyboard Shortcuts (Critical Gap)

**Current Documentation** (`09-gui-features.md:462-476`):

Only documents 7 global shortcuts:
- Ctrl+1-5 for view switching
- Ctrl+R for refresh
- Ctrl+Q for quit

Plus DAG editor shortcuts in a small table.

**What Actually Exists** (`gui/src/main.rs:581-879`, `dag_canvas_interactions.rs:497-574`):

**Global Shortcuts:**
- `1-6` - Direct view switching (no Ctrl required)
- `Tab/Shift+Tab` - Focus navigation between inputs
- `Escape/q` - Multi-level cancel (dismiss dialogs, clear errors)
- `r/F5` - View-specific refresh

**Vim-Like Navigation:**
- `j/k` - Up/down in lists (sessions, time travel)
- `h/l` - Left/right in timeline (time travel debugger)
- `g` - Jump to start/first item
- `G` (Shift+g) - Jump to end/last item
- `i/a` - Insert/append mode (focus chat input)
- `/` - Search/filter (sessions view)
- `o` - Open/create new item

**Chat View:**
- `i` - Focus input (vim insert mode)
- `a` - Focus input (vim append mode)
- `Ctrl+L` - Clear conversation

**Sessions View:**
- `j/k` or `Up/Down` - Navigate sessions
- `Enter` - Activate selected session
- `g` - Jump to first session
- `G` - Jump to last session
- `o` or `Ctrl+N` - New session dialog
- `/` - Focus filter input

**Time Travel Debugger:**
- `h/Left` - Previous event
- `l/Right` - Next event
- `g/Home` - Jump to start
- `G/End` - Jump to end
- `Space` - Toggle playback
- `+/-` - Zoom in/out
- `L` (Shift+l) - Toggle loop

**DAG Editor:**
- `Space+Drag` - Pan canvas
- `Delete` - Delete selected nodes
- `Ctrl+A` - Select all
- `Ctrl+Z` - Undo
- `Ctrl+Shift+Z` or `Ctrl+Y` - Redo
- `Escape` - Cancel operation
- Mouse wheel - Zoom to cursor

**Recommendation:** Create new section in `09-gui-features.md` titled "Comprehensive Keyboard Reference" with tables organized by view.

### Gap 2: Iterative Loop System (Missing Documentation)

**What Exists:**
- `core/src/iterative_loop.rs` (1,312 lines)
- `cli/src/commands/loop_cmd.rs` (256 lines)
- `gui/src/loop_state.rs` (195 lines)
- `gui/src/loop_view.rs` (368 lines)

**Features Not Documented:**

1. **CLI Commands:**
   ```bash
   descartes loop start --command claude --prompt "Task..." --max-iterations 20
   descartes loop status
   descartes loop resume --state-file .descartes/loop-state.json
   descartes loop cancel
   ```

2. **Completion Detection:**
   - Detects `<promise>TEXT</promise>` tags in output
   - Configurable completion promise text
   - Falls back to exit code 0 if no promise configured

3. **State Persistence:**
   - Saves to `.descartes/loop-state.json`
   - Full resume capability after interruption
   - Tracks iteration count, output history, timestamps

4. **Backend Support:**
   - `claude` backend with stream-json output
   - `opencode` backend with json format
   - `generic` backend for any CLI tool

5. **Git Integration:**
   - Optional auto-commit after each iteration
   - Configurable commit message template
   - Optional branch creation

6. **Iteration Context:**
   - Adds iteration number to prompts
   - Reminds agent of completion promise format
   - Tracks max iterations

**Recommendation:** Add new blog post `12-iterative-loops.md` or add major section to `11-advanced-features.md`.

### Gap 3: SCUD Loop Integration (Missing Documentation)

**What Exists:**
- `core/src/scud_loop.rs` (934 lines)
- Integration with SCUD task management

**Features Not Documented:**

1. **Wave-Based Execution:**
   - Tasks organized into dependency waves
   - Executes all tasks in wave before advancing
   - Auto-commits after each wave completion

2. **Task Status Tracking:**
   - Statuses: pending, in-progress, done, blocked
   - Automatic status updates during execution
   - Blocked task tracking with reasons

3. **Verification System:**
   - Runs verification command after each task
   - Default: `make check test`
   - Only marks "done" if verification passes

4. **SCUD Stats Integration:**
   - Parses `scud stats` output for progress
   - Completion: when pending=0 and in_progress=0

5. **Wave Commits:**
   - Commit message: `feat({tag}): complete wave {N}`
   - Tracks which tasks completed in each commit
   - Records commit hashes in state

6. **GUI Visualization:**
   - Shows wave progress (current/total)
   - Displays tasks in current wave with status icons
   - Shows wave commit history

**Recommendation:** Add section to `07-flow-workflow.md` about SCUD loop integration, or create separate `12-iterative-loops.md` covering both generic and SCUD loops.

### Gap 4: Flow Agent Definitions (Minor Gap)

**What Exists:**
- `agents/flow-orchestrator.md`
- `agents/flow-ingest.md`
- `agents/flow-review-graph.md`
- `agents/flow-plan-tasks.md`
- `agents/flow-implement.md`
- `agents/flow-qa.md`
- `agents/flow-summarize.md`

The blog (`07-flow-workflow.md`) mentions agents but doesn't explain:
- How to customize agent behavior
- Agent file format and structure
- Tool level requirements for each phase

**Recommendation:** Add "Customizing Flow Agents" section to `07-flow-workflow.md`.

## Recommended Actions

### Priority 1: Update `09-gui-features.md`

Add comprehensive keyboard shortcuts section (~200 lines):

```markdown
## Comprehensive Keyboard Reference

### Philosophy: Vim-Style Navigation

Descartes GUI embraces vim-style navigation patterns...

### Global Shortcuts (All Views)
| Key | Action |
|-----|--------|
| 1-6 | Switch to view (Sessions/Dashboard/Chat/Agents/Debugger/DAG) |
| Tab | Focus next input |
| Shift+Tab | Focus previous input |
| Escape | Cancel/dismiss/clear |
| r / F5 | Refresh current view |

### Chat View
[table of shortcuts]

### Sessions View
[table with vim-like j/k/g/G navigation]

### Time Travel Debugger
[table with h/l/g/G timeline navigation]

### DAG Editor
[existing table plus Space+drag for pan]
```

### Priority 2: Create New Blog Post or Section

Either:
- **Option A:** New `12-iterative-loops.md` covering both generic and SCUD loops
- **Option B:** Add "Iterative Loops" section to `11-advanced-features.md`

Content should cover:
- `descartes loop` CLI commands
- Configuration options
- State persistence and resume
- Promise-based completion detection
- Git integration
- SCUD wave-based execution
- GUI visualization

### Priority 3: Update `03-cli-commands.md`

Add section for loop commands:

```markdown
## Loop Commands

### loop start
Start an iterative loop that runs a command repeatedly.

### loop status
Show current loop state and progress.

### loop resume
Resume an interrupted loop from saved state.

### loop cancel
Cancel a running loop gracefully.
```

### Priority 4: Update `07-flow-workflow.md`

Add section on SCUD integration:
- How Flow uses SCUD for task management
- Wave computation and execution
- Task status transitions
- Customizing flow agents

## Code References

### Keyboard Handling
- `gui/src/main.rs:581-879` - Main keyboard handler
- `gui/src/main.rs:1346-1403` - Event subscription
- `gui/src/dag_canvas_interactions.rs:497-574` - DAG keyboard handling

### Iterative Loop
- `core/src/iterative_loop.rs:317-351` - Loop initialization
- `core/src/iterative_loop.rs:390-500` - Main execution loop
- `cli/src/commands/loop_cmd.rs:79-255` - CLI commands

### SCUD Loop
- `core/src/scud_loop.rs:259-301` - SCUD loop construction
- `core/src/scud_loop.rs:549-677` - SCUD-aware execution
- `core/src/scud_loop.rs:451-501` - Wave commits

### GUI Loop Components
- `gui/src/loop_state.rs:26-82` - Loop view state
- `gui/src/loop_view.rs:10-358` - Loop view rendering

## Architecture Documentation

The documentation gap stems from features being added in two main PRs:
1. PR #9: Added keyboard UX improvements (vim bindings, Tab navigation)
2. PR #10: Added iterative loop system and SCUD integration

The blog was written before these features were merged, hence the gaps.

## Open Questions

1. Should iterative loops be a separate blog post or part of advanced features?
2. Should keyboard shortcuts have their own dedicated post given the volume?
3. Should flow agent customization be documented separately?

---

*Research completed 2025-12-29*
