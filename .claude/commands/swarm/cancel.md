---
description: Cancel active Swarm loop
---

# Cancel Swarm Loop

Stop an active Swarm loop and preserve state for later resume.

## Execution

1. Check for active loop: `descartes swarm --status`
2. If active, cancel: `descartes swarm --cancel`
3. Report final state

## Output

```
Cancelling Swarm loop...

Final Status:
- Tag: {tag}
- Tasks completed: {done}/{total}
- Waves completed: {waves}
- State saved to: .scud/loop-state.json

To resume later: /swarm:loop {tag} --resume
```
