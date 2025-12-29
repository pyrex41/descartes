---
date: 2025-12-29T00:00:00-05:00
researcher: Claude
git_commit: 1e7a261
branch: claude/review-plugin-patterns-O6wMc
repository: descartes
topic: "Integrated Flow Prompt Patterns: Blending Ralph Wiggum, HumanLayer, and SCUD"
tags: [research, prompts, workflow, ralph-wiggum, humanlayer, scud, flow]
status: complete
last_updated: 2025-12-29
last_updated_by: Claude
---

# Research: Integrated Flow Prompt Patterns

**Date**: 2025-12-29
**Researcher**: Claude
**Repository**: descartes

## Research Question

How can we blend the best features from Ralph Wiggum (autonomous loops), HumanLayer (handoff documents), and SCUD (wave-based task management) into Descartes' flow system to create an integrated, autonomous development workflow?

## Summary

After analyzing all three systems, I've identified a cohesive architecture that combines:
1. **Ralph Wiggum's** self-correcting loop with completion detection
2. **HumanLayer's** structured handoff documents for clean context transfers
3. **SCUD's** wave-based parallelism and task tracking
4. **Descartes' existing** flow system and sub-agent infrastructure

The key insight is that these aren't competing approaches—they solve different parts of the same problem and can be layered together elegantly.

---

## Detailed Analysis

### System 1: Ralph Wiggum

**Core Concept**: "Ralph is a Bash loop" - a self-referential feedback mechanism.

**Key Strengths**:
- **Completion Detection**: Uses `<promise>` tags with truthful completion statements
- **Self-Correction**: Claude sees its previous work via files/git, enabling iterative improvement
- **Autonomous Execution**: Stop hook intercepts exits, keeps loop running
- **Clear Termination**: Measurable success criteria prevent infinite loops

**Implementation Pattern**:
```
Loop Start → Same Prompt → Claude Works → Check Promise →
  ↓ (not satisfied)                              ↓ (satisfied)
  └─────────────────────────────────────────── Exit
```

**What Descartes Can Adopt**:
- Completion detection via SCUD task states (all tasks DONE = loop complete)
- Self-correction by running verification commands and fixing failures
- Truthful completion promise: "All tests pass AND all tasks marked DONE"

**What to Avoid**:
- External stop hooks (keep it in-agent using TodoWrite + Task tools)
- Infinite loops without safeguards (use wave count as natural limit)

---

### System 2: HumanLayer

**Core Concept**: Structured handoff documents enable clean context transfers between sessions.

**Key Strengths**:
- **Handoff Documents**: Capture task state, learnings, references, and next steps
- **Resume Capability**: New session can pick up exactly where previous left off
- **Reference Preservation**: `file:line` syntax for easy navigation
- **Phase Workflow**: Research → Plan → Implement with clear boundaries

**Handoff Document Structure**:
```markdown
- Task Status (completed/in-progress/planned)
- Critical References (2-3 most important files)
- Recent Changes (file:line syntax)
- Learnings (patterns, insights discovered)
- Artifacts (files produced/modified)
- Action Items & Next Steps
```

**What Descartes Can Adopt**:
- Handoff document generation at phase transitions
- `resume_handoff` capability for session continuity
- Reference preservation format (`file:line`)
- Explicit "next steps" guidance for receiving agent

**What to Avoid**:
- Over-reliance on human intervention (auto-generate handoffs)
- Verbose handoffs (keep focused on actionable items)

---

### System 3: SCUD CLI

**Core Concept**: Wave-based parallel task execution with Fibonacci complexity scoring.

**Key Strengths**:
- **Wave Computation**: Automatic dependency analysis for parallel execution
- **Task States**: Clear P/I/D/R/B/X progression
- **Claim/Release**: Multi-agent coordination
- **Built-in Prompts**: PRD parsing, complexity analysis, task expansion

**What Descartes Already Has**:
- Full SCUD integration via `scud` CLI
- `scud:*` commands for role-based agents
- Wave-aware task execution

**What to Add**:
- Tighter integration with loop completion detection
- Automatic wave progression (no human approval between waves)
- Sub-agent spawning per task (currently manual)

---

### System 4: Descartes Current State

**Existing Infrastructure**:
- `cl:*` commands for research/plan/implement workflow
- `scud:*` commands for role-based agent workflow
- Flow executor (Rust) for phase orchestration
- Sub-agent system (codebase-locator, codebase-analyzer, etc.)
- `thoughts/shared/` for research, plans, handoffs
- TodoWrite for progress tracking

**Gaps to Fill**:
- No automatic handoff generation between phases
- No Ralph Wiggum-style autonomous loop
- Manual transitions between cl:* phases
- No integrated completion detection

---

## Proposed Integrated Architecture

### Phase 1: Research (Clean Context Entry)

