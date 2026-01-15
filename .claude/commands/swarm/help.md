---
description: Explain Swarm technique and commands
---

# Swarm Help

## What is Swarm?

Swarm is Descartes' implementation of the Ralph Wiggum technique (created by Geoffrey Huntley) - an iterative AI development loop:

```bash
while :; do cat PROMPT.md | claude-code; done
```

**Key principles (from Ralph Wiggum):**
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

### /swarm:loop <tag> [options]

Start loop for SCUD tag:
```
/swarm:loop my-feature --plan ./plan.md
```

Options:
- `--plan <path>` - Implementation plan document
- `--spec <path>` - Additional spec files
- `--max-iterations <n>` - Safety limit

### /swarm:cancel

Stop active loop, preserve state for resume.

### /swarm:help

Show this help.

## Learn More

- Original technique: https://ghuntley.com/ralph/
- Research doc: thoughts/shared/research/2026-01-08-ralph-loop-scud-integration.md
