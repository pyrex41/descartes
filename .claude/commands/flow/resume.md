---
description: Resume from any handoff document
model: opus
---

# Flow Resume

Resume work from a handoff document, picking up exactly where the previous session left off.

## Initial Setup

When this command is invoked with a handoff path:

```
Loading handoff: {path}

Analyzing handoff type and state...
```

Read the handoff document and determine:
1. **Phase**: research, plan, or implement
2. **Status**: complete or partial
3. **Next action**: what to do next

## Process

### Step 1: Read and Parse Handoff

Read the handoff document and extract:
- `type` and `phase` from frontmatter
- `status` (complete/partial/blocked)
- All referenced documents
- Git state (branch, commit)

### Step 2: Verify Environment

Check that the current environment matches the handoff:

```bash
# Check branch
git branch --show-current

# Check for uncommitted changes
git status --porcelain

# Verify referenced files exist
# Check SCUD state if applicable
```

Present verification results:
```
Handoff Verification:

Document: {path}
Phase: {phase}
Status: {status}

Environment Check:
✓ Branch matches: {branch}
✓ Referenced files exist
✓ SCUD tag present (if applicable)
⚠ Uncommitted changes detected (if any)

Context loaded. Ready to continue.
```

### Step 3: Load Context Based on Phase

**For Research Handoff (phase: research):**
- Read the referenced research document
- Review key findings
- Present planning options

```
Research handoff loaded.

Topic: {topic}
Research Document: {path}

Key Findings:
{summary of findings}

Ready to proceed with planning.

Continue with /flow:plan using this handoff? (y/n)
```

If yes, internally invoke the plan phase with the handoff context.

**For Plan Handoff (phase: plan):**
- Read the plan document
- Check SCUD task status
- Resume or start implementation

```
Plan handoff loaded.

Topic: {topic}
SCUD Tag: {tag}

Task Status:
- Total: {N}
- Completed: {done}
- Remaining: {pending}
- Blocked: {blocked}

Current Wave: {wave_num} - {description}
Pending Tasks:
- Task {id}: {title}
- Task {id}: {title}

Ready to continue implementation.

Continue with /flow:implement {tag}? (y/n)
```

If yes, internally invoke the implement phase.

**For Implementation Handoff (phase: implement):**
- Check if implementation was complete or partial
- Verify current SCUD state
- Resume implementation or proceed to next steps

```
Implementation handoff loaded.

Topic: {topic}
SCUD Tag: {tag}
Status: {complete|partial|blocked}

Progress at handoff:
- Tasks: {done}/{total}
- Waves: {waves_done}/{waves_total}

Current SCUD state:
- Tasks: {current_done}/{total}
- Blocked: {blocked}

{If complete}
Implementation was complete. Suggested next steps:
- /cl:describe_pr - Create pull request
- /scud:retrospective - Capture learnings

{If partial}
Implementation was partial. Resume from Wave {wave_num}?

{If blocked}
Implementation was blocked. Review blocked tasks?
```

### Step 4: Handle State Mismatches

If the environment has drifted from the handoff:

```
⚠ State Mismatch Detected

Handoff says:
- Branch: {expected_branch}
- Commit: {expected_commit}
- Tasks done: {expected_done}

Current state:
- Branch: {current_branch}
- Commit: {current_commit}
- Tasks done: {current_done}

Options:
1. Continue anyway (environment has progressed)
2. Investigate differences
3. Abort and verify manually

Choice:
```

## Quick Reference

| Handoff Phase | Next Command | What Happens |
|---------------|--------------|--------------|
| research | /flow:plan | Start planning with research context |
| plan | /flow:implement | Begin SCUD task execution |
| implement (partial) | /flow:implement | Resume from last wave |
| implement (complete) | /cl:describe_pr | Create PR |

## Example Flow

```
/flow:resume thoughts/shared/handoffs/plan/2025-12-29_15-30_auth-system.md

Reading handoff: thoughts/shared/handoffs/plan/2025-12-29_15-30_auth-system.md

Handoff Type: Planning
Topic: Auth System
SCUD Tag: auth-system
Tasks: 8 total, 0 completed
Plan: thoughts/shared/plans/2025-12-29-auth-system.md

Verifying state...
✓ Branch matches: feature/auth-system
✓ SCUD tag exists with 8 pending tasks
✓ Plan document exists

Ready to begin implementation.

Continue with /flow:implement auth-system? (y/n)
> y

Starting implementation loop...
```

## Important Notes

- Always verify environment matches handoff before proceeding
- If changes have been made since the handoff, acknowledge them
- For partial implementations, check which tasks are already done
- Handoffs are snapshots - reality may have changed
- Use TodoWrite to track what was loaded and what's next