**Trigger**: User provides question or PRD
**Agent**: Enhanced `cl:research_codebase_nt`
**Output**: `thoughts/shared/research/YYYY-MM-DD-*.md`

**Enhancements**:
1. Use parallel sub-agents (already implemented)
2. Generate **handoff summary** at end:

```markdown
## Research Handoff

### Ready for Planning
Research complete: [path to research doc]

### Key Findings Summary
- [Finding 1] → [file:line reference]
- [Finding 2] → [file:line reference]
- [Finding 3] → [file:line reference]

### Context for Planner
[2-3 sentences with the essential context the planning agent needs]

### Recommended Planning Approach
Based on research, suggest: [approach recommendation]

### Critical Files to Read
1. [file path] - [why important]
2. [file path] - [why important]
3. [file path] - [why important]

---
To continue: Start new session with `/flow:plan [research-doc-path]`
```

---

### Phase 2: Planning (Clean Context Entry)

**Trigger**: `/flow:plan [research-doc-path]` or handoff resume
**Agent**: Enhanced `cl:create_plan_nt` + SCUD task generation
**Output**:
- `thoughts/shared/plans/YYYY-MM-DD-*.md`
- SCUD tasks via `scud` CLI

**Workflow**:
1. Read research handoff + full research document
2. Interactive planning with user (existing behavior)
3. Generate implementation plan with phases
4. **NEW**: Auto-generate SCUD tasks from plan phases
5. **NEW**: Run `scud analyze` for complexity scoring
6. **NEW**: Run dependency analysis
7. **NEW**: Generate planning handoff

**SCUD Task Generation** (from plan phases):
```bash
# For each phase in the plan, create SCUD tasks
scud create --tag <feature> --title "Phase 1: [name]" --description "[from plan]"
scud create --tag <feature> --title "Phase 1.1: [subtask]" --depends 1
# ...

# Analyze complexity
scud analyze --tag <feature>

# Show waves for approval
scud waves --tag <feature>
```

**Planning Handoff**:
```markdown
## Planning Handoff

### Ready for Implementation
- Plan: [path to plan doc]
- SCUD Tag: [tag name]
- Total Tasks: [N]
- Waves: [M]
- Estimated Complexity: [total points]

### Wave Overview
| Wave | Tasks | Points | Description |
|------|-------|--------|-------------|
| 1    | 3     | 8      | Foundation  |
| 2    | 4     | 13     | Core logic  |
| ...  | ...   | ...    | ...         |

### Critical Context
- [Key constraint from planning]
- [Important pattern to follow]
- [Testing strategy]

### Files to Modify (from plan)
1. [file] - [what changes]
2. [file] - [what changes]

---
To continue: Start new session with `/flow:implement [tag-name]`
```

---

### Phase 3: Implementation (Ralph-Style Autonomous Loop)

**Trigger**: `/flow:implement [tag-name]` or handoff resume
**Agent**: Enhanced `scud:dev` with Ralph Wiggum loop behavior
**Output**: Completed code + commits per wave

**The Ralph-SCUD Hybrid Loop**:

```
┌──────────────────────────────────────────────────────────────┐
│                    IMPLEMENTATION LOOP                        │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Check Loop State                                          │
│     scud stats --tag <tag>                                    │
│     → All DONE? Exit loop with success                        │
│     → Max iterations? Exit with partial completion            │
│                                                               │
│  2. Get Current Wave                                          │
│     scud waves --tag <tag>                                    │
│     scud next --tag <tag>                                     │
│                                                               │
│  3. For Each Task in Wave (spawn sub-agents):                 │
│     a. Claim task: scud set-status <id> in-progress           │
│     b. Read plan phase for this task                          │
│     c. Implement changes                                      │
│     d. Run verification (tests, lint, typecheck)              │
│     e. If pass: scud set-status <id> done                     │
│     f. If fail: Fix and retry (max 3 attempts)                │
│     g. If stuck: scud set-status <id> blocked                 │
│                                                               │
│  4. Wave Completion                                           │
│     git add -A && git commit -m "feat(<tag>): wave N"         │
│     Update TodoWrite with progress                            │
│                                                               │
│  5. Loop Back to Step 1                                       │
│                                                               │
│  COMPLETION PROMISE:                                          │
│  "scud stats --tag <tag> shows 0 pending AND                  │
│   make test passes AND no blocked tasks"                      │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

**Key Differences from Pure Ralph Wiggum**:
1. **Natural Loop Boundary**: Waves provide natural iteration points
2. **Built-in Completion Detection**: `scud stats` replaces `<promise>` tags
3. **Sub-agent Delegation**: Spawn implementation sub-agents per task
4. **Commit Checkpointing**: Each wave is committed (rollback possible)
5. **Blocked Task Handling**: Can skip blocked tasks, continue with wave

**Implementation Agent Prompt Structure**:
```markdown
# Flow Implementation Loop

