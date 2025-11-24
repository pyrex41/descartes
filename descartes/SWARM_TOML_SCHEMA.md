# Swarm.toml Schema & Format Specification

**Version**: 1.0
**Status**: Reference Implementation
**Date**: November 23, 2025

---

## Overview

Swarm.toml is a declarative configuration format for defining multi-agent workflows in Descartes. It describes:

1. **State Machines** - Agent workflow states and transitions
2. **Agents** - Which agents participate in each state
3. **Context** - Data requirements and constraints
4. **Handlers** - Actions executed on state transitions
5. **Contracts** - Input/output specifications
6. **Resources** - External services and API dependencies

---

## File Structure

```toml
# metadata.toml section
[metadata]
version = "1.0"
name = "Code Review and Merge Workflow"
description = "Automated code review, testing, and merge orchestration"
author = "Descartes Team"
created = "2025-11-23"

# Global agent definitions
[agents]
[agents.architect]
model = "claude-3-opus"  # High-capability planning
max_tokens = 8000
temperature = 0.7
tags = ["planning", "design"]

[agents.coder]
model = "claude-3-sonnet"
max_tokens = 4000
temperature = 0.5
tags = ["implementation"]

[agents.tester]
model = "claude-3-haiku"  # Lightweight testing
max_tokens = 2000
temperature = 0.2
tags = ["testing", "verification"]

[agents.reviewer]
model = "gpt-4"
max_tokens = 3000
temperature = 0.6
tags = ["review", "feedback"]

# Global resource definitions
[resources]

[resources.github_api]
type = "http"
endpoint = "https://api.github.com"
auth_required = true
secret_key = "GITHUB_TOKEN"

[resources.slack]
type = "webhook"
endpoint = "${SLACK_WEBHOOK_URL}"
description = "Slack notifications for workflow events"

# Workflow definitions
[[workflows]]
name = "code_review"
description = "Primary code review and merge workflow"

[workflows.metadata]
initial_state = "Submitted"
completion_timeout_seconds = 3600
max_retries = 3
retry_backoff_seconds = 60

# State definitions within workflow
[workflows.states]

[workflows.states.Submitted]
description = "Code change submitted for review"
entry_actions = ["log_submission", "assign_reviewers"]
terminal = false
parent = null  # Root state

handlers = [
    { event = "analyze", target = "Analyzing", guards = [] },
    { event = "timeout", target = "TimedOut", guards = [] },
]

[workflows.states.Analyzing]
description = "Automated analysis in progress"
entry_actions = ["run_static_analysis", "run_security_scan"]
agents = ["architect"]
parent = null
terminal = false

handlers = [
    { event = "analysis_pass", target = "ReadyForReview", guards = ["checks_passed"] },
    { event = "analysis_fail", target = "AnalysisFailed", guards = [] },
]

[workflows.states.ReadyForReview]
description = "Automated checks passed, awaiting human review"
agents = ["reviewer"]
parent = null
terminal = false

handlers = [
    { event = "review_approved", target = "Approved", guards = ["all_reviewers_approved"] },
    { event = "review_rejected", target = "ChangesRequested", guards = [] },
    { event = "timeout", target = "TimedOut", guards = [] },
]

[workflows.states.ChangesRequested]
description = "Developer must address feedback"
entry_actions = ["notify_developer"]
terminal = false

handlers = [
    { event = "changes_pushed", target = "Submitted", guards = [] },
    { event = "abandoned", target = "Abandoned", guards = [] },
]

[workflows.states.Approved]
description = "All reviews approved, ready to merge"
entry_actions = ["run_final_tests", "prepare_merge"]
agents = ["tester"]
terminal = false
parallel_execution = false

handlers = [
    { event = "merge_complete", target = "Merged", guards = ["tests_pass"] },
    { event = "merge_failed", target = "MergeFailed", guards = [] },
]

[workflows.states.Merged]
description = "Successfully merged to main branch"
entry_actions = ["create_deployment", "notify_team"]
terminal = true

[workflows.states.AnalysisFailed]
description = "Automated analysis found issues"
entry_actions = ["notify_developer"]
terminal = true

[workflows.states.TimedOut]
description = "Workflow timed out waiting for response"
terminal = true

[workflows.states.Abandoned]
description = "PR abandoned by developer"
terminal = true

[workflows.states.MergeFailed]
description = "Merge conflict or other merge failure"
terminal = true

# Contract definitions for state outputs
[workflows.contracts]

[workflows.contracts.analysis]
name = "Static Analysis Report"
description = "Output from code analysis"

[workflows.contracts.analysis.input]
source_code = "string"
language = "enum:['rust', 'python', 'js', 'ts']"

[workflows.contracts.analysis.output]
issues = "array<{severity: enum, line: u32, description: string}>"
passing = "boolean"

[workflows.contracts.review]
name = "Code Review Result"

[workflows.contracts.review.output]
approved = "boolean"
feedback = "string"
requested_changes = "array<string>"

---

## Type System

### Field Types

```toml
# Primitive types
value = "string"
count = 123                    # integer
timeout = 3.14                 # float
enabled = true                 # boolean

