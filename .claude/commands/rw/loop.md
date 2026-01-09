---
description: Start Ralph Wiggum loop for SCUD tag
---

# Ralph Loop

Start a Geoff-style iterative loop for implementing SCUD tasks.

## Arguments

$ARGUMENTS should be a SCUD tag name, optionally followed by flags:
- `--plan <path>` - Path to implementation plan
- `--spec <path>` - Additional spec file (can repeat)
- `--max-iterations <n>` - Safety limit (default: 100)
- `--tune` / `--no-tune` - Enable/disable auto-tuning (default: enabled)
- `--max-tune-attempts <n>` - Auto-retry attempts before human checkpoint (default: 3)

## Execution

1. Parse arguments to extract tag and options
2. Verify SCUD tag exists: `scud stats --tag {tag}`
3. Start the loop via Descartes CLI:

```bash
descartes loop start \
    --scud-tag {tag} \
    --plan {plan_path} \
    --spec-file {spec_files...} \
    --verify "cargo check && cargo test" \
    --max-tune-attempts 3
```

4. Monitor progress and report status

## Example Usage

```
/rw:loop my-feature --plan thoughts/shared/plans/my-feature.md
```

## Tuning Options ("Tune the Guitar")

When a task fails, the loop automatically:
1. Captures failure context (output, errors, git diff)
2. Spawns a "tuner" agent to suggest prompt refinements
3. Retries with refined prompt (up to `max_tune_attempts`)
4. If still failing, pauses for human review

### When Tasks Fail After Max Attempts

If a task fails after max attempts, the loop pauses:

1. Run `descartes loop tune` to review all attempts
2. Select a variant: `descartes loop tune --select 2`
3. Or edit manually: `descartes loop tune --edit`
4. Resume: `descartes loop resume`

## Output Format

```
Starting Ralph loop for tag: {tag}

📊 Initial Status:
- Tasks: {pending}/{total}
- Waves: {total_waves}
- Tuning: enabled (max 3 attempts)

🔄 Loop running...
- Use /rw:cancel-ralph to stop
- Progress saved to .scud/loop-state.json

Wave 1: Implementing {n} tasks...
  ✓ Task 1: {title}
  ✓ Task 2: {title} (succeeded on attempt 2)
  ⏸ Task 3: {title} (awaiting tune - 3 attempts failed)

Loop paused. Run `descartes loop tune` to review variants.
```
