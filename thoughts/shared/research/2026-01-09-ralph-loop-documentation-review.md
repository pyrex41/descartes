---
date: 2026-01-09T13:22:16-06:00
researcher: Claude
git_commit: 1fa21e220c9a839bec4f6ce510b80fe4c8a07de9
branch: master
repository: cap
topic: "Ralph Loop Feature Documentation Review"
tags: [research, ralph-wiggum, descartes, documentation, scud-loop]
status: complete
last_updated: 2026-01-09
last_updated_by: Claude
---

# Research: Ralph Loop Feature Documentation Review

**Date**: 2026-01-09T13:22:16-06:00
**Researcher**: Claude
**Git Commit**: 1fa21e220c9a839bec4f6ce510b80fe4c8a07de9
**Branch**: master
**Repository**: cap

## Research Question

Review the codebase and blog documentation for the new Ralph loop feature. Document how the feature works, assess the documentation quality, and ensure it's navigable for new developers.

## Summary

The Ralph loop feature is fully implemented with comprehensive documentation across three main locations:

1. **Blog documentation** (`descartes/docs/blog/`) - Two articles covering base loops and tuning
2. **Slash commands** (`.claude/commands/ralph-wiggum/`) - Three commands for Claude Code integration
3. **Core implementation** (`descartes/core/src/scud_loop.rs`) - Full tuning system with state persistence

The documentation effectively explains the feature but has some navigation gaps and could benefit from a consolidated quickstart guide.

## Detailed Findings

### Blog Documentation Structure

#### README.md (Navigation Index)
**Location**: `descartes/docs/blog/README.md`

The README provides a well-organized index with:
- Quick Links table for task-based navigation
- Numbered blog series with descriptions
- Core Concepts reference (4 tools, tool levels, flow phases)
- Architecture diagram

**Current Ralph loop coverage** in Quick Links:
- Row 12: "Run iterative loops" → [Iterative Loops](12-iterative-loops.md)
- **Gap**: No mention of the "Tune the Guitar" article (13-tune-the-guitar.md)

#### 12-iterative-loops.md (Base Loop Documentation)
**Location**: `descartes/docs/blog/12-iterative-loops.md` (620 lines)

Comprehensive coverage of:
- Quick Start with command examples
- CLI Commands (start, status, resume, cancel)
- Completion detection (tagged `<promise>` format)
- Exit conditions table
- Backend configuration (Claude, OpenCode, Generic)
- Git integration (auto-commit, branch templates)
- SCUD integration section with wave-based execution
- Slash command examples
- GUI visualization mockups
- Configuration reference tables

**Gap**: Tuning flags (`--tune`, `--no-tune`, `--max-tune-attempts`) are documented in the file but the connection to article 13 is not explicitly made in navigation.

#### 13-tune-the-guitar.md (Tuning Documentation)
**Location**: `descartes/docs/blog/13-tune-the-guitar.md` (127 lines)

Covers:
- How tuning works (5-step flow)
- Configuration flags
- Human review commands (`descartes loop tune`)
- Tuning flow diagram (ASCII art)
- What gets captured (prompt, output, verification, diff, refinement)
- Tuner agent prompt structure
- Philosophy quote from Geoffrey Huntley
- See Also links to articles 12 and 07

**Structure is good** but could use:
- More concrete examples with actual error messages
- Troubleshooting section for common issues

### Slash Commands

#### ralph-loop.md
**Location**: `.claude/commands/ralph-wiggum/ralph-loop.md`

Documents:
- Arguments: SCUD tag + optional flags
- Execution steps (4 numbered steps)
- Example usage pattern
- Tuning options section with human intervention flow
- Output format with emoji status indicators

#### cancel-ralph.md
**Location**: `.claude/commands/ralph-wiggum/cancel-ralph.md`

Documents:
- Execution steps
- Output format with final status

#### help.md
**Location**: `.claude/commands/ralph-wiggum/help.md`

Documents:
- What is Ralph (key principles)
- SCUD integration explanation
- Available commands summary
- External links to original technique

### Core Implementation

#### Data Structures (`scud_loop.rs:289-381`)

| Struct | Purpose |
|--------|---------|
| `TaskAttempt` | Single execution attempt with prompt, output, verification, diff |
| `TaskTuneState` | Human checkpoint state with all attempts and selection |
| `TuneConfig` | Configuration for tuning behavior |
| `TaskExecutionResult` | Enum with `Success`, `Blocked`, `Unknown`, `AwaitingTune` |

#### Key Methods (`scud_loop.rs`)

| Method | Lines | Purpose |
|--------|-------|---------|
| `execute_task_with_tuning()` | 1119-1231 | Main tuning loop with retry logic |
| `build_tuner_prompt()` | 1238-1296 | Constructs analysis prompt for tuner agent |
| `parse_tuner_output()` | 1299-1307 | Extracts refinement from tuner response |
| `spawn_tuner_agent()` | 1310-1318 | Executes tuner agent |
| `save_tune_state()` | 1367-1380 | Persists tune state to JSON |
| `load_tune_state()` | 457-478 | Loads tune state on resume |