# Enum types (validated against allowed values)
language = "enum:['rust', 'python', 'js']"
severity = "enum:['critical', 'high', 'medium', 'low']"

# Array types
tags = "array<string>"
agents = ["architect", "coder"]
numbers = [1, 2, 3]

# Object/table types
[config]
nested_key = "value"

# Optional types (can be null)
optional_value = "optional:string"
```

### Common Patterns

#### Duration Format
```toml
# All durations use seconds
timeout_seconds = 3600
retry_backoff_seconds = 60
max_execution_time_seconds = 7200
```

#### Event Handlers
```toml
handlers = [
    { event = "success", target = "NextState", guards = ["condition_name"] },
    { event = "failure", target = "ErrorState", guards = [] },
]
```

#### Guard Conditions
```toml
# Guards are named conditions evaluated before transition
guards = ["all_tests_pass", "code_review_approved", "no_breaking_changes"]

# Guard definitions (referenced in state handlers)
[workflows.guards]
all_tests_pass = "SELECT COUNT(*) FROM test_results WHERE passed = true"
code_review_approved = "agent.tester.decision == 'APPROVE'"
no_breaking_changes = "!git_diff.contains('breaking change')"
```

---

## Example 1: Simple Linear Workflow

```toml
[[workflows]]
name = "simple_approval"
description = "Simple approval workflow"

[workflows.metadata]
initial_state = "Pending"

[workflows.states]

[workflows.states.Pending]
description = "Awaiting approval"
handlers = [
    { event = "approve", target = "Approved", guards = [] },
    { event = "reject", target = "Rejected", guards = [] },
]

[workflows.states.Approved]
description = "Request approved"
terminal = true

[workflows.states.Rejected]
description = "Request rejected"
terminal = true
```

---

## Example 2: Hierarchical Workflow with Substates

```toml
[[workflows]]
name = "development_workflow"
description = "Hierarchical development process"

[workflows.metadata]
initial_state = "Planning"

[workflows.states]

[workflows.states.Planning]
description = "Planning phase"
parent = "InProgress"  # Parent state
agents = ["architect"]

[workflows.states.Implementation]
description = "Implementation phase"
parent = "InProgress"
agents = ["coder"]

[workflows.states.Testing]
description = "Testing phase"
parent = "InProgress"
agents = ["tester"]

[workflows.states.InProgress]
description = "Work in progress (parent state)"
entry_actions = ["log_start"]
exit_actions = ["log_end"]
handlers = [
    { event = "blocked", target = "Blocked", guards = [] },
    { event = "complete", target = "Complete", guards = ["all_substates_complete"] },
]

[workflows.states.Blocked]
description = "Work blocked"
handlers = [
    { event = "unblock", target = "Planning", guards = [] },
]

[workflows.states.Complete]
description = "Workflow complete"
terminal = true
```

---

## Example 3: Parallel Processing Workflow

```toml
[[workflows]]
name = "parallel_code_review"
description = "Multiple agents review code simultaneously"

