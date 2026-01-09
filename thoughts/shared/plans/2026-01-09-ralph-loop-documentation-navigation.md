# Ralph Loop Documentation Navigation Fix

## Overview

Fix navigation gaps in the Descartes documentation so that new developers can discover all Ralph loop-related content. The documentation exists and is comprehensive; it just needs better cross-linking.

## Current State Analysis

Three documentation gaps were identified:

1. **README.md missing article 13** - The Quick Links table and Blog Series list stop at article 12
2. **Article 12 doesn't link to article 13** - "Next Steps" section links to 7, 10, 11 but not 13
3. **Slash commands not mentioned in blog** - The `/rw:*` commands exist but aren't referenced from blog articles

### Key Discoveries:
- README.md Quick Links table: `descartes/docs/blog/README.md:22`
- README.md Blog Series ends at 12: `descartes/docs/blog/README.md:82-84`
- Article 12 Next Steps: `descartes/docs/blog/12-iterative-loops.md:612-616`
- Slash commands exist at: `.claude/commands/rw/`

## Desired End State

After implementation:
1. README.md includes article 13 in both Quick Links and Blog Series
2. Article 12 links to article 13 in its "Next Steps" section
3. Article 12 mentions the slash commands in the SCUD Integration section
4. All Ralph loop documentation is discoverable from multiple entry points

### Verification:
- Read README.md and confirm article 13 appears in both tables
- Read article 12 and confirm it links to article 13
- Follow links to verify they resolve correctly

## What We're NOT Doing

- Not reorganizing the documentation structure
- Not splitting article 12 into separate articles
- Not creating a new "Ralph Loop Quickstart" guide
- Not modifying article 13 content (only article 12 gets slash command mention)
- Not modifying the implementation code

## Implementation Approach

Simple text edits to three markdown files. Each file gets one small addition.

## Phase 1: Update README.md

### Overview
Add article 13 to both navigation tables in the README.

### Changes Required:

#### 1.1 Add Quick Links entry

**File**: `descartes/docs/blog/README.md`
**Changes**: Add row for article 13 after line 22

After:
```markdown
| Run iterative loops | [Iterative Loops](12-iterative-loops.md) |
```

Add:
```markdown
| Tune the guitar (prompt refinement) | [Tune the Guitar](13-tune-the-guitar.md) |
```

#### 1.2 Add Blog Series entry

**File**: `descartes/docs/blog/README.md`
**Changes**: Add article 13 entry after article 12 (around line 84)

After:
```markdown
12. **[Iterative Loops](12-iterative-loops.md)**

    Autonomous task execution with the Ralph Wiggum pattern: repeatedly execute commands until completion detection, with SCUD integration for wave-based task tracking.
```

Add:
```markdown
13. **[Tune the Guitar](13-tune-the-guitar.md)**

    Automatic prompt refinement for failing tasks. When tasks don't pass verification, a tuner agent analyzes failures and suggests prompt improvements.
```

### Success Criteria:

#### Automated Verification:
- [x] File renders as valid markdown (no syntax errors)
- [x] Links resolve: `ls descartes/docs/blog/13-tune-the-guitar.md`

#### Manual Verification:
- [x] Quick Links table shows article 13 with correct link
- [x] Blog Series shows article 13 with proper description

**Implementation Note**: After completing this phase and all automated verification passes, proceed to Phase 2.

---

## Phase 2: Update Article 12 "Next Steps"

### Overview
Add link to article 13 in the Next Steps section and mention slash commands.

### Changes Required:

#### 2.1 Add article 13 to Next Steps

**File**: `descartes/docs/blog/12-iterative-loops.md`
**Changes**: Update Next Steps section (lines 612-616)

Replace:
```markdown
## Next Steps

- **[Flow Workflow](07-flow-workflow.md)** - PRD to code automation using iterative loops
- **[Sub-Agent Tracking](10-subagent-tracking.md)** - Monitor agents spawned during loops
- **[Advanced Features](11-advanced-features.md)** - Time-travel and state restoration
```

With:
```markdown
## Next Steps

- **[Tune the Guitar](13-tune-the-guitar.md)** - Automatic prompt refinement for failing tasks
- **[Flow Workflow](07-flow-workflow.md)** - PRD to code automation using iterative loops
- **[Sub-Agent Tracking](10-subagent-tracking.md)** - Monitor agents spawned during loops
- **[Advanced Features](11-advanced-features.md)** - Time-travel and state restoration
```

#### 2.2 Expand slash commands section

**File**: `descartes/docs/blog/12-iterative-loops.md`
**Changes**: Expand the slash commands subsection (around lines 314-323) to include a table with descriptions and reference to source files

Replace:
```markdown
### Slash Commands for Claude Code

Use these commands directly in Claude Code:

```
/rw:loop my-feature --plan thoughts/shared/plans/my-feature.md
/rw:cancel-ralph
/rw:help
```
```

With:
```markdown
### Slash Commands for Claude Code

Use these commands directly in Claude Code:

| Command | Description |
|---------|-------------|
| `/rw:loop <tag> [options]` | Start a SCUD loop for the given tag |
| `/rw:cancel-ralph` | Cancel the active loop |
| `/rw:help` | Show available commands and usage |

**Options for loop:**
- `--plan <path>` - Implementation plan document
- `--spec <path>` - Additional spec files (can repeat)
- `--max-iterations <n>` - Safety limit (default: 100)
- `--tune` / `--no-tune` - Enable/disable auto-tuning (default: enabled)
- `--max-tune-attempts <n>` - Retries before human checkpoint (default: 3)

See the command source files at `.claude/commands/rw/` for full details.
```

### Success Criteria:

#### Automated Verification:
- [x] File renders as valid markdown
- [x] Link to article 13 resolves: `ls descartes/docs/blog/13-tune-the-guitar.md`

#### Manual Verification:
- [x] Next Steps section shows article 13 as first item
- [x] Slash commands table is readable and complete

**Implementation Note**: After completing this phase and all automated verification passes, the plan is complete.

---

## Testing Strategy

### Manual Testing Steps:
1. Open README.md in a markdown viewer and verify article 13 appears in both tables
2. Click the article 13 links to verify they work
3. Open article 12 and scroll to Next Steps - verify article 13 link is present
4. Verify the slash commands table renders correctly

## References

- Research document: `thoughts/shared/research/2026-01-09-ralph-loop-documentation-review.md`
- README.md: `descartes/docs/blog/README.md`
- Article 12: `descartes/docs/blog/12-iterative-loops.md`
- Article 13: `descartes/docs/blog/13-tune-the-guitar.md`
- Slash commands: `.claude/commands/rw/`
