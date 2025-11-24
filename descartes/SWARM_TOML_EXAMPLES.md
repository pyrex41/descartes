# Swarm.toml - Practical Examples

**Status**: Reference Examples for Phase 2 Implementation
**Date**: November 23, 2025

---

## Table of Contents

1. [Simple Linear Workflow](#simple-linear-workflow)
2. [Code Review Workflow](#code-review-workflow)
3. [Multi-Agent Implementation](#multi-agent-implementation)
4. [Hierarchical Workflow](#hierarchical-workflow)
5. [Parallel Processing](#parallel-processing)
6. [Error Handling & Recovery](#error-handling--recovery)

---

## Example 1: Simple Linear Workflow

**Use Case**: Basic approval/rejection process

**File**: `examples/swarm_toml/simple_approval.toml`

```toml
[metadata]
version = "1.0"
name = "Simple Approval Workflow"
description = "Basic approval request with two outcomes"
author = "Descartes Team"

[agents.reviewer]
model = "claude-3-haiku"
max_tokens = 1000
temperature = 0.5
tags = ["review"]

[[workflows]]
name = "approval_request"
description = "Simple approval request workflow"

[workflows.metadata]
initial_state = "Submitted"
completion_timeout_seconds = 86400  # 24 hours

[workflows.states]

[workflows.states.Submitted]
description = "Request submitted for approval"
entry_actions = ["log_submission"]
handlers = [
    { event = "review", target = "Under Review", guards = [] },
    { event = "timeout", target = "Expired", guards = [] },
]

[workflows.states."Under Review"]
description = "Being reviewed by human"
agents = ["reviewer"]
entry_actions = ["notify_reviewer"]
handlers = [
    { event = "approve", target = "Approved", guards = [] },
    { event = "reject", target = "Rejected", guards = [] },
    { event = "request_info", target = "Information Requested", guards = [] },
]

[workflows.states."Information Requested"]
description = "Reviewer requested more information"
entry_actions = ["notify_requester"]
handlers = [
    { event = "resubmit", target = "Submitted", guards = [] },
    { event = "cancel", target = "Cancelled", guards = [] },
]

[workflows.states.Approved]
description = "Request approved"
entry_actions = ["process_approval", "notify_approver"]
terminal = true

[workflows.states.Rejected]
description = "Request rejected"
entry_actions = ["notify_requester"]
terminal = true

[workflows.states.Expired]
description = "Request expired due to timeout"
terminal = true

[workflows.states.Cancelled]
description = "Request cancelled"
terminal = true
```

**Generated State Machine**:
```
Submitted -> Under Review -> Approved (terminal)
         -> Under Review -> Rejected (terminal)
         -> Under Review -> Information Requested -> Submitted
         -> Expired (terminal)
```

---

## Example 2: Code Review Workflow

**Use Case**: Comprehensive code review with multiple reviewers

**File**: `examples/swarm_toml/code_review.toml`

```toml
[metadata]
version = "1.0"
name = "Code Review Workflow"
description = "Multi-reviewer code review with automated checks"
author = "Descartes Team"

[agents.analyzer]
model = "claude-3-sonnet"
max_tokens = 2000
temperature = 0.3
tags = ["analysis", "static-analysis"]

[agents.security_reviewer]
model = "gpt-4"
max_tokens = 3000
temperature = 0.4
tags = ["security", "review"]

[agents.code_reviewer]
model = "claude-3-opus"
max_tokens = 4000
temperature = 0.5
tags = ["code-review", "architecture"]

[resources.github_api]
type = "http"
endpoint = "https://api.github.com"
auth_required = true
secret_key = "GITHUB_TOKEN"

[resources.slack]
type = "webhook"
endpoint = "${SLACK_WEBHOOK_URL}"
description = "Slack notifications"

[[workflows]]
name = "code_review"
description = "Full code review and merge workflow"

[workflows.metadata]
initial_state = "Submitted"
completion_timeout_seconds = 604800  # 7 days
max_retries = 3
retry_backoff_seconds = 300

[workflows.states]

[workflows.states.Submitted]
description = "Pull request submitted"
entry_actions = [
    "log_pr_submission",
    "notify_maintainer",
    "assign_reviewers",
]
handlers = [
    { event = "start_analysis", target = "Analyzing", guards = [] },
    { event = "skip_analysis", target = "Waiting for Reviews", guards = [] },
]

[workflows.states.Analyzing]
description = "Automated code analysis in progress"
agents = ["analyzer", "security_reviewer"]
entry_actions = ["start_static_analysis", "start_security_scan"]
handlers = [
    { event = "analysis_pass", target = "Waiting for Reviews", guards = ["no_critical_issues"] },
    { event = "analysis_fail", target = "Analysis Failed", guards = [] },
]

[workflows.states."Waiting for Reviews"]
description = "Awaiting human code reviews"
agents = ["code_reviewer"]
timeout_seconds = 432000  # 5 days
timeout_target = "Review Timeout"
entry_actions = ["notify_reviewers", "start_review_timer"]
handlers = [
    { event = "all_approved", target = "Approved", guards = ["all_reviewers_approved"] },
    { event = "changes_requested", target = "Changes Requested", guards = [] },
    { event = "conflict", target = "Merge Conflict", guards = [] },
]

[workflows.states."Changes Requested"]
description = "Reviewers requested changes"
entry_actions = ["notify_developer", "notify_reviewers"]
handlers = [
    { event = "changes_pushed", target = "Submitted", guards = [] },
    { event = "abandon", target = "Abandoned", guards = [] },
    { event = "dismiss_review", target = "Waiting for Reviews", guards = ["author_override"] },
]

[workflows.states.Approved]
description = "All reviews approved"
entry_actions = [
    "run_final_tests",
    "verify_ci_passing",
    "notify_reviewers",
]
handlers = [
    { event = "merge", target = "Merging", guards = ["ci_passing", "no_conflicts"] },
    { event = "cancel", target = "Cancelled", guards = [] },
]

[workflows.states.Merging]
description = "Executing merge operation"
entry_actions = ["execute_merge", "push_to_main"]
handlers = [
    { event = "merge_success", target = "Merged", guards = [] },
    { event = "merge_conflict", target = "Merge Conflict", guards = [] },
]

[workflows.states.Merged]
description = "Successfully merged"
entry_actions = [
    "close_pr",
    "notify_team",
    "trigger_deployment",
]
terminal = true

[workflows.states."Analysis Failed"]
description = "Static analysis found critical issues"
entry_actions = ["notify_developer", "create_issues"]
terminal = true

[workflows.states."Merge Conflict"]
description = "Merge conflict detected"
entry_actions = ["notify_developer"]
handlers = [
    { event = "conflict_resolved", target = "Submitted", guards = [] },
]

[workflows.states."Review Timeout"]
description = "Timeout waiting for reviews"
entry_actions = ["notify_maintainer"]
terminal = true

[workflows.states.Abandoned]
description = "PR abandoned"
terminal = true

[workflows.states.Cancelled]
description = "Merge cancelled"
terminal = true

# Guard conditions
[workflows.guards]
no_critical_issues = "security_scan.critical_count == 0"
all_reviewers_approved = "reviews.approved_count >= 2"
author_override = "pr.author.is_maintainer"
ci_passing = "ci_status == 'SUCCESS'"
no_conflicts = "!has_merge_conflicts(main)"
```

---

## Example 3: Multi-Agent Implementation

**Use Case**: Collaborative feature development

**File**: `examples/swarm_toml/implementation.toml`

```toml
[metadata]
version = "1.0"
name = "Feature Implementation Workflow"
description = "Multi-phase feature development"
author = "Descartes Team"

[agents.architect]
model = "claude-3-opus"
max_tokens = 8000
temperature = 0.7
tags = ["planning", "architecture"]

[agents.developer]
model = "claude-3-sonnet"
max_tokens = 4000
temperature = 0.5
tags = ["coding", "implementation"]

[agents.tester]
model = "claude-3-haiku"
max_tokens = 2000
temperature = 0.2
tags = ["testing", "qa"]

[agents.documenter]
model = "claude-3-sonnet"
max_tokens = 3000
temperature = 0.6
tags = ["documentation"]

[[workflows]]
name = "feature_implementation"
description = "Complete feature development workflow"

[workflows.metadata]
initial_state = "Planning"
completion_timeout_seconds = 2592000  # 30 days

[workflows.states]

[workflows.states.Planning]
description = "Architectural planning phase"
agents = ["architect"]
entry_actions = [
    "review_requirements",
    "create_architecture_doc",
    "identify_dependencies",
]
handlers = [
    { event = "plan_approved", target = "Implementation", guards = ["plan_reviewed"] },
    { event = "needs_revision", target = "Planning", guards = [] },
]

[workflows.states.Implementation]
description = "Feature implementation"
agents = ["developer"]
entry_actions = [
    "create_feature_branch",
    "setup_environment",
]
handlers = [
    { event = "implementation_done", target = "Testing", guards = ["code_compiles", "basic_checks_pass"] },
    { event = "needs_design_update", target = "Planning", guards = [] },
]

[workflows.states.Testing]
description = "Comprehensive testing phase"
agents = ["tester"]
entry_actions = [
    "run_unit_tests",
    "run_integration_tests",
    "run_e2e_tests",
]
parallel_execution = true
handlers = [
    { event = "all_tests_pass", target = "Documentation", guards = ["test_coverage_sufficient"] },
    { event = "tests_failing", target = "Implementation", guards = [] },
    { event = "design_issues_found", target = "Planning", guards = [] },
]

[workflows.states.Documentation]
description = "API and user documentation"
agents = ["documenter"]
entry_actions = [
    "generate_api_docs",
    "write_user_guide",
    "update_changelog",
]
handlers = [
    { event = "docs_complete", target = "Ready for Merge", guards = [] },
]

[workflows.states."Ready for Merge"]
description = "Feature ready for integration"
entry_actions = [
    "create_pull_request",
    "run_final_checks",
]
handlers = [
    { event = "approved", target = "Merged", guards = ["all_checks_pass"] },
    { event = "needs_changes", target = "Implementation", guards = [] },
]

[workflows.states.Merged]
description = "Successfully merged to main"
entry_actions = [
    "merge_to_main",
    "trigger_deployment",
    "notify_stakeholders",
]
terminal = true

# Guards
[workflows.guards]
plan_reviewed = "architect.approval == true"
code_compiles = "build_result.success == true"
basic_checks_pass = "lint.errors == 0"
test_coverage_sufficient = "coverage.percentage >= 80"
all_checks_pass = "ci_passing and code_reviewed"
```

---

## Example 4: Hierarchical Workflow

**Use Case**: Complex nested state management

**File**: `examples/swarm_toml/hierarchical.toml`

```toml
[metadata]
version = "1.0"
name = "Hierarchical Development Workflow"

[agents.all]
model = "claude-3-sonnet"
max_tokens = 4000
temperature = 0.5

[[workflows]]
name = "hierarchical_dev"
description = "Workflow with hierarchical states"

[workflows.metadata]
initial_state = "Planning"

[workflows.states]

# Root states
[workflows.states.Planning]
description = "Planning phase"
parent = null

[workflows.states.InProgress]
description = "Active development"
parent = null
entry_actions = ["start_timer"]
exit_actions = ["stop_timer"]

# Child states of InProgress
[workflows.states.Coding]
description = "Implementation in progress"
parent = "InProgress"

[workflows.states.CodeReview]
description = "Code being reviewed"
parent = "InProgress"

[workflows.states.Testing]
description = "Testing phase"
parent = "InProgress"

[workflows.states.Blocked]
description = "Work blocked by external issue"
parent = null
entry_actions = ["notify_team", "record_blocker"]

[workflows.states.Complete]
description = "Work complete"
parent = null
terminal = true

# Transitions
[workflows.states.Planning]
handlers = [
    { event = "plan_ready", target = "Coding", guards = [] },
]

[workflows.states.Coding]
handlers = [
    { event = "code_ready", target = "CodeReview", guards = [] },
    { event = "blocked", target = "Blocked", guards = [] },
]

[workflows.states.CodeReview]
handlers = [
    { event = "approved", target = "Testing", guards = [] },
    { event = "needs_revision", target = "Coding", guards = [] },
]

[workflows.states.Testing]
handlers = [
    { event = "all_pass", target = "Complete", guards = [] },
    { event = "failures", target = "Coding", guards = [] },
]

[workflows.states.Blocked]
handlers = [
    { event = "resolved", target = "Coding", guards = [] },
]
```

---

## Example 5: Parallel Processing

**Use Case**: Independent agents working simultaneously

**File**: `examples/swarm_toml/parallel.toml`

```toml
[metadata]
version = "1.0"
name = "Parallel Processing Workflow"

[agents.security]
model = "gpt-4"
max_tokens = 2000
temperature = 0.3

[agents.performance]
model = "claude-3-sonnet"
max_tokens = 2000
temperature = 0.3

[agents.docs]
model = "claude-3-sonnet"
max_tokens = 2000
temperature = 0.5

[[workflows]]
name = "parallel_review"
description = "Multiple agents review in parallel"

[workflows.metadata]
initial_state = "Submitted"

[workflows.states]

[workflows.states.Submitted]
description = "Code submitted"
handlers = [
    { event = "start_reviews", target = "Multi Review", guards = [] },
]

[workflows.states."Multi Review"]
description = "Multiple parallel reviews"
agents = ["security", "performance", "docs"]
parallel_execution = true
entry_actions = ["dispatch_to_agents"]
handlers = [
    { event = "all_complete", target = "Complete", guards = ["all_passed"] },
    { event = "failures", target = "Failed", guards = [] },
]

[workflows.states.Complete]
description = "All reviews passed"
terminal = true

[workflows.states.Failed]
description = "Review failed"
terminal = true

[workflows.guards]
all_passed = "security.approved and performance.approved and docs.approved"
```

---

## Example 6: Error Handling & Recovery

**Use Case**: Robust error handling with retries

**File**: `examples/swarm_toml/resilient.toml`

```toml
[metadata]
version = "1.0"
name = "Resilient Workflow with Error Recovery"

[agents.worker]
model = "claude-3-sonnet"
max_tokens = 4000
temperature = 0.5

[[workflows]]
name = "resilient_task"
description = "Task with error handling and retries"

[workflows.metadata]
initial_state = "Ready"
max_retries = 3
retry_backoff_seconds = 60

[workflows.states]

[workflows.states.Ready]
description = "Ready to execute"
handlers = [
    { event = "execute", target = "Executing", guards = [] },
]

[workflows.states.Executing]
description = "Task execution in progress"
agents = ["worker"]
entry_actions = ["start_execution"]
handlers = [
    { event = "success", target = "Complete", guards = [] },
    { event = "failure", target = "Error", guards = [] },
    { event = "timeout", target = "Timeout", guards = [] },
]

[workflows.states.Error]
description = "Execution failed"
entry_actions = ["log_error", "notify_admin"]
handlers = [
    { event = "retry", target = "Ready", guards = ["retries_remaining"] },
    { event = "abandon", target = "Failed", guards = [] },
]

[workflows.states.Timeout]
description = "Execution timed out"
entry_actions = ["cancel_task", "notify_admin"]
handlers = [
    { event = "retry", target = "Ready", guards = ["retries_remaining"] },
    { event = "escalate", target = "Escalated", guards = [] },
]

[workflows.states.Complete]
description = "Task completed successfully"
terminal = true

[workflows.states.Failed]
description = "Task failed - max retries exceeded"
terminal = true

[workflows.states.Escalated]
description = "Task escalated to manual handling"
terminal = true

[workflows.guards]
retries_remaining = "retry_count < max_retries"
```

---

## Validation Examples

### Example: Valid Workflow (All Checks Pass)

```toml
# This workflow passes all validation checks
[metadata]
version = "1.0"
name = "Valid Workflow"

[agents.reviewer]
model = "claude-3-haiku"
max_tokens = 1000
temperature = 0.5

[[workflows]]
name = "valid"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Starting"
handlers = [
    { event = "go", target = "End", guards = [] },
]

[workflows.states.End]
description = "Ending"
terminal = true
```

**Validation Result**: ✅ PASS
- Initial state exists
- All referenced states exist
- No cycles
- All states reachable
- Terminal state is reachable

---

### Example: Invalid Workflow (Multiple Errors)

```toml
# This workflow has multiple validation errors
[[workflows]]
name = "invalid"

[workflows.metadata]
initial_state = "Start"  # ERROR: State doesn't exist

[workflows.states]

[workflows.states.State1]
handlers = [
    { event = "go", target = "NonExistent", guards = [] },  # ERROR: Target doesn't exist
]

[workflows.states.State2]
handlers = [
    { event = "cycle", target = "State1", guards = [] },
]

# State1 -> State2 -> State1 creates a cycle
# ERROR: Cycle detected
```

**Validation Result**: ❌ FAIL
- Initial state "Start" not found
- Target state "NonExistent" not found
- Cycle detected (State1 -> State2 -> State1)
- State "State2" unreachable from initial state

---

## Running These Examples

### Parse Example
```bash
# Create example file
cp examples/swarm_toml/code_review.toml my_workflow.toml

# Parse and validate
cargo run -- validate my_workflow.toml

# Generate code
cargo run -- generate my_workflow.toml --output src/generated/

# Generate diagram
cargo run -- diagram my_workflow.toml --output docs/workflow.md
```

### Execute Example
```bash
# Execute workflow
cargo run -- execute my_workflow.toml --task PR-12345 --data '{}'

# Monitor execution
cargo run -- monitor my_workflow.toml --task PR-12345

# List running workflows
cargo run -- list-workflows
```

---

## Tips & Tricks

### 1. Reusable Agent Definitions

Define agents once at top level, reference in multiple workflows:

```toml
[agents.gpt4]
model = "gpt-4"
max_tokens = 2000

[agents.claude3]
model = "claude-3-opus"
max_tokens = 4000

# Use in multiple workflows
[[workflows]]
[workflows.states.Analysis]
agents = ["gpt4", "claude3"]  # Both participate
```

### 2. Common Entry/Exit Actions

```toml
[workflows.states.CriticalState]
entry_actions = [
    "log_state_entry",
    "backup_context",
    "notify_team",
]
exit_actions = [
    "log_state_exit",
    "persist_state",
]
```

### 3. Timeouts and Recovery

```toml
[workflows.states.WaitingForApproval]
timeout_seconds = 604800  # 7 days
timeout_target = "TimedOut"

[workflows.states.TimedOut]
entry_actions = ["escalate_to_manager"]
handlers = [
    { event = "manual_override", target = "Approved", guards = [] },
]
```

### 4. Conditional Transitions

```toml
[workflows.states.Testing]
handlers = [
    { event = "complete", target = "Approved", guards = ["all_tests_pass", "coverage_sufficient"] },
    { event = "complete", target = "NeedsWork", guards = ["!all_tests_pass"] },
]
```

---

## Common Patterns

### Pattern 1: Retry Loop
```
Attempt -> Fail -> Wait -> Attempt (cycle back)
       -> Success -> Complete
```

### Pattern 2: Approval Chain
```
Submitted -> Manager Review -> CTO Review -> Approved
         -> Rejected (terminal)
```

### Pattern 3: Multi-Phase Work
```
Phase1 -> Phase2 -> Phase3 -> Complete
   |        |         |
   +--------+-------- Error -> Manual Review
```

### Pattern 4: Parallel Work with Merge
```
Split -> [Agent1]  -\
      -> [Agent2]  ---> Merge -> Continue
      -> [Agent3]  -/
```

---

## References

- [Swarm.toml Schema](./SWARM_TOML_SCHEMA.md)
- [Integration Plan](./INTEGRATION_PLAN.md)
- [State Machine Evaluation](./STATE_MACHINE_EVALUATION.md)
- [Statig Documentation](https://github.com/mdeloof/statig)

---

**Status**: Ready for use in Phase 2 implementation
**Last Updated**: November 23, 2025