[workflows.metadata]
initial_state = "SubmittedForReview"

[workflows.states]

[workflows.states.SubmittedForReview]
description = "Code submitted, starting reviews"
handlers = [
    { event = "reviews_complete", target = "ReviewsGathered", guards = [] },
]

[workflows.states.ReviewsGathered]
description = "All reviews collected"
agents = ["architect", "coder", "tester"]  # Run in parallel
parallel_execution = true
handlers = [
    { event = "consensus_approved", target = "Approved", guards = ["majority_approved"] },
    { event = "consensus_rejected", target = "ChangesNeeded", guards = [] },
]

[workflows.states.Approved]
description = "Code approved by consensus"
terminal = true

[workflows.states.ChangesNeeded]
description = "Majority requested changes"
terminal = true
```

---

## Advanced Features

### Guard Conditions

Guards are named boolean conditions that gate state transitions:

```toml
[workflows.guards]
# Expression-based guards
all_tests_pass = "context.test_results.passed_count == context.test_results.total_count"
no_lint_errors = "context.lint_errors.length == 0"
code_reviewed = "context.reviews.count >= 2"

# Query-based guards
db_check = "SELECT COUNT(*) FROM approvals WHERE pr_id = $pr_id"
```

Guards are specified in handlers:

```toml
handlers = [
    { event = "ready", target = "Approved", guards = ["all_tests_pass", "code_reviewed"] },
    # Transition only happens if BOTH guards are true
]
```

### Entry and Exit Actions

Actions executed when entering or leaving a state:

```toml
[workflows.states.Implementation]
entry_actions = [
    "log_start_implementation",
    "assign_developer",
    "create_feature_branch",
]
exit_actions = [
    "run_final_tests",
    "push_changes",
    "create_pull_request",
]
```

### Timeouts

Auto-transition on timeout:

```toml
[workflows.states.WaitingForReview]
description = "Waiting for code review"
timeout_seconds = 86400  # 24 hours
timeout_target = "TimedOut"
handlers = [
    { event = "review_complete", target = "ReviewComplete", guards = [] },
]
```

### Resource Requirements

Declare dependencies for each state:

```toml
[workflows.states.DeployProduction]
agents = ["deployer"]
required_resources = ["github_api", "slack", "deployment_service"]
description = "Deploy to production"
handlers = [
    { event = "deployment_success", target = "Live", guards = [] },
    { event = "deployment_failure", target = "RollBack", guards = [] },
]
```

---

## Validation Rules

### Mandatory Fields

1. **Workflow**: `name`, `metadata.initial_state`
2. **State**: `description`, `handlers` (except terminal states)
3. **Handler**: `event`, `target`

### Constraints

1. **State Cycles**: Graphs must be DAG (no cycles except via named transitions)
2. **Unreachable States**: All non-initial states must be reachable from initial state
3. **Deadends**: Terminal states must be reachable (if any agent can enter them)
4. **Agent Validity**: Agents referenced must exist in `[agents]` section
5. **Resource Validity**: Resources referenced must exist in `[resources]` section
6. **Guard Validity**: Guards referenced must be defined in `[workflows.guards]`

### Examples

**Invalid - Unreachable State**:
```toml
initial_state = "A"
[states]
[states.A]
handlers = [{ event = "go", target = "B" }]
[states.B]
handlers = [{ event = "go", target = "A" }]
[states.C]  # Unreachable!
handlers = []
```

**Valid - All States Reachable**:
```toml
initial_state = "A"
[states]
[states.A]
handlers = [{ event = "go_b", target = "B" }, { event = "go_c", target = "C" }]
[states.B]
handlers = [{ event = "done", target = "C" }]
[states.C]
terminal = true
```

---

## Code Generation

### From Swarm.toml to Rust (via Statig)

The compiler generates:

1. **State Enum**
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum CodeReviewState {
       Submitted,
       Analyzing,
       ReadyForReview,
       Approved,
       Merged,
       // ...
   }
   ```

