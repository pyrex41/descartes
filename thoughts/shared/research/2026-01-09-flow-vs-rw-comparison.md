---
date: 2026-01-09T19:40:00+00:00
researcher: Claude Code (Opus 4.5)
git_commit: 1fa21e220c9a839bec4f6ce510b80fe4c8a07de9
branch: master
repository: cap
topic: "Flow vs RW (Ralph Wiggum) Commands Comparison"
tags: [research, codebase, flow, ralph-wiggum, iterative-loops, scud, comparison]
status: complete
last_updated: 2026-01-09
last_updated_by: Claude Code
---

# Research: Flow vs RW (Ralph Wiggum) Commands Comparison

**Date**: 2026-01-09T19:40:00+00:00
**Researcher**: Claude Code (Opus 4.5)
**Git Commit**: 1fa21e220c9a839bec4f6ce510b80fe4c8a07de9
**Branch**: master
**Repository**: cap

## Research Question

Comparison of the Flow commands and RW (Ralph Wiggum) commands to understand their architectural differences, use cases, and tradeoffs for a blog post.

## Summary

**Flow** and **RW (Ralph Wiggum)** are two complementary approaches to autonomous AI-driven development in the Descartes ecosystem. Both leverage SCUD task management for wave-based execution, but they differ fundamentally in how they manage context across sessions:

| Aspect | Flow Commands | RW (Ralph Wiggum) Commands |
|--------|---------------|---------------------------|
| **Philosophy** | Multi-session with handoffs | Single-session iteration |
| **Context Strategy** | Fresh context per session via handoff documents | Fresh context per iteration via fixed spec |
| **Session Model** | Research → Plan → Implement (separate sessions) | Start loop → iterate until done |
| **Origin** | Descartes workflow design | Geoffrey Huntley's Ralph pattern |
| **Best For** | Large features, team collaboration, resumability | Focused tasks, autonomous completion |

Both systems share the same underlying infrastructure: SCUD task management, Descartes loop executors, and wave-based execution with auto-commit.

---

## Detailed Findings

### 1. The Flow System

#### 1.1 Core Philosophy

Flow treats development as a **pipeline of distinct phases**, each in its own Claude session:

```
┌──────────┐   ┌──────────┐   ┌──────────┐
│ RESEARCH │──►│   PLAN   │──►│IMPLEMENT │
│ (Session)│   │(Session) │   │(Session) │
└────┬─────┘   └────┬─────┘   └────┬─────┘
     │              │              │
     ▼              ▼              ▼
 [Handoff]     [Handoff]     [Handoff]
```

**Key Insight**: Each session starts fresh with focused context loaded from the previous session's handoff document.

#### 1.2 Available Commands

| Command | Purpose | File Location |
|---------|---------|---------------|
| `/flow:research` | Conduct research with handoff generation | `.claude/commands/flow/research.md` |
| `/flow:plan` | Create plan and SCUD tasks from research | `.claude/commands/flow/plan.md` |
| `/flow:implement` | Execute SCUD tasks wave-by-wave | `.claude/commands/flow/implement.md` |
| `/flow:resume` | Resume from any handoff document | `.claude/commands/flow/resume.md` |
| `/flow:status` | Show all active flows and handoffs | `.claude/commands/flow/status.md` |

#### 1.3 The Handoff System

Handoffs are the key differentiator. They enable context transfer between sessions:

**Storage Location**: `thoughts/shared/handoffs/{phase}/{YYYY-MM-DD}_{HH-MM}_{topic}.md`

**Handoff Frontmatter Fields**:
```yaml
type: handoff
phase: research|plan|implement
timestamp: {ISO timestamp}
topic: "{feature name}"
git_commit: "{commit hash}"
branch: "{branch name}"
next_phase: plan|implement|complete
next_command: "/flow:{next-phase} {path-or-tag}"
```

**Phase-Specific Content**:

- **Research Handoff**: Key findings summary, critical files list, patterns discovered, recommended planning approach
- **Plan Handoff**: SCUD tag, wave breakdown, architecture decisions, testing strategy, files to modify
- **Implement Handoff**: Completion status, commits made, learnings, manual testing checklist

#### 1.4 Implementation Loop (flow:implement)

When `/flow:implement` runs, it uses a wave-based loop with SCUD tracking:

