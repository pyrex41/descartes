# Choosing Your Workflow: Flow vs RW

*Two philosophies for autonomous development, one shared foundation*

---

Descartes offers two distinct approaches to autonomous AI-driven development: **Flow** and **RW (Ralph Wiggum)**. Both leverage SCUD for wave-based task execution, but they differ fundamentally in how they manage context and session boundaries.

This post assumes familiarity with [Flow](07-flow-workflow.md), [Iterative Loops](12-iterative-loops.md), and [Tune the Guitar](13-tune-the-guitar.md). Here we'll compare philosophies, help you choose the right approach, and explore hybrid possibilities.

## The Two Paths

| Aspect | Flow | RW (Ralph Wiggum) |
|--------|------|-------------------|
| **Model** | Multi-session pipeline | Single-session iteration |
| **Context** | Handoff documents between sessions | Fixed spec fed each iteration |
| **Origin** | Descartes workflow design | Geoffrey Huntley's Ralph pattern |
| **Best for** | Large features, team collaboration | Bounded tasks, autonomous completion |

**Flow: Phase-Based Pipeline**

```
┌──────────┐   ┌──────────┐   ┌──────────┐
│ RESEARCH │──►│   PLAN   │──►│IMPLEMENT │
│ (Session)│   │(Session) │   │(Session) │
└────┬─────┘   └────┬─────┘   └────┬─────┘
     │              │              │
     ▼              ▼              ▼
 [Handoff]     [Handoff]     [Handoff]
```

**RW: Iterative Refinement Loop**

```
┌─────────────────────────────────────┐
│  Loop Start                         │
│  │                                  │
│  ├─► Build spec from SCUD task      │
│  ├─► Spawn sub-agent (fresh context)│
│  ├─► Execute + verify               │
│  │   ├─► Pass → mark done           │
│  │   └─► Fail → tune + retry        │
│  └─► Wave complete → commit         │
│                                     │
│  Exit: all done OR all blocked      │
└─────────────────────────────────────┘
```

---

## Flow: The Multi-Session Pipeline

### The Philosophy

Flow treats session boundaries as **deliberate checkpoints**. Each phase—research, plan, implement—runs in its own Claude session with fresh context. The handoff document transfers knowledge between sessions: key findings, critical files, decisions made.

This mirrors how human teams work: a researcher produces a report, a planner creates a spec, an implementer follows the spec. Each role can be different people (or different sessions).

### When Flow Shines

- **Features spanning days or weeks** — Handoffs let you pause and resume across multiple sessions
- **Team collaboration** — Handoff documents provide context for others to pick up work
- **Need clear phase boundaries** — Review gates between research, planning, implementation
- **Planning needs human sign-off** — Natural pause point before implementation begins
- **Complex research required** — Separate session for deep investigation before committing to a plan

### The Handoff System

Handoffs live in `thoughts/shared/handoffs/{phase}/` and contain:

- **Research handoffs**: Key findings, critical files, patterns discovered
- **Plan handoffs**: SCUD tag, wave breakdown, architecture decisions, files to modify
- **Implement handoffs**: Completion status, commits made, learnings

Each handoff includes `next_command` telling you exactly how to continue:

```yaml
next_phase: implement
next_command: "/flow:implement auth-system"
```

---

## RW: The Iterative Refinement Loop

### The Philosophy

RW implements Geoffrey Huntley's insight: **feed the same spec repeatedly, let the agent see its previous work in git**. Each iteration has fresh context but identical specification. The agent builds on what it finds in files and history.

```bash
# The original Ralph pattern
while :; do cat PROMPT.md | claude-code; done
```

Key principles from Geoff:

1. **Fixed spec allocation** — ~5,000 tokens dedicated to specs that persist
2. **Fresh context per iteration** — No cumulative history, just the spec
3. **External orchestration** — Descartes controls the loop, not the model
4. **Deterministic failures** — Repeated failures enable systematic debugging

### When RW Shines

- **Task is well-defined and bounded** — Clear scope, clear success criteria
- **Want autonomous completion** — Let it run until done without intervention
- **Iterative improvement beneficial** — Failures lead to refined prompts
- **Single-session completion feasible** — Hours, not days
- **Need auto-tuning** — "Tune the guitar" refines prompts on failure

### The Iteration Model

RW uses SCUD tasks as the "fixed spec":

| SCUD Element | Maps to Spec |
|--------------|--------------|
| Task title + description | Core objective |
| Implementation details | Detailed spec |
| Test strategy | Success criteria |
| Dependencies | Prerequisite context |

When verification fails, the tuner agent analyzes the failure and suggests refinements. Up to 3 retries before human checkpoint.

---

## The Shared Foundation: SCUD

Both Flow and RW build on SCUD task management. This is what they share:

### Wave-Based Execution

Tasks organize into dependency waves:

```
Wave 1: [Task A]           # No dependencies
Wave 2: [Task B, Task C]   # Both depend on A
Wave 3: [Task D]           # Depends on B and C
```

Both systems process waves sequentially, with tasks in a wave executed one at a time.

### Completion Detection

Identical in both:
- `pending == 0` — No tasks waiting
- `in_progress == 0` — No tasks currently running

No magic tokens or promise tags. Just task state.

### Auto-Commit After Waves

Both commit after completing each wave, creating natural rollback points:

