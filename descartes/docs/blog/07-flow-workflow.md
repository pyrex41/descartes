# The Flow Workflow: PRD to Production

*Transform requirements into working code, automatically*

---

The **Flow Workflow** is Descartes' flagship automation—a six-phase pipeline that takes a Product Requirements Document (PRD) and produces tested, documented code. It's autonomous, resumable, and observable at every step.

## The Vision

```
PRD Document ────▶ [FLOW WORKFLOW] ────▶ Working Code
                        │
              ┌─────────┼─────────┐
              ▼         ▼         ▼
           Tasks    Plans    Tests
```

Instead of manually breaking down requirements, creating tickets, writing plans, and implementing piece by piece—Flow handles the entire pipeline.

## The Six Phases

```
┌──────────┐   ┌──────────┐   ┌──────────┐
│  INGEST  │──▶│  REVIEW  │──▶│   PLAN   │
│ Parse PRD│   │ Optimize │   │ Generate │
│ → Tasks  │   │  Graph   │   │  Plans   │
└──────────┘   └──────────┘   └──────────┘
                                   │
┌──────────────────────────────────┘
│
▼
┌──────────┐   ┌──────────┐   ┌──────────┐
│IMPLEMENT │──▶│    QA    │──▶│SUMMARIZE │
│ Execute  │   │ Monitor  │   │ Generate │
│  Waves   │   │ Quality  │   │   Docs   │
└──────────┘   └──────────┘   └──────────┘
```

---

## Phase 1: Ingest

**Goal:** Parse the PRD into structured tasks.

### What Happens

1. Agent reads the PRD document
2. Identifies discrete features and requirements
3. Creates SCUD-format tasks with:
   - Titles and descriptions
   - Dependencies between tasks
   - Complexity estimates
   - Priority levels

### Output

```
.scud/tasks/tasks.scg

TASK-001: Set up authentication database schema
  depends_on: []
  complexity: moderate
  priority: high

TASK-002: Implement user registration API
  depends_on: [TASK-001]
  complexity: moderate
  priority: high

TASK-003: Build login endpoint
  depends_on: [TASK-001]
  complexity: simple
  priority: high
```

### Agent Used

`flow-ingest` (Researcher level)

---

## Phase 2: Review Graph

**Goal:** Optimize the task dependency graph.

### What Happens

1. Analyzes task relationships
2. Identifies parallelization opportunities
3. Detects cycles or impossible orderings
4. Suggests groupings for wave execution

### Optimizations

- **Parallelization:** Independent tasks grouped into waves
- **Dependency minimization:** Reduces blocking chains
- **Cycle detection:** Prevents deadlocks

### Output

```
Wave 1: [TASK-001]              # No dependencies
Wave 2: [TASK-002, TASK-003]    # Both depend on TASK-001
Wave 3: [TASK-004, TASK-005]    # Depend on Wave 2
```

### Agent Used

`flow-review-graph` (Researcher level)

---

## Phase 3: Plan Tasks

**Goal:** Create implementation plans for each task.

### What Happens

1. For each task, researches the codebase
2. Identifies relevant files and patterns
3. Writes detailed implementation plan
4. Documents approach and considerations

### Output

```
thoughts/shared/plans/
├── TASK-001-auth-schema.md
├── TASK-002-user-registration.md
├── TASK-003-login-endpoint.md
└── ...
```

### Plan Format

```markdown
# Implementation Plan: TASK-002 User Registration API

## Overview
Implement POST /api/auth/register endpoint...

## Files to Modify
- src/api/routes/auth.ts (new file)
- src/models/user.ts (add validation)
- src/database/migrations/... (add migration)

## Implementation Steps
1. Create user model with validation
2. Add password hashing utility
3. Implement registration endpoint
4. Add input validation middleware
5. Write integration tests

## Testing Strategy
- Unit tests for validation
- Integration tests for endpoint
- E2E test for registration flow

## Dependencies
- Requires TASK-001 (database schema) completed
```

### Agent Used

`flow-plan-tasks` (Planner level)

---

## Phase 4: Implement

**Goal:** Execute tasks wave-by-wave.

### What Happens

1. Loads task waves from Phase 2
2. For each wave, spawns implementation agents
3. Tasks execute sequentially within each wave
4. Waits for wave completion before next wave
5. Commits changes after each wave

> **Note:** Parallel execution within waves is planned for a future release. Currently, tasks execute one at a time.

### Wave Execution

```
Wave 1 ─────────▶ [Agent: TASK-001] ──────────▶ Commit
                         │
                         ▼
Wave 2 ─────────▶ [Agent: TASK-002] ┐
                 [Agent: TASK-003] ┼────────▶ Commit
                         │
                         ▼
Wave 3 ─────────▶ [Agent: TASK-004] ┐
                 [Agent: TASK-005] ┼────────▶ Commit
```

### Git Integration

After each wave:
```bash
git add .
git commit -m "feat(flow): Wave 2 - TASK-002, TASK-003"
```

Checkpoints enable rollback via git:
```bash
# Rollback to after Wave 1 using git
git log --oneline  # Find the wave commit
git reset --hard <commit-hash>
```

