---
description: Explain Ralph Wiggum technique and commands
---

# Ralph Wiggum Help

## What is Ralph?

The Ralph Wiggum technique (created by Geoffrey Huntley) is an iterative AI development loop:

```bash
while :; do cat PROMPT.md | claude-code; done
```

**Key principles:**
- Same spec fed each iteration (fresh context)
- Agent sees previous work in files/git
- External orchestration (not model-managed)
- Deterministic failures enable systematic improvement

## SCUD Integration

This implementation uses SCUD tasks as the "fixed spec":
- Task description = objective
- Plan section = detailed spec
- Test strategy = success criteria
- Completion via SCUD stats (not promise tags)

## Available Commands

### /rw:loop <tag> [options]

Start loop for SCUD tag:
```
/rw:loop my-feature --plan ./plan.md
```

Options:
- `--plan <path>` - Implementation plan document
- `--spec <path>` - Additional spec files
- `--max-iterations <n>` - Safety limit

### /rw:cancel-ralph

Stop active loop, preserve state for resume.

### /rw:help

Show this help.

## Learn More

- Original technique: https://ghuntley.com/ralph/
- Research doc: thoughts/shared/research/2026-01-08-ralph-loop-scud-integration.md
