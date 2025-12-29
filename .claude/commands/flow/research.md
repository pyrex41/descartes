---
description: Enhanced research with handoff generation for phase transition
model: opus
---

# Flow Research

You are conducting research that will generate a structured handoff document for the planning phase. This enables clean context transfer between sessions.

## Initial Setup

When this command is invoked, respond with:
```
I'm ready to conduct flow research with handoff generation. Please provide your research topic or question.

Once complete, I'll create:
1. A research document in thoughts/shared/research/
2. A handoff document in thoughts/shared/handoffs/research/

The handoff enables a clean transition to planning in a new session.
```

Then wait for the user's research query.

## Process

### Step 1: Conduct Research

Follow the same thorough research process as `/cl:research_codebase_nt`:

1. **Read any mentioned files FULLY first**
   - Use the Read tool WITHOUT limit/offset to read entire files
   - Read these in the main context before spawning sub-tasks

2. **Analyze and decompose the research question:**
   - Break down the query into composable research areas
   - Identify specific components, patterns, or concepts to investigate
   - Create a research plan using TodoWrite

3. **Spawn parallel sub-agent tasks:**
   - Use **codebase-locator** to find WHERE files and components live
   - Use **codebase-analyzer** to understand HOW specific code works
   - Use **codebase-pattern-finder** to find examples of existing patterns

4. **Wait for all sub-agents and synthesize findings:**
   - Compile all sub-agent results
   - Connect findings across different components
   - Include specific file paths and line numbers

5. **Write research document:**
   - Location: `thoughts/shared/research/{YYYY-MM-DD}-{topic-in-kebab-case}.md`
   - Include all findings with code references

### Step 2: Generate Handoff Document

After completing research, create a handoff document:

**Location**: `thoughts/shared/handoffs/research/{YYYY-MM-DD}_{HH-MM}_{topic}.md`

Use this format:
```markdown
---
type: handoff
phase: research
timestamp: {ISO timestamp}
topic: "{research topic}"
research_doc: "thoughts/shared/research/{path}.md"
git_commit: "{current commit hash}"
branch: "{current branch}"
next_phase: plan
next_command: "/flow:plan {this-handoff-path}"
---

# Research Handoff: {Topic}

## Status
Research complete. Ready for planning phase.

## Research Document
`{path to full research document}`

## Key Findings Summary

### Finding 1: {Title}
{2-3 sentence summary}
- **Reference**: `{file:line}`
- **Implication**: {what this means for implementation}

### Finding 2: {Title}
{2-3 sentence summary}
- **Reference**: `{file:line}`
- **Implication**: {what this means for implementation}

### Finding 3: {Title}
{2-3 sentence summary}
- **Reference**: `{file:line}`
- **Implication**: {what this means for implementation}

## Critical Files
Files the planner MUST read:
1. `{path}` - {why important}
2. `{path}` - {why important}
3. `{path}` - {why important}

## Existing Patterns to Follow
- **Pattern**: {name} in `{file:line}`
- **Pattern**: {name} in `{file:line}`

## Recommended Planning Approach
{2-3 sentences on how to approach the plan based on research}

## Open Questions for Planning
- {Question that research couldn't answer}
- {Design decision that needs human input}

---

## Next Steps

To continue in a new session:
```bash
# Start new Claude session, then run:
/flow:plan {this-handoff-path}
```
```

### Step 3: Present Summary

After writing both documents, present:
```
Research complete!

📄 Research document: {path}
📋 Handoff document: {handoff-path}

Key findings:
- {finding 1}
- {finding 2}
- {finding 3}

Critical files identified:
- {file 1}
- {file 2}
- {file 3}

To continue in a new session:
/flow:plan {handoff-path}
```

## Important Notes

- Always create BOTH the research document AND the handoff
- The handoff should be a focused summary, not a copy of the research
- Include specific file:line references for quick navigation
- Commit hash enables time-travel to the exact codebase state
- Open questions should capture what the planner needs to decide
