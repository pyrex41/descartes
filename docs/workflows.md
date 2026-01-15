[Home](index.md) | [Getting Started](getting-started.md) | [Configuration](configuration.md) | [Swarm](swarm.md) | [Harnesses](harnesses.md) | [Workflows](workflows.md)

---

# Common Workflows

This guide covers common usage patterns for Descartes.

## Workflow 1: PRD to Implementation

The most common workflow: take a Product Requirements Document and implement it.

### Step 1: Write a PRD

Create a markdown file describing what you want to build:

```markdown
# Feature: User Authentication

## Overview
Add email/password authentication to the application.

## Requirements
1. User registration with email and password
2. Password hashing with bcrypt
3. JWT token generation on login
4. Protected route middleware
5. Password reset via email

## Technical Notes
- Use existing User model
- Store tokens in Redis
- Follow existing API patterns
```

### Step 2: Initialize and Execute

```bash
# One command does it all:
descartes swarm \
    --prd ./docs/auth-prd.md \
    --tag auth-feature \
    --verify "cargo test"
```

This will:
1. Parse the PRD into SCUD tasks
2. Expand complex tasks into subtasks
3. Validate dependencies
4. Execute tasks in dependency order
5. Run tests after each wave

### Step 3: Monitor Progress

```bash
# Check task status
scud list --tag auth-feature

# See execution waves
scud waves --tag auth-feature

# View statistics
scud stats --tag auth-feature
```

## Workflow 2: Plan-Then-Build

For complex features, create a plan document first, then execute with it.

### Step 1: Create Implementation Plan

Write an implementation plan manually or use AI assistance:

```markdown
# Complex Feature Implementation Plan

## Phase 1: Database Schema
- Add user_preferences table
- Create migration

## Phase 2: API Layer
- Add GET/POST /api/preferences endpoints
- Implement validation

## Phase 3: UI Integration
- Add preferences panel component
- Wire up API calls
```

Save to `./docs/IMPLEMENTATION_PLAN.md`.

### Step 2: Generate Tasks from PRD

```bash
# Generate SCUD tasks from PRD (parse → expand → check-deps)
scud generate ./docs/complex-feature.md --tag complex

# Review the generated tasks
scud list --tag complex
scud waves --tag complex
```

### Step 3: Execute with Plan Context

```bash
descartes swarm \
    --scud-tag complex \
    --plan ./docs/IMPLEMENTATION_PLAN.md \
    --verify "npm test"
```

The plan document provides additional context for each task.

## Workflow 3: Incremental Development

Work on tasks one at a time with manual control.

### Step 1: See What's Ready

```bash
scud next --tag my-feature
```

### Step 2: Execute Single Task

```bash
# Spawn a builder agent for a specific task
descartes spawn builder "Implement TASK-001: Add user validation"
```

### Step 3: Validate and Continue

```bash
# Run tests
cargo test

# Mark complete
scud set-status TASK-001 done

# Move to next
scud next --tag my-feature
```

## Workflow 4: Fast Iteration

Use fast models for quick iteration cycles.

```bash
# Force fast harness for all tasks
descartes swarm \
    --scud-tag prototype \
    --harness opencode \
    --model xai/grok-code-fast-1 \
    --no-validate  # Skip validation for speed
```

Good for:
- Prototyping
- Exploratory coding
- Non-critical changes

## Workflow 5: High-Quality Implementation

Use smart models with thorough validation.

```bash
descartes swarm \
    --scud-tag production-feature \
    --harness claude-code \
    --model opus \
    --verify "cargo test && cargo clippy -- -D warnings && cargo fmt --check" \
    --round-size 1  # One task at a time for careful review
```

Good for:
- Production code
- Security-sensitive features
- Complex refactoring

## Workflow 6: Parallel Research

Use fast parallel agents for codebase exploration.

```bash
# Spawn multiple searchers in parallel
descartes spawn searcher "Find all authentication-related code"
descartes spawn searcher "Find all database migration files"
descartes spawn analyzer "Analyze the user model structure"
```

Or configure high parallelism:

```bash
descartes swarm \
    --scud-tag research \
    --round-size 10 \
    --harness opencode
```