```
LOOP START
│
├─► Check Completion (scud stats --tag)
│   └─► All done? → Exit with success
│
├─► Get Current Wave (scud waves, scud next)
│   └─► No tasks? → Commit wave, advance
│
├─► For Each Pending Task in Wave:
│   ├─► Claim Task (scud set-status in-progress)
│   ├─► Gather Context (read task, find plan, use pattern-finder)
│   ├─► Implement (write code, follow patterns, add tests)
│   ├─► Verify (cargo check && cargo test, up to 3 attempts)
│   └─► Complete (scud set-status done)
│
├─► Wave Complete → git commit
└─► Continue to Next Wave
```

**Reference**: `.claude/commands/flow/implement.md:50-90`

---

### 2. The RW (Ralph Wiggum) System

#### 2.1 Core Philosophy

RW is based on Geoffrey Huntley's "Ralph Wiggum" pattern - an iterative loop that feeds the same spec repeatedly until completion:

```bash
# The original Ralph pattern
while :; do cat PROMPT.md | claude-code; done
```

**Key Insight**: The agent sees its previous work in files and git history. Each iteration has fresh context but identical spec.

#### 2.2 Available Commands

| Command | Purpose | File Location |
|---------|---------|---------------|
| `/rw:loop` | Start Ralph loop for SCUD tag | `.claude/commands/rw/loop.md` |
| `/rw:cancel-ralph` | Stop active loop, preserve state | `.claude/commands/rw/cancel-ralph.md` |
| `/rw:help` | Explain the technique | `.claude/commands/rw/help.md` |

#### 2.3 Geoff-Style Key Principles

From the research conversation with Geoffrey Huntley:

1. **Fixed Spec Allocation**: ~5,000 tokens dedicated to core specs that persist across iterations
2. **Fresh Context Per Goal**: Reset context each iteration, no cumulative history
3. **External Orchestration**: Shell script or Rust executor controls the loop, not the model
4. **Deterministic Failures**: Repeated failures enable systematic debugging
5. **No Promise Tags**: Completion detected via task state, not model output signals

#### 2.4 SCUD Integration

RW uses SCUD tasks as the "fixed spec":

| SCUD Element | Maps to Geoff's Spec Concept |
|--------------|------------------------------|
| Task title + description | Core objective |
| Implementation details from plan | Detailed spec |
| Test strategy | Success criteria |
| Dependencies | Prerequisite context |
| Wave membership | Scope boundary |

#### 2.5 Implementation Loop

The RW loop via `descartes loop start --scud-tag`:

```
Loop Start
│
├─► Load SCUD Stats (pending, done, blocked counts)
├─► Load Waves Structure
│
├─► For Each Task (until all done):
│   │
│   ├─► Build Task Spec
│   │   ├─► SCUD task details (title, description, test strategy)
│   │   ├─► Relevant plan section
│   │   └─► Additional spec files (--spec-file)
│   │
│   ├─► Execute Task (spawn sub-agent with fresh context)
│   │   └─► Agent receives: spec + verification command
│   │
│   ├─► Verify (run verification command)
│   │   └─► Success? → Mark done
│   │   └─► Fail? → Auto-tune or mark blocked
│   │
│   └─► Wave Complete? → git commit, advance wave
│
└─► Exit: All done OR all blocked
```

**Reference**: `descartes/core/src/scud_loop.rs:754-918`

#### 2.6 "Tune the Guitar" Feature

When tasks fail, RW has automatic prompt refinement:

1. **Capture failure context**: Output, errors, git diff
2. **Spawn tuner agent**: Analyzes failure and suggests refinement
3. **Retry with refined prompt**: Up to `max_tune_attempts` (default: 3)
4. **Human checkpoint**: If still failing, pause for review

**Tuner workflow**:
```
Task fails verification
    │
    ├─► Capture: output, stderr, git diff
    ├─► Spawn tuner agent with failure context
    ├─► Tuner outputs: "REFINEMENT: {suggestion}"
    ├─► Append refinement to spec
    └─► Retry task
```

**CLI commands for human review**:
```bash
# View all attempts
descartes loop tune --show-variants

# Select a variant
descartes loop tune --select 2

# Edit prompt manually
descartes loop tune --edit

# Resume loop
descartes loop resume
```

**Reference**: `descartes/core/src/scud_loop.rs:1119-1231`, `descartes/docs/blog/13-tune-the-guitar.md`

---

### 3. Shared Infrastructure: SCUD

Both Flow and RW rely on SCUD for task management:

#### 3.1 SCUD Directory Structure

```
.scud/
├── tasks/
│   ├── tasks.scg          # SCG format (all phases)
│   ├── tasks.json         # JSON format
│   └── {tag}.json         # Per-phase files
├── workflow-state.json    # Current phase tracking
├── active-tag             # Current tag name
├── loop-state.json        # RW loop state
└── tune-state.json        # Tuning state (if failed)
```

