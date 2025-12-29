---
description: Planning with SCUD task generation and handoff document
model: opus
---

# Flow Plan

You create implementation plans and automatically generate SCUD tasks for wave-based execution.

## Initial Setup

When this command is invoked:

**If a handoff path is provided:**
```
Loading research handoff: {path}
Reading referenced documents...
```
1. Read the handoff document
2. Read the referenced research document
3. Read all critical files listed in the handoff

**If no handoff but a topic is provided:**
```
No handoff document provided. Options:
1. Run /flow:research first for comprehensive context
2. Provide topic directly and I'll research as needed

What would you like to do?
```

## Process

### Step 1: Gather Context

If resuming from handoff:
- Use the key findings as your foundation
- Read the critical files listed
- Review existing patterns identified

If starting fresh:
- Conduct focused research using sub-agents
- Identify relevant code patterns
- Map the affected components

### Step 2: Create Plan

Follow the `/cl:create_plan_nt` approach:

1. **Design options analysis:**
   - Present 2-3 architectural approaches
   - Compare trade-offs
   - Recommend an approach with rationale

2. **User approval:**
   - Get explicit approval before proceeding
   - Incorporate any feedback

3. **Write plan document:**
   - Location: `thoughts/shared/plans/{YYYY-MM-DD}-{feature-name}.md`
   - Include implementation phases
   - Specify files to modify
   - Define testing strategy

### Step 3: Generate SCUD Tasks

After plan approval, create SCUD tasks:

```bash
# Create tag (kebab-case, descriptive)
scud init --tag {feature-name}

# Create tasks for each implementation step:
# - Group by phase/wave
# - Set dependencies (--depends)
# - Include complexity estimates

# Example:
scud create --tag {tag} --title "Phase 1: Create base types" --complexity 3
scud create --tag {tag} --title "Phase 1.1: Add serialization" --depends 1 --complexity 2
scud create --tag {tag} --title "Phase 2: Implement core logic" --depends 1 --complexity 5
scud create --tag {tag} --title "Phase 2.1: Add error handling" --depends 3 --complexity 3
scud create --tag {tag} --title "Phase 3: Integration" --depends 3,4 --complexity 5
scud create --tag {tag} --title "Phase 4: Tests" --depends 5 --complexity 3

# Analyze to assign waves
scud analyze --tag {tag}

# Show wave breakdown
scud waves --tag {tag}
```

**Task Creation Guidelines:**
- Each task should be completable in 1-3 iterations
- Complexity: 1-2 (small), 3-5 (medium), 5-8 (large)
- Use dependencies to create natural waves
- Include verification/test tasks

### Step 4: Generate Handoff Document

**Location**: `thoughts/shared/handoffs/plan/{YYYY-MM-DD}_{HH-MM}_{tag}.md`

```markdown
---
type: handoff
phase: plan
timestamp: {ISO timestamp}
topic: "{feature name}"
plan_doc: "thoughts/shared/plans/{path}.md"
research_handoff: "thoughts/shared/handoffs/research/{path}.md"
scud_tag: "{tag-name}"
total_tasks: {N}
total_waves: {M}
total_complexity: {points}
git_commit: "{commit hash}"
branch: "{branch name}"
next_phase: implement
next_command: "/flow:implement {scud-tag}"
---

# Planning Handoff: {Feature Name}

## Status
Planning complete. SCUD tasks created. Ready for implementation.

## Documents
- **Plan**: `{path to plan document}`
- **Research**: `{path to research document}`

## SCUD Overview

**Tag**: `{tag-name}`
**Tasks**: {N} total
**Waves**: {M} waves
**Complexity**: {points} points

### Wave Breakdown

| Wave | Tasks | Points | Focus |
|------|-------|--------|-------|
| 1 | {n} | {p} | {description} |
| 2 | {n} | {p} | {description} |
| ... | ... | ... | ... |

### Task Summary

#### Wave 1 (Foundation)
- [ ] Task {id}: {title} [{complexity}]
- [ ] Task {id}: {title} [{complexity}]

#### Wave 2 (Core)
- [ ] Task {id}: {title} [{complexity}] ← {dependency}
- [ ] Task {id}: {title} [{complexity}] ← {dependency}

## Critical Context for Implementation

### Architecture Decisions
- {Decision 1}: {rationale}
- {Decision 2}: {rationale}

### Patterns to Follow
- **For {component}**: Follow pattern in `{file:line}`
- **For {component}**: Follow pattern in `{file:line}`

### Testing Strategy
- Unit tests: {approach}
- Integration tests: {approach}
- Manual verification: {what to test}

## Files to Modify
1. `{path}` - {what changes}
2. `{path}` - {what changes}
3. `{path}` - {what changes}

## Success Criteria
- [ ] All SCUD tasks marked DONE
- [ ] Tests pass
- [ ] {Manual verification item}
- [ ] {Manual verification item}

---

## Next Steps

To continue in a new session:
```bash
# Start new Claude session, then run:
/flow:implement {scud-tag}

# Or to resume from this handoff:
/flow:resume {this-handoff-path}
```
```

### Step 5: Present Summary

```
Planning complete!

📄 Plan: {path}
🏷️ SCUD Tag: {tag}
📊 Tasks: {N} tasks in {M} waves
📋 Handoff: {handoff-path}

Wave breakdown:
- Wave 1: {n} tasks - {description}
- Wave 2: {n} tasks - {description}
- Wave 3: {n} tasks - {description}

To continue in a new session:
/flow:implement {tag}
```

## Important Notes

- Always get user approval on the plan before creating SCUD tasks
- Tasks should map 1:1 with plan phases where possible
- Include realistic complexity estimates
- Dependencies create natural wave groupings
- The handoff should be self-contained for resuming in a new session
