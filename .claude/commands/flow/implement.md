---
description: SCUD-aware implementation loop with wave-based execution
model: opus
---

# Flow Implement

You implement SCUD tasks using a wave-based loop, automatically committing after each wave completes.

## Initial Setup

**If a SCUD tag is provided:**
```
Loading SCUD tag: {tag}

Checking status...
```

Run these commands to gather state:
```bash
scud stats --tag {tag}
scud waves --tag {tag}
```

Then present:
```
📊 SCUD Status: {tag}

Tasks: {done}/{total} ({percentage}%)
Waves: {current}/{total}

Current Wave Tasks:
- [ ] Task {id}: {title}
- [ ] Task {id}: {title}
- [x] Task {id}: {title} (done)

Ready to begin implementation loop.
```

**If resuming from handoff:**
1. Read the implementation handoff
2. Verify SCUD state matches handoff
3. Continue from last wave

## Implementation Loop

Execute this loop until all tasks complete or blocked:

```
LOOP START
│
├─► Check Completion
│   scud stats --tag {tag}
│   └─► All done? → Exit with success
│
├─► Get Current Wave
│   scud waves --tag {tag}
│   scud next --tag {tag}
│   └─► No tasks in wave? → Commit wave, advance
│
├─► For Each Pending Task in Wave:
│   │
│   ├─► 1. Claim Task
│   │   scud set-status {id} in-progress --tag {tag}
│   │
│   ├─► 2. Gather Context
│   │   - Read task details
│   │   - Find relevant plan section
│   │   - Use codebase-pattern-finder for examples
│   │
│   ├─► 3. Implement
│   │   - Write the code
│   │   - Follow existing patterns
│   │   - Add/update tests as needed
│   │
│   ├─► 4. Verify
│   │   Run: cargo check && cargo test
│   │   └─► Fail? → Fix (up to 3 attempts)
│   │   └─► Still fail? → Mark blocked with reason
│   │
│   └─► 5. Complete
│       scud set-status {id} done --tag {tag}
│
├─► Wave Complete
│   git add -A
│   git commit -m "feat({tag}): complete wave {N}"
│   Present progress update
│
└─► Continue to Next Wave
```

## Task Execution Detail

For each task:

### 1. Claim the Task
```bash
scud set-status {id} in-progress --tag {tag}
```

### 2. Read Context
- Read the task description with `scud show {id} --tag {tag}`
- Read relevant plan section
- Use sub-agents if complex:
  - **codebase-pattern-finder** to find similar implementations
  - **codebase-analyzer** to understand affected components

### 3. Implement
- Follow existing code patterns
- Keep changes focused on the task
- Write tests alongside implementation

### 4. Verify
Run verification command (default: `cargo check && cargo test`):

```bash
# Try up to 3 times
for attempt in 1 2 3; do
    cargo check && cargo test
    if success; then break; fi
    # Fix issues and retry
done
```

If still failing after 3 attempts:
```bash
scud set-status {id} blocked --tag {tag} --reason "Test failure: {description}"
```

### 5. Complete
```bash
scud set-status {id} done --tag {tag}
```

## Progress Reporting

After each task:
```
Task {id} complete ✓

Progress: {completed}/{total} tasks
Wave {N}: {wave_done}/{wave_total} tasks
```

After each wave:
```
═══════════════════════════════════════════════════
Wave {N} Complete ✓

Tasks completed this wave: {n}
Commit: {short_hash}

Overall progress: {completed}/{total} ({percentage}%)

Remaining:
- Wave {N+1}: {m} tasks - {description}
- Wave {N+2}: {m} tasks - {description}

Continuing to Wave {N+1}...
═══════════════════════════════════════════════════
```

## Completion

When all tasks are done:
```
═══════════════════════════════════════════════════
Implementation Complete! ✓

📊 Summary:
- Tasks: {N} completed
- Waves: {M} completed
- Commits:
  - {hash1}: Wave 1 - {description}
  - {hash2}: Wave 2 - {description}
  - ...

Files changed: {count}
Lines added: +{n}
Lines removed: -{n}
═══════════════════════════════════════════════════

📋 Generating implementation handoff...
```

Then generate the implementation handoff:

**Location**: `thoughts/shared/handoffs/implement/{YYYY-MM-DD}_{HH-MM}_{tag}.md`

```markdown
---
type: handoff
phase: implement
timestamp: {ISO timestamp}
topic: "{feature name}"
scud_tag: "{tag-name}"
plan_doc: "thoughts/shared/plans/{path}.md"
plan_handoff: "thoughts/shared/handoffs/plan/{path}.md"
tasks_completed: {N}
tasks_total: {M}
waves_completed: {W}
waves_total: {T}
status: "complete"
git_commits: ["{hash1}", "{hash2}"]
branch: "{branch name}"
---

# Implementation Handoff: {Feature Name}

## Status
Implementation complete. All SCUD tasks done.

## Progress

**Tag**: `{tag-name}`
**Tasks**: {completed}/{total} (100%)
**Waves**: {completed}/{total}

### Completed Waves
- **Wave 1**: {summary} (commit: `{hash}`)
- **Wave 2**: {summary} (commit: `{hash}`)
- ...

## Commits Made
1. `{hash}` - {message}
2. `{hash}` - {message}

## Files Changed
{git diff --stat summary}

## Key Learnings
- {Learning 1}
- {Learning 2}

## Manual Testing Required
- [ ] {Test case 1}
- [ ] {Test case 2}

## Follow-up Items
- [ ] {Item 1}
- [ ] {Item 2}

---

## Next Steps

```bash
# Create PR
/cl:describe_pr

# Or run retrospective
/scud:retrospective
```
```

Present final summary:
```
Implementation complete!

📄 Handoff: {handoff-path}

Next steps:
- /cl:describe_pr - Create pull request description
- /scud:retrospective - Capture learnings

Ready to proceed?
```

## Error Handling

### Task Blocked

When a task is blocked after 3 attempts:
```
Task {id} blocked after 3 attempts.

Error: {error details}

Options:
1. Mark blocked and continue with remaining tasks
2. Investigate with sub-agent
3. Pause for human intervention

Choice (or provide fix):
```

### Dependency Issues

If a dependency is blocked:
```
Task {id} cannot proceed.

Blocked by: Task {dep_id} - {reason}

Skipping to next available task...
```

## Important Notes

- Always commit after completing a wave (not after each task)
- Use the SCUD CLI for all task state changes
- Keep iterations focused - don't over-engineer
- Verification is mandatory before marking done
- Blocked tasks should have clear reasons recorded
- The loop continues until all tasks are done OR all remaining tasks are blocked