#### 3.2 Task Status Flow

```
pending → in-progress → done
                     ↓
                  blocked
```

**Status meanings**:
- `pending` (P): Ready to work, dependencies met
- `in-progress` (I): Currently being executed
- `done` (D): Completed and verified
- `blocked` (B): Failed, needs intervention

#### 3.3 Wave-Based Execution

Tasks organized into waves based on dependencies:
- **Wave 1**: No dependencies (can run first)
- **Wave 2**: Depends on Wave 1 tasks
- **Wave N**: Depends on Wave N-1 tasks

Both Flow and RW process waves sequentially, committing after each wave.

#### 3.4 Completion Detection

| System | Completion Method |
|--------|-------------------|
| Flow | `scud stats --tag` shows `pending == 0 && in_progress == 0` |
| RW | Same SCUD-based detection, no promise tags |

---

### 4. Architectural Comparison

#### 4.1 Context Management

| Aspect | Flow | RW |
|--------|------|-----|
| **Session scope** | One phase per session | One loop per session |
| **Context transfer** | Handoff documents | N/A (single session) |
| **Per-iteration context** | Cumulative within phase | Fresh spec each iteration |
| **Context size** | Full session context | Fixed ~5k token spec |

#### 4.2 Loop Execution

| Aspect | Flow | RW |
|--------|------|-----|
| **Loop location** | Inline in Claude Code | Descartes CLI (`descartes loop`) |
| **Task execution** | Claude Code native | Sub-agent spawn |
| **Verification** | Manual per task | Automatic with command |
| **Auto-tuning** | No | Yes ("tune the guitar") |

#### 4.3 State Persistence

| State Type | Flow Location | RW Location |
|------------|---------------|-------------|
| Phase progress | `thoughts/shared/handoffs/` | `.scud/loop-state.json` |
| Task status | `.scud/tasks/{tag}.json` | `.scud/tasks/{tag}.json` |
| Tuning state | N/A | `.scud/tune-state.json` |

#### 4.4 When to Use Each

**Use Flow when:**
- Feature spans multiple sessions (large scope)
- Team collaboration (handoffs provide context for others)
- Need clear phase boundaries for review/approval
- Want to pause and resume across days/weeks
- Planning phase needs human approval before implementation

**Use RW when:**
- Task is well-defined and bounded
- Want autonomous completion without intervention
- Iterative improvement on failures is beneficial
- Single-session completion is feasible
- Need "tune the guitar" automatic prompt refinement

---

### 5. Technical Deep Dive

#### 5.1 Flow Implementation Stack

```
/.claude/commands/flow/*.md     ← Slash command definitions
         │
         ▼
Claude Code native execution     ← No external loop
         │
         ▼
SCUD CLI commands               ← Task state management
         │
         ▼
thoughts/shared/handoffs/       ← Phase transitions
```

**Key files**:
- `.claude/commands/flow/implement.md` (306 lines) - Implementation loop specification
- `.claude/commands/flow/plan.md` (221 lines) - Planning with SCUD task generation
- `.claude/commands/flow/research.md` (157 lines) - Research with handoff generation

#### 5.2 RW Implementation Stack

```
/.claude/commands/rw/*.md       ← Slash command definitions
         │
         ▼
descartes loop CLI              ← External loop orchestration
         │
         ▼
ScudIterativeLoop (Rust)        ← Loop executor
         │
         ├─► Spec building      ← Task + plan + custom files
         ├─► Sub-agent spawn    ← Fresh Claude per task
         ├─► Verification       ← Run test command
         └─► Auto-tuning        ← Prompt refinement on failure
         │
         ▼
.scud/loop-state.json           ← State persistence
```

**Key files**:
- `descartes/core/src/scud_loop.rs` (1917 lines) - SCUD loop with tuning
- `descartes/core/src/iterative_loop.rs` (1315 lines) - Base iterative loop
- `descartes/cli/src/commands/loop_cmd.rs` (530 lines) - CLI interface

#### 5.3 Backend Support

Both systems ultimately use Claude, but differently:

**Flow**: Claude Code runs directly, receives slash command, executes inline.

**RW**: Descartes spawns Claude as subprocess:
```rust
// scud_loop.rs:1057-1064
let mut cmd = Command::new("claude");
cmd.args(["-p", "--output-format", "text"])
   .arg(prompt)
   .current_dir(&self.config.working_directory)
```

RW also supports other backends:
- `LoopClaudeBackend` - Claude Code CLI
- `LoopOpenCodeBackend` - OpenCode CLI
- `LoopGenericBackend` - Any CLI command

