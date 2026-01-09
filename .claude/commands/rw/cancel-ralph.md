---
description: Cancel active Ralph Wiggum loop
---

# Cancel Ralph Loop

Stop an active Ralph loop and preserve state for later resume.

## Execution

1. Check for active loop: `descartes loop status`
2. If active, cancel: `descartes loop cancel`
3. Report final state

## Output

```
Cancelling Ralph loop...

📊 Final Status:
- Tag: {tag}
- Tasks completed: {done}/{total}
- Waves completed: {waves}
- State saved to: .scud/loop-state.json

To resume later: /rw:loop {tag} --resume
```