You are implementing tasks for tag: [tag-name]

## Completion Criteria
This loop is complete when:
1. `scud stats --tag [tag]` shows 0 pending tasks
2. All verification commands pass
3. No tasks are blocked (or blocked tasks are documented)

## Current State
- Wave: [N] of [M]
- Tasks Complete: [X] of [Y]
- Last Iteration: [timestamp]
- Iteration Count: [count] of [max]

## Loop Process

### On Each Iteration:
1. Run `scud stats --tag [tag]` to check completion
2. If complete, output completion summary and exit
3. Get next task with `scud next --tag [tag]`
4. Implement the task (see task details below)
5. Verify implementation passes tests
6. Mark task done or blocked
7. If wave complete, commit changes
8. Continue loop

### Task Implementation Pattern:
For each task:
- Read the corresponding plan phase
- Use codebase-pattern-finder to find similar implementations
- Write code following existing patterns
- Write/update tests
- Run verification: `make check test`
- Fix any failures (up to 3 attempts)

### Wave Completion Pattern:
After all tasks in a wave:
- `git add -A`
- `git commit -m "feat([tag]): complete wave [N] - [summary]"`
- Update TodoWrite with wave progress
- Continue to next wave

## Self-Correction
If tests fail:
1. Read the error output carefully
2. Identify the root cause
3. Fix the code
4. Re-run tests
5. If stuck after 3 attempts, mark task blocked with reason

## Current Task
[Populated dynamically by scud show]

## Relevant Plan Phase
[Populated from thoughts/shared/plans/...]

## Patterns to Follow
[Populated by codebase-pattern-finder results]
```

---

### Phase 4: Retrospective & Handoff

**Trigger**: Implementation loop completes or manual trigger
**Agent**: `scud:retrospective` + handoff generation
**Output**: Learning document + final handoff

**Retrospective Handoff**:
```markdown
## Implementation Complete

### Summary
- Tag: [tag-name]
- Tasks: [completed]/[total]
- Waves: [completed]/[total]
- Commits: [list of commit hashes]

### What Was Built
[Summary of features/changes]

### Key Learnings
- [Pattern discovered]
- [Challenge overcome]
- [Improvement for next time]

### Files Changed
[git diff --stat summary]

### Testing Notes
- All automated tests pass
- Manual testing areas: [list]