#### CLI Implementation (`loop_cmd.rs`)

| Subcommand | Lines | Purpose |
|------------|-------|---------|
| `start` | 148-272 | Start new loop (SCUD or generic mode) |
| `tune` | 372-529 | Review variants, select, or edit |
| `resume` | 274-309 | Resume from saved state |
| `status` | 311-348 | Show current loop status |
| `cancel` | 350-370 | Cancel running loop |

### Documentation Navigation Assessment

#### Current Navigation Paths

```
README.md
    ├── Quick Links → 12-iterative-loops.md
    │                     └── "See Also" → 13-tune-the-guitar.md (implicit)
    │
    └── Blog Series → 12-iterative-loops.md
                          └── No next link to 13

13-tune-the-guitar.md
    └── See Also → 12-iterative-loops.md
                 → 07-flow-workflow.md
```

#### Navigation Gaps Identified

1. **README.md missing entry**: Article 13 is not listed in Quick Links or Blog Series
2. **No "Next Steps" in article 12**: Article 12 ends with links to 07, 10, 11 but not 13
3. **Slash commands not linked**: Blog articles don't mention the slash commands exist in `.claude/commands/`

### File Locations Summary

| Component | Location |
|-----------|----------|
| Blog index | `descartes/docs/blog/README.md` |
| Iterative loops docs | `descartes/docs/blog/12-iterative-loops.md` |
| Tune the guitar docs | `descartes/docs/blog/13-tune-the-guitar.md` |
| Slash commands | `.claude/commands/ralph-wiggum/` |
| Core implementation | `descartes/core/src/scud_loop.rs` |
| CLI implementation | `descartes/cli/src/commands/loop_cmd.rs` |
| Loop state file | `.scud/loop-state.json` (runtime) |
| Tune state file | `.scud/tune-state.json` (runtime) |
| Task files | `.scud/tasks/{tag}.json` (runtime) |

## Code References

### Data Structures
- `descartes/core/src/scud_loop.rs:289-317` - TaskAttempt struct
- `descartes/core/src/scud_loop.rs:320-337` - TaskTuneState struct
- `descartes/core/src/scud_loop.rs:341-381` - TuneConfig struct
- `descartes/core/src/scud_loop.rs:188-195` - TaskExecutionResult enum

### Core Tuning Flow
- `descartes/core/src/scud_loop.rs:1119-1231` - execute_task_with_tuning method
- `descartes/core/src/scud_loop.rs:1238-1296` - build_tuner_prompt method
- `descartes/core/src/scud_loop.rs:1310-1318` - spawn_tuner_agent method

### CLI Commands
- `descartes/cli/src/commands/loop_cmd.rs:27-92` - LoopStartArgs with tuning flags
- `descartes/cli/src/commands/loop_cmd.rs:115-136` - LoopTuneArgs
- `descartes/cli/src/commands/loop_cmd.rs:372-529` - handle_tune function

### Slash Commands
- `.claude/commands/ralph-wiggum/ralph-loop.md` - Start loop command
- `.claude/commands/ralph-wiggum/cancel-ralph.md` - Cancel command
- `.claude/commands/ralph-wiggum/help.md` - Help/overview command

## Architecture Documentation

### Tuning Flow

```
Task Execution
      │
      ▼
┌─────────────────────────────────────┐
│  Attempt 1: Execute with base spec  │
│  ├─ Success → Done ✓                │
│  └─ Failed → Capture context        │
│                                     │
│  Tuner Agent: "What went wrong?"    │
│  └─ Suggests refinement             │
│                                     │
│  Attempt 2: Execute with refinement │
│  ├─ Success → Done ✓                │
│  └─ Failed → Capture, refine again  │
│                                     │
│  ... up to max_tune_attempts ...    │
│                                     │
│  All Failed → Human Checkpoint      │
│  └─ descartes loop tune             │
│  └─ --select N or --edit            │
│  └─ descartes loop resume           │
└─────────────────────────────────────┘
```

### State Management

```
.scud/
├── tasks/{tag}.json        # Task definitions with status
├── loop-state.json         # Loop execution state
└── tune-state.json         # Human tuning checkpoint
```

### Wave-Based Execution

```
Wave 1: [Task A]           # No dependencies
         ↓ commit
Wave 2: [Task B, Task C]   # Both depend on A
         ↓ commit
Wave 3: [Task D]           # Depends on B and C
         ↓ commit
Loop Complete
```

## Related Research

- Original Ralph Wiggum technique: https://ghuntley.com/ralph/
- Implementation plan: `thoughts/shared/plans/2026-01-09-ralph-tune-the-guitar.md`

## Open Questions

1. Should article 12 be split into "Basic Loops" and "SCUD Loops" for clarity?
2. Should a consolidated "Ralph Loop Quickstart" guide be created?
3. Should the slash commands be documented in the blog articles?
