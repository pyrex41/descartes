# Claude Code Workflow (`/cl:*`)

A structured workflow for AI-assisted development with research, planning, and implementation phases.

## Quick Start

```
/cl:research_codebase_nt  →  Understand the codebase
/cl:create_plan_nt        →  Create implementation plan
/cl:implement_plan        →  Execute the plan
/cl:commit                →  Commit your changes
```

## Workflow Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        RESEARCH PHASE                            │
│  /cl:research_codebase_nt "How does X work?"                    │
│  → Spawns specialized agents to explore codebase                 │
│  → Produces research doc in thoughts/shared/research/            │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                        PLANNING PHASE                            │
│  /cl:create_plan_nt "Add feature Y based on research"           │
│  → Interactive planning with codebase research                   │
│  → Produces plan in thoughts/shared/plans/                       │
│                                                                  │
│  /cl:iterate_plan_nt thoughts/shared/plans/YYYY-MM-DD-plan.md   │
│  → Refine existing plans based on feedback                       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                      IMPLEMENTATION PHASE                        │
│  /cl:implement_plan thoughts/shared/plans/YYYY-MM-DD-plan.md    │
│  → Executes plan phase by phase                                  │
│  → Runs automated verification after each phase                  │
│  → Pauses for manual verification when needed                    │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                        COMMIT PHASE                              │
│  /cl:commit                                                      │
│  → Reviews changes and creates focused commits                   │
│                                                                  │
│  /cl:describe_pr                                                 │
│  → Generates PR description from changes                         │
└─────────────────────────────────────────────────────────────────┘
```

## Commands

| Command | Purpose |
|---------|---------|
| `/cl:research_codebase_nt` | Document how existing code works (read-only) |
| `/cl:create_plan_nt` | Create detailed implementation plan with research |
| `/cl:iterate_plan_nt` | Update existing plans based on feedback |
| `/cl:implement_plan` | Execute an approved plan step-by-step |
| `/cl:commit` | Create git commits for session changes |
| `/cl:describe_pr` | Generate PR description from diff |

**Note**: The `_nt` suffix means "no thoughts directory prompting" - streamlined for direct use.

## Specialized Agents

The commands spawn specialized sub-agents for parallel research:

### Codebase Research
| Agent | Purpose |
|-------|---------|
| `codebase-locator` | Find WHERE files and components live |
| `codebase-analyzer` | Understand HOW code works (with file:line refs) |
| `codebase-pattern-finder` | Find existing patterns to model after |

### Documentation Research
| Agent | Purpose |
|-------|---------|
| `thoughts-locator` | Find relevant docs in `thoughts/` directory |
| `thoughts-analyzer` | Extract key insights from thought documents |

### External Research
| Agent | Purpose |
|-------|---------|
| `web-search-researcher` | Search web for docs, APIs, best practices |

## Example Usage

### 1. Research Before Implementing
```
> /cl:research_codebase_nt How does the authentication system work?

[Agent spawns codebase-locator + codebase-analyzer in parallel]
[Produces: thoughts/shared/research/2025-01-15-authentication-flow.md]
```

### 2. Create a Plan
```
> /cl:create_plan_nt Add OAuth2 support based on the auth research

[Interactive planning session]
[Produces: thoughts/shared/plans/2025-01-15-oauth2-support.md]
```

### 3. Implement the Plan
```
> /cl:implement_plan thoughts/shared/plans/2025-01-15-oauth2-support.md

Phase 1 Complete - Ready for Manual Verification

Automated verification passed:
- [x] cargo check passes
- [x] cargo test passes

Please perform manual verification:
- [ ] OAuth flow works in browser
- [ ] Token refresh works correctly

Let me know when manual testing is complete.
```

### 4. Commit Changes
```
> /cl:commit

I plan to create 2 commits:
1. "feat: Add OAuth2 provider interface" (3 files)
2. "feat: Implement Google OAuth2 provider" (5 files)

Shall I proceed?
```

## Directory Structure

```
.claude/
├── agents/cl/           # Specialized sub-agents
│   ├── codebase-analyzer.md
│   ├── codebase-locator.md
│   ├── codebase-pattern-finder.md
│   ├── thoughts-analyzer.md
│   ├── thoughts-locator.md
│   └── web-search-researcher.md
└── commands/cl/         # Workflow commands
    ├── commit.md
    ├── create_plan_nt.md
    ├── describe_pr.md
    ├── implement_plan.md
    ├── iterate_plan_nt.md
    └── research_codebase_nt.md

thoughts/shared/         # Output directory
├── research/            # Research documents
├── plans/               # Implementation plans
├── tickets/             # Ticket documentation
└── prs/                 # PR descriptions
```

## Key Principles

1. **Research First**: Understand before you build
2. **Plan Explicitly**: Write detailed plans with success criteria
3. **Phase-by-Phase**: Implement incrementally with verification gates
4. **Human-in-the-Loop**: Pause for manual testing between phases
5. **Document Everything**: All artifacts go to `thoughts/shared/`
