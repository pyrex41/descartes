---
date: 2025-12-06T20:54:13Z
researcher: Claude
git_commit: 1f6145b
branch: master
repository: pyrex41/descartes
topic: "CL Workflow Commands and Agents Documentation"
tags: [research, workflow, cl-commands, agents, documentation]
status: complete
last_updated: 2025-12-06
last_updated_by: Claude
---

# Research: CL Workflow Commands and Agents Documentation

**Date**: 2025-12-06T20:54:13Z
**Researcher**: Claude
**Git Commit**: 1f6145b
**Branch**: master
**Repository**: pyrex41/descartes

## Research Question
Document the cl workflow commands and agents, understand how they fit together, and create a README explaining the workflow.

## Summary

The `.claude/` directory contains a structured AI-assisted development workflow with three main phases:
1. **Research** - Understanding existing code
2. **Planning** - Creating detailed implementation plans
3. **Implementation** - Executing plans with verification gates

## Detailed Findings

### Commands Structure

Located in `.claude/commands/cl/`:

| Command | Description | Model |
|---------|-------------|-------|
| `research_codebase_nt.md` | Document codebase (read-only) | Default |
| `create_plan_nt.md` | Create implementation plans | Opus |
| `iterate_plan_nt.md` | Update existing plans | Opus |
| `implement_plan.md` | Execute approved plans | Default |
| `commit.md` | Create git commits | Default |
| `describe_pr.md` | Generate PR descriptions | Default |

### Agents Structure

Located in `.claude/agents/cl/`:

| Agent | Purpose | Tools | Model |
|-------|---------|-------|-------|
| `codebase-locator` | Find WHERE files live | Grep, Glob, LS | Sonnet |
| `codebase-analyzer` | Understand HOW code works | Read, Grep, Glob, LS | Sonnet |
| `codebase-pattern-finder` | Find existing patterns | Grep, Glob, Read, LS | Sonnet |
| `thoughts-locator` | Find docs in thoughts/ | Grep, Glob, LS | Sonnet |
| `thoughts-analyzer` | Extract insights from docs | Read, Grep, Glob, LS | Sonnet |
| `web-search-researcher` | Search web for info | WebSearch, WebFetch, etc. | Sonnet |

### Workflow Data Flow

1. **Research Phase** (`/cl:research_codebase_nt`)
   - Spawns `codebase-locator` to find relevant files
   - Spawns `codebase-analyzer` to understand implementation
   - Output: `thoughts/shared/research/YYYY-MM-DD-topic.md`

2. **Planning Phase** (`/cl:create_plan_nt`)
   - Interactive planning with codebase research
   - Uses specialized agents for context gathering
   - Output: `thoughts/shared/plans/YYYY-MM-DD-feature.md`

3. **Iteration Phase** (`/cl:iterate_plan_nt`)
   - Updates existing plans based on feedback
   - Re-researches when changes require new understanding
   - Updates same plan file

4. **Implementation Phase** (`/cl:implement_plan`)
   - Reads plan completely
   - Implements phase by phase
   - Runs automated verification (make check, tests)
   - Pauses for manual verification between phases
   - Updates checkboxes in plan as work completes

5. **Commit Phase** (`/cl:commit`, `/cl:describe_pr`)
   - Reviews session changes
   - Creates focused commits
   - Generates PR descriptions from diff

### Key Design Principles

1. **Documentarians, Not Critics** - All agents describe what exists without suggesting improvements
2. **Parallel Research** - Multiple agents run concurrently for efficiency
3. **Human-in-the-Loop** - Pauses for manual verification at phase boundaries
4. **Structured Output** - All artifacts go to `thoughts/shared/` with consistent naming

## Code References

- `.claude/commands/cl/research_codebase_nt.md` - Main research command
- `.claude/commands/cl/create_plan_nt.md` - Plan creation with Opus model
- `.claude/commands/cl/implement_plan.md` - Plan execution with verification
- `.claude/agents/cl/codebase-locator.md` - File location specialist
- `.claude/agents/cl/codebase-analyzer.md` - Implementation analysis specialist

## Architecture Documentation

The workflow follows a "research → plan → implement" pattern common in structured development:

```
User Request
    ↓
Research Phase (understand what exists)
    ↓
Planning Phase (design solution interactively)
    ↓
Implementation Phase (execute with gates)
    ↓
Commit Phase (clean git history)
```

Each phase produces artifacts in `thoughts/shared/` for traceability.

## Related Research

- Created: `.claude/README.md` - User-facing documentation of the workflow

## Open Questions

None - workflow is well-documented in command/agent files.