## Workflow 7: Recovery from Failures

When tasks fail, recover gracefully.

### Check Failed Tasks

```bash
scud list --tag my-feature --status failed
```

### Retry Failed Tasks

```bash
# Reset failed tasks to pending
scud set-status TASK-003 pending
scud set-status TASK-004 pending

# Re-run
descartes swarm --scud-tag my-feature
```

### Debug with Transcripts

```bash
# Find the failed task's transcript
descartes transcripts | grep TASK-003

# View what happened
descartes show <transcript-id>
```

## Workflow 8: Custom Validation

Add project-specific validation.

### Create Validation Script

```bash
#!/bin/bash
# scripts/validate.sh

set -e

echo "Running tests..."
cargo test

echo "Running clippy..."
cargo clippy -- -D warnings

echo "Checking formatting..."
cargo fmt --check

echo "Running security audit..."
cargo audit

echo "All validations passed!"
```

### Use in Swarm Loop

```bash
descartes swarm \
    --scud-tag secure-feature \
    --verify "./scripts/validate.sh"
```

## Workflow 9: Multi-Project Orchestration

Work across multiple related projects.

### Project A: Backend

```bash
cd backend/
descartes swarm --prd ./docs/api-prd.md --tag api-v2
```

### Project B: Frontend

```bash
cd frontend/
descartes swarm --prd ./docs/ui-prd.md --tag ui-v2
```

### Coordinate via Specs

Share spec files between projects:

```bash
descartes swarm \
    --scud-tag api-v2 \
    --spec-file ../shared/API_CONTRACT.md
```

## Workflow 10: Interactive Mode

Run Descartes interactively for manual control and exploration.

### Starting Interactive Mode

```bash
descartes interactive
```

### Interactive Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/pause` | Pause current execution |
| `/resume` | Resume paused execution |
| `/cancel` | Cancel current task |
| `/scud` | Show SCUD task status |
| `/waves` | Display wave structure |
| `/diff` | Show pending changes |
| `/context` | Display current context |
| `/skill <name>` | Run a skill by name |

### Running Skills

Skills are reusable prompts for common tasks:

```bash
# In interactive mode
/skill research "authentication patterns"
/skill create_plan
/skill commit
```

### Built-in Skills

| Skill | Aliases | Description |
|-------|---------|-------------|
| `research` | `r` | Research a topic or codebase area |
| `create_plan` | `plan`, `cp` | Create implementation plan |
| `implement_plan` | `implement`, `ip` | Implement from plan |
| `commit` | `c` | Create git commit |
| `review` | `rv` | Review code changes |
| `fix` | `f` | Fix an issue |
| `test` | `t` | Run tests and fix failures |

### Custom Skills

Add custom skills in `.descartes/skills/`:

```markdown
# .descartes/skills/security-audit.md

Review the following code for security vulnerabilities:
- SQL injection
- XSS
- Authentication bypass
- Sensitive data exposure

Files to review: {{files}}
```

Use with `/skill security-audit`.

### Cross-Tool Compatibility

Descartes can load skills from other tools:
- `.claude/commands/` - Claude Code commands
- `.opencode/skill/` - OpenCode skills
- `.codex/skills/` - Codex skills

This means your existing Claude Code slash commands work in Descartes.

## Tips for Success

### 1. Start Small

Begin with a small feature to learn the workflow:

```bash
descartes swarm --prd ./docs/small-feature.md --tag test
```

### 2. Use Dry Run

Preview before executing:

```bash
descartes swarm --scud-tag feature --dry-run
```

### 3. Keep PRDs Focused

One feature per PRD works better than monolithic documents.

### 4. Trust the Graph

Let SCUD manage dependencies. Don't manually order tasks.

### 5. Review Transcripts

When things go wrong, transcripts tell the full story:

```bash
# List today's transcripts
descartes transcripts --today

# View a specific transcript
descartes show <session-id>
```

### 6. Iterate on Specs

If tasks fail repeatedly, improve your spec files:
- Add more context
- Clarify requirements
- Include examples

---

← [Harnesses](harnesses.md) | [Back to Home](index.md)
