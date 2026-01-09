# Implementation Plan: Flow vs RW Comparison Blog Post

## Overview

Write blog post #14 for the Descartes documentation comparing Flow and RW (Ralph Wiggum) - two complementary approaches to autonomous AI development. The post should help readers understand the philosophical differences, decide when to use each approach, and explore hybrid possibilities.

## Current State Analysis

### Existing Documentation
- `07-flow-workflow.md` (667 lines) - Comprehensive Flow documentation
- `12-iterative-loops.md` (631 lines) - Iterative loops with SCUD integration
- `13-tune-the-guitar.md` (127 lines) - Auto-tuning feature for RW

### Gap Identified
No existing post compares these philosophies directly or helps users decide when to use which approach. The research document (`thoughts/shared/research/2026-01-09-flow-vs-rw-comparison.md`) fills this gap with detailed analysis.

### Key Discoveries
- Both systems share SCUD infrastructure for wave-based task management
- Flow uses handoff documents for context transfer between sessions
- RW uses fixed spec (~5k tokens) fed each iteration with fresh context
- "Tune the guitar" is RW-only; handoffs are Flow-only
- Completion detection is identical: `pending == 0 && in_progress == 0`

## Desired End State

A new blog post at `descartes/docs/blog/14-choosing-your-workflow.md` that:
1. Presents both philosophies clearly with visual diagrams
2. Provides a decision matrix for choosing between approaches
3. Explores hybrid possibilities with concrete examples
4. Links back to detailed documentation (07, 12, 13)
5. Uses concise, open-ended style inviting further exploration

### Verification
- [x] File exists at correct location with correct naming convention
- [x] Follows blog format (title, tagline, `---` dividers, `##` sections)
- [x] README.md updated with entry for post #14
- [x] Internal links work to posts 07, 12, 13
- [x] Diagrams render correctly in markdown

## What We're NOT Doing

- Not repeating detailed documentation from posts 07, 12, 13
- Not making definitive prescriptions about which approach is "better"
- Not implementing new hybrid workflow features (just discussing possibilities)
- Not adding code changes to Descartes itself

## Implementation Approach

Write a single blog post that serves as both a conceptual guide and practical decision tool. Use the established blog format with ASCII diagrams, tables, and cross-references.

---

## Phase 1: Write the Blog Post

### Overview
Create the blog post with all sections, following the established format.

### Changes Required

#### 1.1 Create Blog Post

**File**: `descartes/docs/blog/14-choosing-your-workflow.md`
**Changes**: New file with the following structure

