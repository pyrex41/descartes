---
date: 2025-12-29
author: Claude
status: complete
ticket: N/A
topic: Blog Documentation Updates for New Features
---

# Implementation Plan: Blog Documentation Updates

## Overview

Update the Descartes blog documentation to cover new features added in PRs #9 and #10:
- Comprehensive keyboard shortcuts (vim-like navigation)
- Iterative loop system (`descartes loop` commands)
- SCUD loop integration (wave-based execution)

## Phase 1: Update GUI Features (09-gui-features.md)

### Changes

Add comprehensive keyboard shortcuts documentation after the existing "Keyboard Navigation" section (line ~476).

**New Section Content:**

1. **Philosophy section** - Explain vim-style approach
2. **Global shortcuts table** - All-view shortcuts (1-6, Tab, Escape, r/F5)
3. **Chat View table** - i/a for insert mode, Ctrl+L clear
4. **Sessions View table** - j/k/g/G navigation, o for new, / for search
5. **Time Travel Debugger table** - h/l/g/G timeline, Space play, +/- zoom, L loop
6. **DAG Editor table** - Update existing table with Space+drag pan

### Files to Modify

- `docs/blog/09-gui-features.md` - Add ~150 lines after line 476

### Success Criteria

- [x] New keyboard reference section exists
- [x] All views have shortcut documentation
- [x] Vim-like philosophy explained
- [x] File renders correctly as markdown

---

## Phase 2: Create Iterative Loops Documentation (12-iterative-loops.md)

### Changes

Create new blog post documenting the iterative loop system.

**Content Structure:**

1. **Introduction** - What is the iterative loop, Ralph Wiggum style concept
2. **Quick Start** - Basic `descartes loop start` example
3. **CLI Commands** - start, status, resume, cancel with all options
4. **Completion Detection** - Promise tags, exit codes
5. **State Management** - State file, resume capability
6. **Backend Configuration** - claude, opencode, generic backends
7. **Git Integration** - Auto-commit, branch creation
8. **SCUD Integration** - Wave-based execution, task tracking
9. **GUI Visualization** - Loop view features
10. **Configuration Reference** - All config options

### Files to Create

- `docs/blog/12-iterative-loops.md` - ~500 lines

### Success Criteria

- [x] New blog post exists
- [x] All CLI commands documented
- [x] SCUD integration covered
- [x] Examples for each backend
- [x] GUI visualization explained

---

## Phase 3: Update CLI Commands Reference (03-cli-commands.md)

### Changes

Add loop commands section to the CLI reference.

**New Section:**

```markdown
## Loop Commands

Commands for iterative execution loops.

### loop start
### loop status
### loop resume
### loop cancel
```

### Files to Modify

- `docs/blog/03-cli-commands.md` - Add section for loop commands

### Success Criteria

- [x] Loop commands section added
- [x] All four commands documented
- [x] Options/flags listed
- [x] Examples provided

---

## Phase 4: Update Flow Workflow Documentation (07-flow-workflow.md)

### Changes

Add SCUD integration section explaining how Flow uses SCUD for task management.

**New Content:**

1. **SCUD Integration subsection** - How Flow leverages SCUD
2. **Wave Execution details** - How waves are computed and executed
3. **Task Status Transitions** - Status flow during implementation
4. **Customizing Flow Agents** - Brief guide to agent files

### Files to Modify

- `docs/blog/07-flow-workflow.md` - Add section before "Next Steps"

### Success Criteria

- [x] SCUD integration explained
- [x] Wave execution documented
- [x] Agent customization mentioned

---

## Phase 5: Update README Index

### Changes

Update `docs/blog/README.md` to include the new iterative loops post.

### Files to Modify

- `docs/blog/README.md` - Add entry for 12-iterative-loops.md

### Success Criteria

- [x] README updated with new post link
- [x] Navigation table updated

---

## Verification

After all phases:
- All markdown files render correctly
- Links between posts work
- No broken references
