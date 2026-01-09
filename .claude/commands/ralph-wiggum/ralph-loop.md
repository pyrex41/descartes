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

## Execution

1. Parse arguments to extract tag and options
2. Verify SCUD tag exists: `scud stats --tag {tag}`
3. Start the loop via Descartes CLI:

```bash
descartes loop start \
    --scud-tag {tag} \
    --plan {plan_path} \
    --spec-file {spec_files...} \
    --verify "cargo check && cargo test"
```

4. Monitor progress and report status

## Example Usage

```
/ralph-wiggum:ralph-loop my-feature --plan thoughts/shared/plans/my-feature.md
```

## Output Format

```
Starting Ralph loop for tag: {tag}

📊 Initial Status:
- Tasks: {pending}/{total}
- Waves: {total_waves}

🔄 Loop running...
- Use /ralph-wiggum:cancel-ralph to stop
- Progress saved to .scud/loop-state.json

Wave 1: Implementing {n} tasks...
  ✓ Task 1: {title}
  ✓ Task 2: {title}
  ✗ Task 3: {title} (blocked: {reason})

Wave 1 complete. Committed: {hash}

...continues until all tasks done or all blocked...
```