```markdown
# Choosing Your Workflow: Flow vs RW

*Two philosophies for autonomous development, one shared foundation*

---

## The Two Paths

[Opening that frames the core distinction]

| Aspect | Flow | RW (Ralph Wiggum) |
|--------|------|-------------------|
| Model | Multi-session pipeline | Single-session iteration |
| Context | Handoff documents between sessions | Fixed spec fed each iteration |
| Origin | Descartes workflow design | Geoffrey Huntley's Ralph pattern |
| Best for | Large features, team collaboration | Bounded tasks, autonomous completion |

[Two side-by-side ASCII diagrams showing each approach]

---

## Flow: The Multi-Session Pipeline

### The Philosophy

[Explain: Session boundaries as deliberate checkpoints]
[Key insight: Fresh context per session, handoffs transfer knowledge]

### When Flow Shines

- Features spanning days or weeks
- Team collaboration (handoffs provide context for others)
- Need clear phase boundaries for review/approval
- Planning phase needs human sign-off before implementation
- Want to pause and resume across multiple sessions

### The Handoff System

[Brief explanation with reference to 07-flow-workflow.md]
[Diagram showing Research → Handoff → Plan → Handoff → Implement]

---

## RW: The Iterative Refinement Loop

### The Philosophy

[Explain Geoffrey Huntley's insight]
[Key principles: Fixed spec allocation, fresh context per iteration, external orchestration]

### When RW Shines

- Task is well-defined and bounded
- Want autonomous completion without intervention
- Iterative improvement on failures is beneficial
- Single-session completion is feasible
- Need "tune the guitar" automatic prompt refinement

### The Iteration Model

[Brief explanation with reference to 12-iterative-loops.md]
[Diagram showing the loop with tune-the-guitar]

---

## The Shared Foundation: SCUD

[Explain what both approaches share]

### Wave-Based Execution

```
Wave 1: [Task A]           # No dependencies
Wave 2: [Task B, Task C]   # Both depend on A
Wave 3: [Task D]           # Depends on B and C
```

### Completion Detection

Both systems use the same completion criteria:
- `pending == 0` - No tasks waiting
- `in_progress == 0` - No tasks currently running

### Auto-Commit After Waves

Both commit after completing each wave, creating natural checkpoints.

---

## Decision Matrix

[Table with criteria and which approach fits]

| Criterion | Choose Flow | Choose RW |
|-----------|-------------|-----------|
| Scope | Multi-day feature | Single-session task |
| Team | Multiple people involved | Solo developer |
| Review | Need approval gates | Trust autonomous execution |
| Failures | Want to debug manually | Want auto-tuning |
| Context | Need rich handoff docs | Fixed spec is sufficient |
| Resumability | Resume from any phase | Resume within single session |

---

## The Hybrid Approach

[Open-ended discussion of combining approaches]

### Option 1: Flow for Research/Plan, RW for Implement

[Diagram]
[Discussion of how handoff from Plan phase could feed into RW loop]
[Example workflow]

### Option 2: RW with Handoff Generation

[Discussion: Could RW generate handoffs for partial completion?]
[When this might be useful]

### Option 3: Flow with Auto-Tuning

[Discussion: Should Flow's implement phase have tune-the-guitar?]
[Trade-offs]

### Open Questions

- How should the handoff-to-RW transition work?
- Should RW generate handoffs for team visibility?
- Could Flow phases themselves be RW loops?

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
| `descartes loop tune` | Human review of failures |

---

## Further Reading

- **[Flow Workflow](07-flow-workflow.md)** — Full Flow documentation
- **[Iterative Loops](12-iterative-loops.md)** — RW loop mechanics and SCUD integration
- **[Tune the Guitar](13-tune-the-guitar.md)** — Automatic prompt refinement

---

*Choose the right tool for the job—or combine them for the best of both worlds.*
```

#### 1.2 Update README Index

**File**: `descartes/docs/blog/README.md`
**Changes**: Add entry for post #14 in Quick Links table and Blog Series

In Quick Links table (after line 23):
```markdown
| Choose between Flow and RW | [Choosing Your Workflow](14-choosing-your-workflow.md) |
```

In Blog Series, under "Advanced Topics" section (after line 89):
```markdown
14. **[Choosing Your Workflow: Flow vs RW](14-choosing-your-workflow.md)**

    Compare two philosophies for autonomous development: Flow's multi-session pipeline with handoffs versus RW's iterative refinement loop. Includes decision matrix and hybrid approaches.
```

### Success Criteria

#### Automated Verification:
- [x] File `14-choosing-your-workflow.md` exists in `descartes/docs/blog/`
- [x] README.md contains reference to post #14
- [x] No markdown syntax errors (lint warnings match project conventions - same as 07-flow-workflow.md)

#### Manual Verification:
- [x] Content accurately represents both approaches
- [x] Diagrams are clear and render correctly (ASCII art in code blocks)
- [x] Cross-references to posts 07, 12, 13 are accurate (4 links, all files exist)
- [x] Tone is balanced, not prescriptive (only 1 prescriptive word match)
- [x] Hybrid discussion is genuinely open-ended (uses "might", "could", "Trade-offs", "Open questions")

---

## Testing Strategy

### Content Review
- Verify accuracy against research document
- Ensure no misleading claims about either approach
- Check that hybrid possibilities are presented as exploratory

### Link Verification
- All internal links resolve to existing files
- Section references within linked posts are accurate

### Format Verification
- Follows established blog format patterns
- ASCII diagrams align properly
- Tables render correctly

---

## References

- Research document: `thoughts/shared/research/2026-01-09-flow-vs-rw-comparison.md`
- Flow documentation: `descartes/docs/blog/07-flow-workflow.md`
- Iterative loops: `descartes/docs/blog/12-iterative-loops.md`
- Tune the guitar: `descartes/docs/blog/13-tune-the-guitar.md`
- Blog README: `descartes/docs/blog/README.md`