```bash
git commit -m "feat(feature-x): complete wave 2"
```

### Task Status Flow

```
pending → in-progress → done
                     ↘ blocked (if verification fails)
```

---

## Decision Matrix

| Criterion | Choose Flow | Choose RW |
|-----------|-------------|-----------|
| **Scope** | Multi-day feature | Single-session task |
| **Team** | Multiple people involved | Solo developer |
| **Review** | Need approval gates | Trust autonomous execution |
| **Failures** | Want to debug manually | Want auto-tuning |
| **Context** | Need rich handoff docs | Fixed spec is sufficient |
| **Resumability** | Resume from any phase | Resume within single session |
| **Orchestration** | Claude Code native | External (Descartes CLI) |

### The Scope Question

**Flow** excels when you don't know the full scope upfront. Research discovers it, planning refines it, implementation follows the plan. Each phase can surface surprises.

**RW** excels when scope is known. You have tasks, you have a plan, you want autonomous execution until done.

### The Failure Question

**Flow** keeps you in the loop. Failures happen during your session; you debug interactively.

**RW** handles failures autonomously. The tuner agent analyzes what went wrong and suggests fixes. You only intervene after 3 failed attempts.

---

## The Hybrid Approach

The open question from the research: could these approaches combine?

### Option 1: Flow for Research/Plan, RW for Implement

The most natural hybrid. Use Flow's multi-session approach for discovery and planning, then hand off to RW for autonomous implementation.

```
/flow:research "authentication system"
    ↓ [handoff]
/flow:plan thoughts/shared/handoffs/research/...
    ↓ [handoff with SCUD tag]
/rw:loop auth-system --plan thoughts/shared/plans/auth-system.md
```

**How it might work:**

1. Research phase explores the problem space, produces handoff
2. Plan phase creates SCUD tasks with detailed specs, produces handoff
3. RW loop consumes the plan and SCUD tag, executes autonomously

**What this gains:**
- Human review between research/plan/implement
- Autonomous implementation with auto-tuning
- Best of both: thoughtful planning + relentless execution

**Open questions:**
- Should the plan handoff include RW-specific configuration?
- How does RW report back if it completes or blocks?
- Should RW generate an implement handoff for continuity?

### Option 2: RW with Handoff Generation

Could RW generate handoffs for partial completion scenarios?

Currently, RW either completes all tasks or blocks. But imagine:

```
RW completes 8/12 tasks, blocks on 4
    ↓
Generates handoff: "completed wave 1-2, blocked on wave 3"
    ↓
Human reviews, adjusts tasks
    ↓
/rw:loop --resume (or /flow:implement to continue differently)
```

**When this might help:**
- Team visibility into autonomous work
- Handoff to different developer mid-feature
- Documentation of what was learned during execution

**Trade-offs:**
- Adds complexity to RW's simplicity
- Handoff generation during autonomous execution feels contradictory
- Loop state file already captures similar information

### Option 3: Flow with Auto-Tuning

Should Flow's implement phase have tune-the-guitar capability?

Currently, `/flow:implement` runs in your Claude session. Failures are interactive—you see them, you fix them.

Adding auto-tuning would mean:

```
/flow:implement
    ↓
Task fails verification
    ↓
Tuner agent (same session) suggests refinement
    ↓
Retry with refined approach
```

**Trade-offs:**
- Pros: More autonomous within phase, fewer manual interventions
- Cons: Loses the interactive debugging that makes Flow valuable for complex work
- Middle ground: Optional `--auto-tune` flag?

### Open Questions

These hybrid approaches raise interesting design questions:

1. **Where does orchestration live?** Flow is Claude-native; RW uses Descartes CLI. Hybrids need clear boundaries.

2. **What context transfers?** Handoffs work for human-readable summaries. Do they work for machine-to-machine transitions?

3. **Who owns the session?** Flow assumes you're present. RW assumes you're not. Hybrids blur this.

4. **Could Flow phases themselves be RW loops?** Each phase as an autonomous loop with its own tune-the-guitar?

No definitive answers here—these are possibilities worth exploring based on your needs.

---

## Quick Reference

### Flow Commands

| Command | Purpose |
|---------|---------|
| `/flow:research` | Conduct research with handoff |
| `/flow:plan` | Create plan and SCUD tasks |
| `/flow:implement` | Execute waves with handoffs |
| `/flow:resume` | Resume from any handoff |
| `/flow:status` | Show active flows |

### RW Commands

| Command | Purpose |
|---------|---------|
| `/rw:loop <tag>` | Start SCUD loop |
| `/rw:cancel-ralph` | Stop active loop |
| `/rw:help` | Show usage |

### RW CLI (Descartes)

| Command | Purpose |
|---------|---------|
| `descartes loop start --scud-tag <tag>` | Start loop |
| `descartes loop status` | Check progress |
| `descartes loop tune` | Review failed attempts |
| `descartes loop tune --edit` | Manual prompt editing |
| `descartes loop resume` | Continue after intervention |

---

## Further Reading

- **[Flow Workflow](07-flow-workflow.md)** — Full Flow documentation with all six phases
- **[Iterative Loops](12-iterative-loops.md)** — RW loop mechanics and SCUD integration
- **[Tune the Guitar](13-tune-the-guitar.md)** — Automatic prompt refinement details

---

*Choose the right tool for the job—or combine them for the best of both worlds.*
