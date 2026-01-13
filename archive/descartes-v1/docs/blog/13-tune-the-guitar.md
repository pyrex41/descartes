# "Tune the Guitar" - Prompt Refinement Loop

Based on Geoffrey Huntley's Ralph Wiggum technique, the SCUD loop now supports automatic prompt refinement when tasks fail.

## How It Works

1. **Task fails** → Agent attempted but verification didn't pass
2. **Analyze failure** → Tuner agent examines output and suggests refinement
3. **Retry with refinement** → Prompt updated with guidance from failure
4. **Repeat** → Up to N times (default: 3)
5. **Human checkpoint** → If still failing, pause for human review

## Configuration

```bash
descartes loop start \
    --scud-tag my-feature \
    --tune                    # Enable tuning (default)
    --max-tune-attempts 5     # Retries before human checkpoint
```

Or disable tuning:

```bash
descartes loop start \
    --scud-tag my-feature \
    --no-tune                 # Use old behavior
```

## Human Review

When a task needs human intervention:

```bash
# View all attempts
descartes loop tune

# Select a variant
descartes loop tune --select 2

# Edit prompt manually
descartes loop tune --edit

# Resume
descartes loop resume
```

## Tuning Flow

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

## What Gets Captured

Each attempt records:
- **Prompt used** - The full spec sent to the agent
- **Agent output** - What the agent said/did (truncated)
- **Verification result** - stdout/stderr from verification command
- **Git diff** - Changes made by the agent (before revert)
- **Suggested refinement** - Tuner's suggestion for next attempt

## Tuner Agent Prompt

The tuner agent receives context about the failure:

```
## Original Task
**ID:** 42
**Title:** Implement feature X
**Description:** Add X to module Y

## Prompt Used
[full prompt...]

## What Happened
### Agent Output (truncated)
[agent's response...]

### Verification Error
[stderr from verification...]

### Git Diff
[changes made...]

## Your Job
Analyze why this failed and suggest a refinement...
```

## Philosophy

From Geoffrey Huntley's Ralph Wiggum:

> "Ralph is deterministically bad in an undeterministic world. When it fails, tune the guitar - add signage saying 'SLIDE DOWN, DON'T JUMP'."

The tuning loop implements this by:
- Capturing failure context (output, errors, diff)
- Having Claude analyze what went wrong
- Adding specific guidance to prevent the same failure
- Preserving all variants for human review

## See Also

- [Iterative Loops](./12-iterative-loops.md) - Base loop functionality
- [SCUD Integration](./07-flow-workflow.md) - Task management