### Follow-up Items
- [ ] [Any remaining work]
- [ ] [Documentation to update]
- [ ] [PR to create]
```

---

## Command Structure for Integrated Flow

### New Commands to Create

| Command | Purpose | Phase |
|---------|---------|-------|
| `/flow:research` | Enhanced research with handoff generation | Research |
| `/flow:plan` | Planning + SCUD task creation | Planning |
| `/flow:implement` | Ralph-style autonomous implementation | Implementation |
| `/flow:resume` | Resume from any handoff document | Any |
| `/flow:status` | Show current flow state across all phases | Monitoring |

### Command Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   /flow:research [question]                                     │
│         │                                                       │
│         ▼                                                       │
│   thoughts/shared/research/YYYY-MM-DD-*.md                      │
│   + Research Handoff Summary                                    │
│         │                                                       │
│         │ (clean context break)                                 │
│         ▼                                                       │
│   /flow:plan [research-path]                                    │
│         │                                                       │
│         ▼                                                       │
│   thoughts/shared/plans/YYYY-MM-DD-*.md                         │
│   + SCUD tasks created                                          │
│   + Planning Handoff Summary                                    │
│         │                                                       │
│         │ (clean context break)                                 │
│         ▼                                                       │
│   /flow:implement [scud-tag]                                    │
│         │                                                       │
│         ▼                                                       │
│   ┌─────────────────────────────┐                               │
│   │ RALPH-STYLE LOOP            │                               │
│   │ ─────────────────           │                               │
│   │ → Check stats               │                               │
│   │ → Get next task             │                               │
│   │ → Spawn sub-agent           │                               │
│   │ → Verify/fix                │                               │
│   │ → Mark done                 │                               │
│   │ → Commit wave               │                               │
│   │ → Loop until complete       │                               │
│   └─────────────────────────────┘                               │
│         │                                                       │
│         ▼                                                       │
│   Implementation Complete                                       │
│   + Retrospective Handoff                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Strategy

### Phase 1: Create Handoff Infrastructure

1. **Create handoff generator utility**
   - Function that generates standardized handoff markdown
   - Used by all phase commands at completion
   - Stored in `.scud/handoffs/` or `thoughts/shared/handoffs/`

2. **Create `/flow:resume` command**
   - Reads handoff document
   - Determines which phase to resume
   - Bootstraps appropriate agent with context

### Phase 2: Enhance Existing Commands

1. **Enhance `cl:research_codebase_nt`** → becomes `/flow:research`
   - Add handoff generation at end
   - Include "next steps" section
   - Reference critical files for planner

2. **Enhance `cl:create_plan_nt`** → becomes `/flow:plan`
   - Auto-generate SCUD tasks from plan phases
   - Run complexity analysis
   - Generate planning handoff

### Phase 3: Create Implementation Loop

1. **Create `/flow:implement` command**
   - Ralph-style loop structure
   - Uses SCUD for task/wave management
   - Sub-agent spawning per task
   - Commit checkpointing per wave
   - Completion detection via `scud stats`

2. **Create implementation sub-agent template**
   - Focused on single task implementation
   - Reads plan context
   - Uses pattern-finder for consistency
   - Runs verification

### Phase 4: Integration & Flow State

1. **Create unified flow state file**
   ```json
   {
     "current_phase": "implement",
     "research_doc": "thoughts/shared/research/...",
     "plan_doc": "thoughts/shared/plans/...",
     "scud_tag": "feature-auth",
     "handoffs": [
       { "phase": "research", "path": "...", "timestamp": "..." },
       { "phase": "plan", "path": "...", "timestamp": "..." }
     ],
     "implementation": {
       "current_wave": 2,
       "total_waves": 4,
       "iteration_count": 7,
       "max_iterations": 50
     }
   }
   ```

2. **Create `/flow:status` command**
   - Show state across all phases
   - Display handoff chain
   - Show implementation progress

---

## Key Design Decisions

### 1. Clean Context vs. Continuous Session

**Decision**: Use **clean context breaks** between major phases.

**Rationale**:
- Prevents context overflow on large projects
- Handoffs capture essential state
- Each phase agent can be specialized
- Matches how humans actually work (not infinite context)

### 2. Loop Termination Strategy

**Decision**: Use **SCUD task states** as completion detection.

**Rationale**:
- Already integrated into codebase
- Natural loop boundary (waves)
- Objective (not dependent on Claude's self-assessment)
- Supports partial completion (blocked tasks documented)

### 3. Sub-Agent Usage in Implementation

**Decision**: Spawn sub-agents **per task**, not per wave.

**Rationale**:
- Keeps each sub-agent focused
- Limits blast radius of failures
- Enables better error isolation
- Matches SCUD's task-level granularity

### 4. Commit Strategy

**Decision**: Commit **per wave**, not per task.

**Rationale**:
- Waves are logically coherent units
- Reduces commit noise
- Enables wave-level rollback
- Matches SCUD's wave concept

### 5. Human Intervention Points

**Decision**: Minimize human intervention but keep **phase transitions** as optional checkpoints.

**Rationale**:
- Automated verification handles most cases
- Phase boundaries are natural review points
- User can skip checkpoints for full autonomy
- Manual verification still supported in plan success criteria

---

## Code References

### Existing Infrastructure to Leverage

- `descartes/core/src/flow_executor.rs:1-150` - Flow state management
- `.claude/commands/cl/research_codebase_nt.md` - Research prompt template
- `.claude/commands/cl/create_plan_nt.md` - Planning prompt template
- `.claude/commands/cl/implement_plan.md` - Implementation prompt template
- `.claude/commands/scud/dev.md` - Developer agent template
- `.claude/agents/cl/codebase-pattern-finder.md` - Pattern finding agent

### External Patterns to Adopt

- Ralph Wiggum stop hook pattern → Adapt to TodoWrite + loop logic
- HumanLayer handoff format → Adopt for phase transitions
- HumanLayer resume_handoff → Basis for `/flow:resume`

---

## Open Questions

1. **Hook Implementation**: Should we implement an actual stop hook like Ralph Wiggum, or keep the loop logic in the prompt? (Recommendation: prompt-based for now, easier to iterate)

2. **Handoff Storage**: Should handoffs go in `.scud/handoffs/` or `thoughts/shared/handoffs/`? (Recommendation: `thoughts/shared/handoffs/` for consistency)

3. **Sub-Agent Model**: Should implementation sub-agents use Sonnet or Opus? (Recommendation: Sonnet for cost efficiency, Opus for complex tasks)

4. **Max Iterations**: What's a reasonable default for max loop iterations? (Recommendation: 50, with wave count * 10 as dynamic alternative)

---

## Next Steps

1. **Create handoff document spec** - Finalize format for all phase handoffs
2. **Implement `/flow:research`** - Enhanced research with handoff
3. **Implement `/flow:plan`** - Planning with SCUD task generation
4. **Implement `/flow:implement`** - Ralph-style loop with SCUD
5. **Implement `/flow:resume`** - Handoff resume capability
6. **Create flow state management** - Unified state file
7. **Test end-to-end** - Full research → plan → implement cycle