---

### 6. Example Workflows

#### 6.1 Flow Workflow Example

```bash
# Session 1: Research
/flow:research "How should we implement user authentication?"
# → Creates: thoughts/shared/research/2026-01-09-auth-system.md
# → Creates: thoughts/shared/handoffs/research/2026-01-09_14-30_auth-system.md

# Session 2: Planning (new session, fresh context)
/flow:plan thoughts/shared/handoffs/research/2026-01-09_14-30_auth-system.md
# → Creates: thoughts/shared/plans/2026-01-09-auth-system.md
# → Creates: SCUD tag "auth-system" with tasks
# → Creates: thoughts/shared/handoffs/plan/2026-01-09_15-45_auth-system.md

# Session 3: Implementation (new session, fresh context)
/flow:implement auth-system
# → Executes waves, commits after each
# → Creates: thoughts/shared/handoffs/implement/2026-01-09_18-00_auth-system.md

# Later: Resume any phase
/flow:resume thoughts/shared/handoffs/plan/2026-01-09_15-45_auth-system.md
```

#### 6.2 RW Workflow Example

```bash
# Start loop (single session)
/rw:loop auth-system --plan thoughts/shared/plans/auth-system.md

# Loop runs autonomously:
# - Builds spec from SCUD task + plan
# - Spawns sub-agent per task
# - Verifies with "cargo check && cargo test"
# - Auto-tunes on failure
# - Commits after each wave

# If human intervention needed:
descartes loop tune --show-variants
descartes loop tune --select 2
descartes loop resume

# Cancel if needed:
/rw:cancel-ralph
```

---

## Code References

| Component | File | Lines |
|-----------|------|-------|
| Flow research command | `.claude/commands/flow/research.md` | 1-157 |
| Flow plan command | `.claude/commands/flow/plan.md` | 1-221 |
| Flow implement command | `.claude/commands/flow/implement.md` | 1-306 |
| Flow resume command | `.claude/commands/flow/resume.md` | 1-218 |
| Flow status command | `.claude/commands/flow/status.md` | 1-145 |
| RW loop command | `.claude/commands/rw/loop.md` | 1-79 |
| RW cancel command | `.claude/commands/rw/cancel-ralph.md` | 1-28 |
| RW help command | `.claude/commands/rw/help.md` | 1-55 |
| ScudIterativeLoop | `descartes/core/src/scud_loop.rs` | 1-1917 |
| IterativeLoop | `descartes/core/src/iterative_loop.rs` | 1-1315 |
| Loop CLI | `descartes/cli/src/commands/loop_cmd.rs` | 1-530 |
| SCUD storage | `descartes/core/src/scg_task_storage.rs` | 1-559 |
| Flow workflow docs | `descartes/docs/blog/07-flow-workflow.md` | 1-667 |
| Loop docs | `descartes/docs/blog/12-iterative-loops.md` | - |
| Tune docs | `descartes/docs/blog/13-tune-the-guitar.md` | - |

---

## Architecture Documentation

### Pattern Summary

**Flow Pattern**: Phase-Based Pipeline
- Separates concerns into distinct phases
- Uses handoffs for context transfer
- Enables team collaboration and review gates
- Best for large, complex features

**RW Pattern**: Iterative Refinement Loop
- Continuous execution until completion
- Auto-tuning on failures
- External orchestration via Descartes
- Best for bounded, autonomous tasks

### Shared Infrastructure

Both systems leverage:
1. **SCUD Task Management** - Wave-based task organization
2. **Git Integration** - Auto-commit after waves
3. **Verification Commands** - Automated testing
4. **Plan Documents** - Implementation guidance

---

## Related Research

- `thoughts/shared/research/2026-01-08-ralph-loop-scud-integration.md` - Ralph loop SCUD integration details
- `thoughts/shared/plans/2026-01-08-geoff-style-ralph-scud-loop.md` - RW implementation plan
- `descartes/thoughts/shared/research/2025-12-28-iterative-agent-loop-ralph-style.md` - Original Ralph research
- `thoughts/shared/plans/2025-12-29-integrated-flow-system.md` - Flow system specification

---

## Open Questions

1. **Hybrid approach?** Could a workflow use Flow for research/planning then RW for implementation?

2. **Team RW?** Could multiple developers share an RW loop via the tune-state files?

3. **Handoffs in RW?** Should RW generate handoffs for partial completion scenarios?

4. **Flow auto-tuning?** Should Flow's implement phase have tune-the-guitar capability?