> **Note:** A dedicated `--rollback` CLI flag is planned for future releases.

### Agent Used

`flow-implement` (Orchestrator level) - spawns Minimal sub-agents per task

---

## Phase 5: QA (Quality Assurance)

**Goal:** Monitor quality during and after implementation.

### Concurrent Monitoring

QA runs **alongside** implementation, not just after:

```
Implementation ────────────────────────────────▶
QA Checks      ─────┬─────┬─────┬─────┬───────▶
                    │     │     │     │
                 Check  Check Check Check
```

### What QA Checks

- **Tests passing:** Runs test suite
- **Linting:** Code style compliance
- **Type checking:** Type safety
- **Build success:** Compilation works
- **Security:** Basic vulnerability scan

### Output

```
.scud/qa-log.json

[
  {
    "timestamp": "2025-01-15T11:30:00Z",
    "wave": 2,
    "checks": {
      "tests": "passed",
      "lint": "passed",
      "types": "2 warnings",
      "build": "passed"
    }
  }
]
```

### Agent Used

`flow-qa` (Researcher level)

---

## Phase 6: Summarize

**Goal:** Generate documentation for the implemented features.

### What Happens

1. Reviews all changes made
2. Generates changelog entries
3. Updates relevant documentation
4. Creates PR description (if applicable)
5. Writes executive summary

### Output

```
thoughts/shared/reports/
└── 2025-01-15-flow-summary.md
```

### Summary Format

```markdown
# Flow Execution Summary

## PRD: Authentication System
**Execution Time:** 45 minutes
**Tasks Completed:** 8/8
**Waves Executed:** 4

## Changes Made
- Added user authentication database schema
- Implemented registration and login endpoints
- Added JWT token management
- Created password reset flow
- Added authentication middleware
- Wrote 24 unit tests, 8 integration tests

## Files Modified
- 12 files added
- 5 files modified
- 0 files deleted

## Test Results
- Unit: 24/24 passing
- Integration: 8/8 passing
- Coverage: 87%

## Next Steps
- Deploy to staging for manual testing
- Security audit before production
```

### Agent Used

`flow-summarize` (Read-Only level)

---

## Running Flow

### Basic Usage

```bash
descartes workflow flow --prd requirements.md
```

### With Options

```bash
descartes workflow flow \
  --prd requirements.md \
  --tag feature-auth \
  --dir /path/to/project
```

### Resume Interrupted Flow

```bash
# Flow saves state after each phase
descartes workflow flow --prd requirements.md --resume
```

### Start from Specific Phase

> **Note:** The `--phase` flag is planned but not yet implemented. Currently, use `--resume` to continue from saved state.

```bash
# Resume from where you left off
descartes workflow flow --prd requirements.md --resume
```

---

## Configuration

### Flow Configuration

Flow currently uses sensible defaults. Custom configuration via TOML is planned for a future release.

**Default Settings:**
- Phase timeout: 30 minutes
- Max retries per phase: 3
- Auto-commit after each wave: enabled

> **Planned:** A `.scud/flow-config.toml` file will allow customization of these settings.

### Timeout Handling

When a phase times out:
1. Current work is saved
2. Orchestrator reviews the situation
3. Decides: **Retry**, **Skip**, or **Abort**

---

## State Management

### Flow State File

```json
// .scud/flow-state.json
{
  "flow_id": "flow-abc123",
  "prd_path": "requirements.md",
  "current_phase": "implement",
  "phases": {
    "ingest": {"status": "completed", "completed_at": "..."},
    "review": {"status": "completed", "completed_at": "..."},
    "plan": {"status": "completed", "completed_at": "..."},
    "implement": {"status": "in_progress", "wave": 2},
    "qa": {"status": "pending"},
    "summarize": {"status": "pending"}
  },
  "git_state": {
    "start_commit": "abc123",
    "phase_checkpoints": {
      "ingest": "def456",
      "review": "ghi789",
      "plan": "jkl012"
    }
  },
  "artifacts": {
    "tasks_path": ".scud/tasks/tasks.scg",
    "plans_dir": "thoughts/shared/plans/",
    "qa_log_path": ".scud/qa-log.json"
  }
}
```

### Recovery

```bash
# Check flow status
cat .scud/flow-state.json | jq '.current_phase'

# Resume from saved state
descartes workflow flow --prd requirements.md --resume

# Rollback to phase checkpoint (using git)
git log --oneline  # Find the checkpoint commit
git reset --hard <commit-hash>
```

---

## Error Handling

### The Orchestrator

When errors occur, the `flow-orchestrator` agent decides next steps:

```
Error in Phase ────▶ Orchestrator Review ────▶ Decision
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
           Retry         Skip        Abort
```

### Decisions

| Decision | When Used |
|----------|-----------|
| **Retry** | Transient failure, can recover |
| **Skip** | Non-critical task, move on |
| **Abort** | Critical failure, stop flow |
| **Continue** | False alarm, proceed normally |

### Retry Logic