2. **Context Struct**
   ```rust
   #[derive(Serialize, Deserialize, Clone, Debug)]
   pub struct CodeReviewContext {
       pr_id: String,
       reviewers: Vec<String>,
       approved_count: u32,
       // ...
   }
   ```

3. **State Machine**
   ```rust
   impl CodeReviewState {
       pub fn on_event(
           self,
           event: CodeReviewEvent,
           context: &mut CodeReviewContext,
       ) -> Self {
           // Generated handler logic
       }
   }
   ```

4. **Mermaid Diagram** (auto-generated documentation)
   ```mermaid
   stateDiagram-v2
       [*] --> Submitted
       Submitted --> Analyzing
       Analyzing --> ReadyForReview
       ReadyForReview --> Approved
       ReadyForReview --> ChangesRequested
       Approved --> Merged
       Merged --> [*]
   ```

---

## Best Practices

### 1. Naming Conventions

- **States**: PascalCase, verb+noun (e.g., `CodeReview`, `WaitingForApproval`)
- **Events**: SCREAMING_SNAKE_CASE or lowercase (e.g., `REVIEW_APPROVED`, `approval_granted`)
- **Guards**: camelCase (e.g., `allTestsPass`, `codeReviewedByTwo`)
- **Actions**: snake_case (e.g., `notify_developer`, `run_tests`)

### 2. State Organization

```toml
# Prefer: Clear linear progression
initial_state = "Submitted"
Submitted -> Analyzed -> Reviewed -> Approved -> Merged

# Over: Excessive backtracking
initial_state = "Submitted"
Submitted -> Reviewed -> Submitted -> Reviewed -> Analyzed
```

### 3. Agent Assignment

```toml
# Good: Clear agent responsibilities
[states.Analysis]
agents = ["architect"]  # One agent, clear role

[states.Review]
agents = ["reviewer"]

# Less ideal: Ambiguous agent selection
agents = ["any_available_agent"]  # Too vague
```

### 4. Timeouts

```toml
# Good: Explicit timeout handling
[states.WaitingForReview]
timeout_seconds = 86400
timeout_target = "TimedOut"

# Less ideal: No timeout (can hang forever)
[states.WaitingForReview]
handlers = [{ event = "approved", target = "Approved" }]
```

---

## Migration Guide: Swarm.toml to Rust

### Step 1: Parse Swarm.toml

```rust
use swarm_toml::{SwarmConfig, parse_toml_file};

let config = parse_toml_file("Swarm.toml")?;
```

### Step 2: Validate Workflow

```rust
config.validate()?;  // Checks for cycles, reachability, etc.
```

### Step 3: Generate State Machine

```rust
let state_machine = config.generate_state_machine()?;
state_machine.write_to_file("generated_workflow.rs")?;
```

### Step 4: Generate Mermaid Diagram

```rust
let diagram = config.generate_mermaid_diagram()?;
std::fs::write("workflow_diagram.md", diagram)?;
```

### Step 5: Compile & Run

```bash
cargo build
cargo run --workflow code_review
```

---

## File Organization

Recommended structure:

```
project/
├── Swarm.toml                    # Main workflow definition
├── swarm/
│   ├── code_review.toml          # Modular workflow definition (optional)
│   ├── development.toml
│   └── deployment.toml
├── generated/
│   ├── workflows.rs              # Generated code
│   └── workflows_mermaid.md      # Generated diagrams
└── src/
    ├── main.rs
    └── workflows.rs              # Manual workflow extensions
```

---

## Version Compatibility

| Swarm.toml Version | Descartes Version | Status |
|-------------------|------------------|--------|
| 1.0               | 0.2.0+           | Current |
| 1.1 (planned)     | 0.3.0+           | Future |

---

## Related Files

- **Implementation**: `/descartes/core/src/swarm_toml.rs`
- **Examples**: `/descartes/examples/swarm_toml/`
- **Tests**: `/descartes/core/tests/swarm_toml_*`

---

## References

- [Statig State Machine Library](https://github.com/mdeloof/statig)
- [Descartes Architecture Document](./descartes/README.md)
- [State Machine Evaluation](./STATE_MACHINE_EVALUATION.md)
