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

For complex features, generate a plan first.

### Step 1: Generate Plan

```bash
# Create tasks and generate implementation plan
descartes swarm \
    --prd ./docs/complex-feature.md \
    --tag complex \
    --plan-only \
    --output ./docs/IMPLEMENTATION_PLAN.md
```

### Step 2: Review and Refine

Open `IMPLEMENTATION_PLAN.md` and review:
- Task breakdown
- Implementation approach
- Risk areas
- Dependencies

Make manual edits if needed.

### Step 3: Execute with Plan

```bash
descartes swarm \
    --scud-tag complex \
    --plan ./docs/IMPLEMENTATION_PLAN.md \
    --verify "npm test"
```

The plan document provides context for each task.

## Workflow 3: Incremental Development

Work on tasks one at a time with manual control.

### Step 1: See What's Ready

```bash
scud next --tag my-feature
```

### Step 2: Execute Single Task

```bash
# Run one task
descartes run --task TASK-001

# Or spawn specific category
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

### Use in Ralph Loop

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
descartes transcripts --last 5
descartes show <id>
```

### 6. Iterate on Specs

If tasks fail repeatedly, improve your spec files:
- Add more context
- Clarify requirements
- Include examples