```
Attempt 1 → Fail → Wait 1s
Attempt 2 → Fail → Wait 2s
Attempt 3 → Fail → Wait 4s
Attempt 4 → Fail → Orchestrator decides
```

---

## Artifacts

### Complete Artifact Tree

```
project/
├── .scud/
│   ├── flow-state.json           # Flow progress
│   ├── tasks/
│   │   └── tasks.scg             # Task definitions
│   ├── sessions/
│   │   ├── flow-ingest-xxx.json  # Phase transcripts
│   │   ├── flow-review-xxx.json
│   │   └── ...
│   └── qa-log.json               # QA results
├── thoughts/
│   └── shared/
│       ├── plans/
│       │   ├── TASK-001-xxx.md   # Implementation plans
│       │   └── ...
│       └── reports/
│           └── 2025-01-15-flow-summary.md
└── ... (your code changes)
```

---

## Best Practices

### 1. Write Good PRDs

The better your PRD, the better the results:

```markdown
# Feature: User Authentication

## Requirements
- Users can register with email and password
- Passwords must be at least 8 characters
- Email verification required before login
- JWT tokens for session management
- Password reset via email

## Technical Constraints
- Use PostgreSQL for user storage
- Argon2 for password hashing
- 15-minute token expiry

## Out of Scope
- Social login (future feature)
- Two-factor authentication (future feature)
```

### 2. Review Before Implement

```bash
# Start the workflow - it saves state after each phase
descartes workflow flow --prd requirements.md

# If you need to pause, use Ctrl+C - state is preserved
# Review generated plans
ls thoughts/shared/plans/

# Resume from saved state
descartes workflow flow --prd requirements.md --resume
```

### 3. Use Tags

```bash
descartes workflow flow --prd requirements.md --tag auth-v1

# Find artifacts later
ls .scud/sessions/ | grep auth-v1
```

### 4. Monitor Progress

```bash
# Watch flow state
watch cat .scud/flow-state.json | jq '.current_phase'

# Tail QA log
tail -f .scud/qa-log.json
```

---

## Troubleshooting

### "Phase Timeout"

Increase timeout:
```toml
[flow]
phase_timeout_secs = 3600  # 1 hour
```

### "Too Many Retries"

Check the specific error:
```bash
descartes logs --format json | jq '.entries[] | select(.error)'
```

### "Tasks Not Found"

Ensure ingest phase completed:
```bash
cat .scud/flow-state.json | jq '.phases.ingest.status'
```

### "Git Conflicts"

Rollback and resolve:
```bash
# Find the wave commit before the conflict
git log --oneline
git reset --hard <commit-hash>
# Resolve conflicts manually
descartes workflow flow --prd requirements.md --resume
```

---

## SCUD Integration

The Flow Workflow can integrate with SCUD, an external task management system, for enhanced tracking and execution.

> **Note:** SCUD is a separate tool with its own CLI. The commands below are SCUD commands, not Descartes commands. If you don't have SCUD installed, Flow uses its own internal task representation.

### Task Management (with SCUD)

When SCUD is available, Flow can use these SCUD commands:
- `scud parse-prd` - Generate tasks from PRD (Ingest phase)
- `scud list` / `scud show` - View task details
- `scud expand` - Break complex tasks into subtasks
- `scud waves` - Compute execution waves
- `scud set-status` - Update task status during implementation
- `scud stats` - Track overall progress

### Wave Computation

Tasks are organized into waves based on dependencies:

```
Wave 1: [Task A]           # No dependencies
Wave 2: [Task B, Task C]   # Both depend on A
Wave 3: [Task D]           # Depends on B and C
```

The `flow-review-graph` agent optimizes wave groupings for maximum parallelization.

### Task Status Flow

During implementation, tasks transition through statuses:

```
pending → in-progress → done
                     ↘ blocked (if verification fails)
```

The implementation agent:
1. Claims task with `scud set-status <id> in-progress`
2. Executes implementation
3. Runs verification
4. Marks `done` or `blocked` based on result

### Customizing Flow Agents

Flow agents are defined in `agents/flow-*.md` files:

| Agent | File | Tool Level | Purpose |
|-------|------|------------|---------|
| Orchestrator | flow-orchestrator.md | orchestrator | Error handling decisions |
| Ingest | flow-ingest.md | researcher | PRD parsing |
| Review Graph | flow-review-graph.md | researcher | Dependency optimization |
| Plan Tasks | flow-plan-tasks.md | planner | Implementation planning |
| Implement | flow-implement.md | orchestrator | Task execution |
| QA | flow-qa.md | researcher | Quality monitoring |
| Summarize | flow-summarize.md | readonly | Report generation |

You can customize agent behavior by editing these files. Each agent has:
- Metadata (name, model, tool_level, tags)
- Responsibilities
- Process steps
- Output format

---

## Next Steps

- **[Skills System →](08-skills-system.md)** — Extend Flow with custom skills
- **[GUI Features →](09-gui-features.md)** — Visual workflow monitoring
- **[Advanced Features →](11-advanced-features.md)** — Time-travel debugging

---

*From requirements to reality—that's the power of Flow.*
