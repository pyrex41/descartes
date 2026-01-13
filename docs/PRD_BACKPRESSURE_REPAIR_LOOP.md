# PRD: Backpressure Repair Loop

## Problem Statement

In the deprecated `scud swarm`, when backpressure validation fails after a wave completes, the system marks all tasks from that wave as "failed" and proceeds to the next wave. This is fundamentally broken because:

1. **Cascading failures**: Wave 2 tasks depend on Wave 1 work - if Wave 1 doesn't compile, Wave 2 will also fail
2. **No recovery**: Failed tasks are abandoned rather than repaired
3. **Wasted compute**: Subsequent waves run against broken code, wasting agent cycles
4. **Manual intervention required**: Human must step in to fix compilation errors

## Desired Behavior

When backpressure checks fail, Descartes Ralph should:

1. **Block wave progression** - Do NOT proceed to the next wave
2. **Spawn repair agents** - Create subagents specifically tasked with fixing the failures
3. **Re-validate** - After repairs complete, re-run backpressure checks
4. **Loop until green** - Repeat steps 2-3 until all checks pass
5. **Then proceed** - Only advance to the next wave when backpressure is green

## Design

### Repair Loop Flow

```
Wave N completes
    │
    ▼
┌─────────────────────┐
│ Run Backpressure    │
│ (cargo build, test) │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐     PASS      ┌─────────────────────┐
│ All checks passed?  │──────────────▶│ Proceed to Wave N+1 │
└─────────────────────┘               └─────────────────────┘
    │
    │ FAIL
    ▼
┌─────────────────────┐
│ Parse Error Output  │
│ (compiler errors,   │
│  test failures)     │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Spawn Repair Agents │
│ (one per error or   │
│  grouped by file)   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Wait for Repairs    │
└─────────────────────┘
    │
    └──────────────────────┐
                           │ (loop back)
                           ▼
              ┌─────────────────────┐
              │ Run Backpressure    │
              └─────────────────────┘
```

### Repair Agent Design

Each repair agent receives:

1. **Error context**: The specific compiler error, test failure, or lint warning
2. **File context**: The file(s) involved in the error
3. **Wave context**: What tasks were attempting to accomplish (for understanding intent)
4. **Verification command**: The specific check they need to make pass

Example repair prompt:
```
The following build error occurred after Wave 1 tasks completed:

ERROR[E0433]: failed to resolve: use of undeclared crate or module `foo`
 --> src/ralph_executor.rs:15:5
  |
15 | use foo::Bar;
  |     ^^^ use of undeclared crate or module `foo`

Wave 1 tasks that may have caused this:
- Task 5.1: Define RalphExecutor Struct and new() Constructor
- Task 6.1: Implement build_prompt with verification placeholder

Your job is to fix this compilation error. After your fix, this command must pass:
  cargo build --release

Do NOT proceed to other tasks. Focus only on making the build pass.
```

### Configuration

Add to `.scud/config.toml`:

```toml
[swarm.backpressure]
commands = ["cargo build --release", "cargo test", "cargo clippy -- -D warnings"]
stop_on_failure = true
timeout_secs = 300

[swarm.repair]
enabled = true
max_repair_rounds = 5        # Give up after N failed repair attempts
max_parallel_repairs = 3     # How many repair agents at once
repair_timeout_secs = 600    # Timeout per repair agent
group_errors_by = "file"     # "file", "error_type", or "individual"
```

### Repair Strategies

#### Strategy 1: Individual Errors
Spawn one agent per distinct error. Best for unrelated issues.

#### Strategy 2: Group by File (default)
Group all errors in the same file into one repair task. Reduces conflicts.

#### Strategy 3: Single Repair Agent
One agent gets all errors. Simpler but slower. Good for tightly coupled errors.

### Failure Modes

1. **Max repair rounds exceeded**: After N failed repair attempts, mark wave as blocked and notify user
2. **Repair timeout**: If repair agent times out, retry with fresh context
3. **Repair makes it worse**: If error count increases, revert and try different strategy
4. **Circular repairs**: Detect if same error keeps recurring, escalate to user

### State Machine

```
WAVE_RUNNING
    │
    │ (all tasks complete)
    ▼
VALIDATING
    │
    ├─── (pass) ──▶ WAVE_COMPLETE ──▶ (next wave or DONE)
    │
    │ (fail)
    ▼
REPAIRING
    │
    ├─── (repairs complete) ──▶ VALIDATING (loop)
    │
    │ (max retries / timeout)
    ▼
BLOCKED (requires human intervention)
```

## Implementation Tasks

### Phase 1: Core Repair Loop
1. Add repair loop state machine to RalphExecutor
2. Implement error parsing for cargo build output
3. Implement error parsing for cargo test output
4. Create RepairAgent prompt builder
5. Integrate repair agents with existing harness system

### Phase 2: Smart Error Grouping
6. Implement file-based error grouping
7. Implement error-type grouping
8. Add configuration for grouping strategy

### Phase 3: Failure Handling
9. Implement max retry detection
10. Implement regression detection (error count increasing)
11. Add BLOCKED state and user notification
12. Implement repair revert on regression

### Phase 4: Observability
13. Add repair metrics to TUI (repair count, success rate)
14. Log repair attempts with full context
15. Generate repair summary after completion

## Success Criteria

1. When backpressure fails, system spawns repair agents instead of proceeding
2. System loops until backpressure passes or max retries exceeded
3. 80%+ of simple compilation errors are auto-repaired
4. Clear visibility into repair progress via TUI
5. Graceful degradation when repairs can't succeed

## Non-Goals

- Repairing logic errors (only compilation/lint/test failures)
- Automatic rollback of problematic commits
- Cross-wave dependency repair (only repairs current wave's issues)
