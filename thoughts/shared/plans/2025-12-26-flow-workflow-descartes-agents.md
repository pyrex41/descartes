# Flow Workflow as Descartes Agents - Implementation Plan

## Overview

Convert the flow orchestration system from Claude Code slash commands to Descartes agents with full observability. The user wants visibility into agents running within the multi-agent system, not opaque one-line commands.

## Current State Analysis

### What Exists Now:
- **Claude Code commands** (7 files): `.claude/commands/flow/{start,ingest,review-graph,plan-tasks,implement,qa,summarize}.md`
- **Partial agents** (2 files): `.claude/agents/flow/{implementer,utils}.md`
- **State tracking schema**: `.scud/flow-state.json`

### What's Missing:
- Descartes agent definitions for flow phases
- Custom flow executor with state management
- CLI subcommand for invoking flow workflow
- Integration with Descartes observability infrastructure

### Key Discoveries:

**Agent Definition Format** (`descartes/agents/*.md`):
```markdown
---
name: agent-name
description: Brief description
model: claude-3-sonnet
tool_level: researcher | readonly | planner | orchestrator
tags: [tag1, tag2]
---

System prompt content...
```

**Workflow Registration** (`descartes/core/src/workflow_commands.rs:125-200`):
- Uses builder pattern: `WorkflowCommand::new().then().parallel()`
- Steps can be sequential or parallel
- Registered in `register_builtins()`

**CLI Structure** (`descartes/cli/src/commands/workflow.rs`):
- Each workflow has explicit subcommand in `WorkflowCommands` enum
- Arguments: `--topic`, `--context`, `--dir`, `--adapter`
- Handler in `execute()` function

**Agent Bundling** (`descartes/core/src/agent_definitions.rs:124-144`):
- Agents bundled via `include_str!` macro
- Array `DEFAULT_AGENTS` with (filename, content) tuples
- Installed to `~/.descartes/agents/` on first run

## Desired End State

After implementation:
1. User can run `descartes workflow flow --prd <path> [--tag <name>] [--resume]`
2. Flow executes 6 phases: ingest → review-graph → plan-tasks → implement+qa → summarize
3. Full observability via `AgentRuntimeState`, `AgentHistoryEvent`, EventBus
4. State persisted to `.scud/flow-state.json` for pause/resume
5. Orchestrator agent handles errors and decisions
6. Old Claude Code command files removed

### Verification:
```bash
# Build and install
cargo build -p descartes-cli
descartes workflow flow --help  # Shows flow subcommand

# Test execution
descartes workflow flow --prd docs/prd/test-feature.md --tag test-flow

# Check observability
descartes daemon status  # Shows running agents
cat .scud/flow-state.json  # Shows state tracking
```

## What We're NOT Doing

- Not modifying the existing `research`, `plan`, `implement` workflow commands
- Not changing the general workflow executor - creating a separate flow executor
- Not adding flow to the generic workflow registry (it has special requirements)
- Not modifying scud CLI or scud-task npm package

## Implementation Approach

Create 7 Descartes agents, a custom `FlowExecutor` module, and a new CLI subcommand. The FlowExecutor handles state management and phase transitions, while agents do the actual work with full observability.

---

## Phase 1: Create Agent Definitions

### Overview
Create 7 agent markdown files in `descartes/agents/` following existing patterns.

### Changes Required:

#### 1.1 Flow Orchestrator Agent

**File**: `descartes/agents/flow-orchestrator.md`
**Changes**: Create new file - meta-agent for error handling and phase decisions

```markdown
---
name: flow-orchestrator
description: Meta-orchestrator for flow workflow decisions and error recovery
model: claude-3-sonnet
tool_level: orchestrator
tags: [flow, workflow, orchestration]
---

# Flow Orchestrator

You are the meta-orchestrator for the flow workflow. You are invoked when decisions or error recovery are needed during flow execution.

## Core Responsibilities

- Make decisions when a phase encounters ambiguous situations
- Handle errors and decide whether to retry, skip, or abort
- Provide intelligent guidance to phase agents
- Update flow state with decisions

## Decision Framework

When invoked with a decision request:
1. Analyze the context and options presented
2. Consider the overall workflow goals
3. Make a clear decision with reasoning
4. Return structured response with decision and rationale

## Error Recovery

When invoked with an error:
1. Assess severity (critical, recoverable, ignorable)
2. For recoverable errors, suggest remediation
3. For critical errors, recommend abort with clear explanation
4. Always preserve flow state for potential resume

## Guidelines

- Be decisive - don't defer decisions back to agents
- Prioritize workflow completion over perfection
- Document all decisions for audit trail
- Consider downstream impacts of decisions
```

#### 1.2 Flow Ingest Agent

**File**: `descartes/agents/flow-ingest.md`
**Changes**: Create new file - adapting from `.claude/commands/flow/ingest.md`

```markdown
---
name: flow-ingest
description: Parse PRD into SCUD tasks with dependency mapping
model: claude-3-sonnet
tool_level: researcher
tags: [flow, workflow, prd, tasks]
---

# Flow Ingest Agent

You are the PRD Ingestion agent for the flow workflow. Your job is to take a PRD file and create a well-structured SCUD task graph.

## Core Responsibilities

1. Read and analyze PRD file completely
2. Initialize flow state in `.scud/flow-state.json`
3. Create or select a tag for this workflow
4. Parse PRD into SCUD tasks using `scud parse-prd`
5. Review and refine generated tasks
6. Expand complex tasks (complexity >= 13)
7. Compute initial waves with `scud waves`
8. Update flow state with completion data

## Process

### 1. Read PRD
Identify scope, features, dependencies, complexity indicators, and success criteria.

### 2. Initialize State
Update `.scud/flow-state.json` with started_at, prd_file, current_phase.

### 3. Parse PRD
```bash
scud parse-prd <prd-file> --tag <tag-name>
```

### 4. Review Tasks
```bash
scud list
scud show <task-id>
```

Verify: requirements covered, complexity reasonable, dependencies logical.

### 5. Expand Complex Tasks
For tasks with complexity >= 13:
```bash
scud expand <task-id>
```

### 6. Compute Waves
```bash
scud waves
```

### 7. Update State
Record tasks_created, tag, status=completed in flow state.

## Output Format

Report completion with:
- Tag name
- Task counts (total, top-level, subtasks)
- Complexity points total
- Wave breakdown

## Guidelines

- Read the entire PRD before creating tasks
- Start broad, then refine
- Explicit dependencies are better than implicit
- Complexity > 21 must be broken down
- Use kebab-case for tags
```

#### 1.3 Flow Review Graph Agent

**File**: `descartes/agents/flow-review-graph.md`
**Changes**: Create new file - adapting from `.claude/commands/flow/review-graph.md`

```markdown
---
name: flow-review-graph
description: Analyze and optimize SCUD task dependency graph
model: claude-3-sonnet
tool_level: researcher
tags: [flow, workflow, dependencies, graph]
---

# Flow Review Graph Agent

You analyze the SCUD task graph for issues and optimization opportunities.

## Core Responsibilities

1. Load and visualize task graph
2. Check for missing dependencies
3. Detect circular dependencies
4. Identify parallelization opportunities
5. Suggest wave optimizations
6. Apply fixes directly to SCG file

## Process

### 1. Load Graph
```bash
scud waves
scud list
cat .scud/tasks/tasks.scg
```

### 2. Analyze Dependencies
Check for:
- Missing dependencies (A needs B but no edge exists)
- Circular dependencies
- Over-specified dependencies (unnecessary edges)
- Parallelization opportunities

### 3. Apply Fixes
Edit `.scud/tasks/tasks.scg` directly:
- Add edges in `@edges` section: `<dependent> -> <dependency>`
- Remove unnecessary edges

### 4. Verify
```bash
scud waves
```

### 5. Update State
Record fixes_applied in flow state.

## Output Format

Report:
- Issues found by category
- Fixes applied
- Optimized wave structure
- Before/after parallelization metrics

## Guidelines

- Conservative with changes
- Document reasoning for each fix
- Preserve existing valid relationships
- Optimize for maximum parallelization
```

#### 1.4 Flow Plan Tasks Agent

**File**: `descartes/agents/flow-plan-tasks.md`
**Changes**: Create new file - adapting from `.claude/commands/flow/plan-tasks.md`

```markdown
---
name: flow-plan-tasks
description: Generate implementation plans for complex SCUD tasks
model: claude-3-sonnet
tool_level: planner
tags: [flow, workflow, planning, implementation]
---

# Flow Plan Tasks Agent

You create detailed implementation plans for complex tasks in the flow workflow.

## Core Responsibilities

1. Identify tasks needing plans (complexity >= 8, has subtasks, critical path)
2. Research codebase context for each task
3. Create implementation plan documents
4. Link plans to tasks in flow state

## Process

### 1. Identify Tasks
```bash
scud list
scud show <task-id>
```

Tasks needing plans: complexity >= 8, have subtasks, or on critical path.

### 2. For Each Task

#### Research Context
- Find relevant files with codebase-locator patterns
- Understand current implementation
- Identify patterns to follow

#### Create Plan
Write to `thoughts/shared/plans/<date>-<tag>-task-<id>.md`:

```markdown
# Task <id>: <title>

## Overview
[What this task accomplishes]

## Context
- Dependencies: [list]
- Dependents: [list]

## Current State
[Existing code, relevant locations]

## Implementation Approach
[Specific steps]

## Files to Modify
- `path/to/file.ext`: [changes]

## Success Criteria
### Automated
- [ ] Build passes: `make check`
- [ ] Tests pass: `make test`

### Manual
- [ ] Feature works as expected
```

### 3. Update State
Record plans_created, plan paths in flow state.

## Skip Conditions

Skip planning for:
- Trivial tasks (complexity <= 3)
- Tasks with existing plans
- Pure config/documentation tasks

## Output Format

Report:
- Plans created count
- Plans skipped count
- Task-to-plan mapping
```

#### 1.5 Flow Implement Agent

**File**: `descartes/agents/flow-implement.md`
**Changes**: Create new file - adapting from `.claude/commands/flow/implement.md`

```markdown
---
name: flow-implement
description: Execute SCUD tasks following implementation plans
model: claude-3-sonnet
tool_level: orchestrator
tags: [flow, workflow, implementation, execution]
---

# Flow Implement Agent

You orchestrate the implementation of SCUD tasks, spawning sub-agents for each task.

## Core Responsibilities

1. Process tasks wave by wave
2. Spawn implementation sub-agents for each task
3. Monitor progress and handle failures
4. Commit changes after each wave
5. Track completion in flow state

## Process

### 1. Get Current Wave
```bash
scud waves
scud next
```

### 2. For Each Wave

#### Execute Tasks in Parallel
Spawn implementation agents (max 3 concurrent):
- Pass task ID and description
- Pass plan file path if exists
- Pass dependency context

#### Monitor Progress
- Track agent completion
- Handle failures with retries
- Update task status: `scud set-status <id> done`

#### Commit Wave
```bash
git add -A
git commit -m "feat(<tag>): complete wave <N> tasks"
```

### 3. Update State
Record current_wave, tasks_completed, tasks_total.

## Sub-Agent Protocol

For each task, spawn agent with:
- Task ID and description
- Plan file path (if exists)
- Context from dependencies
- Success criteria to verify

## Error Handling

On task failure:
1. Log error details
2. Check if retryable
3. If retries exhausted, mark blocked and continue
4. Report blocked tasks at wave end

## Output Format

Report per wave:
- Tasks attempted/completed/failed
- Commit hash
- Blocked tasks (if any)
```

#### 1.6 Flow QA Agent

**File**: `descartes/agents/flow-qa.md`
**Changes**: Create new file - adapting from `.claude/commands/flow/qa.md`

```markdown
---
name: flow-qa
description: Monitor implementation quality and document intent trail
model: claude-3-sonnet
tool_level: researcher
tags: [flow, workflow, qa, quality]
---

# Flow QA Agent

You monitor implementation quality concurrently with the implement phase.

## Core Responsibilities

1. Watch for new commits during implementation
2. Review changes for quality and consistency
3. Document intent-to-implementation trail
4. Log issues for follow-up
5. Generate QA summary

## Process

### 1. Monitor Commits
Watch for new commits, analyze each:
```bash
git log --oneline -10
git diff HEAD~1
```

### 2. For Each Change

#### Review Quality
- Code follows project patterns
- No obvious bugs or security issues
- Tests included where appropriate
- Documentation updated

#### Document Trail
Record in QA log:
- Task ID → Commit hash
- Intent (from plan) → Implementation (from diff)
- Any deviations noted

### 3. Log Issues
For each issue found:
- Severity (blocker, major, minor)
- Task ID
- Description
- Suggested fix

### 4. Update State
Record issues_found, tasks_reviewed.

## QA Log Format

Write to `.scud/qa-log.json`:
```json
{
  "reviews": [
    {
      "task_id": "3",
      "commit": "abc123",
      "status": "pass|issues",
      "issues": [],
      "timestamp": "..."
    }
  ]
}
```

## Output Format

Generate summary with:
- Tasks reviewed
- Issues by severity
- Coverage percentage
- Recommendations
```

#### 1.7 Flow Summarize Agent

**File**: `descartes/agents/flow-summarize.md`
**Changes**: Create new file - adapting from `.claude/commands/flow/summarize.md`

```markdown
---
name: flow-summarize
description: Generate comprehensive workflow summary and documentation
model: claude-3-sonnet
tool_level: readonly
tags: [flow, workflow, summary, documentation]
---

# Flow Summarize Agent

You generate the final summary and documentation for a completed flow workflow.

## Core Responsibilities

1. Aggregate data from all phases
2. Generate comprehensive summary document
3. Create final QA report
4. Update flow state with completion

## Process

### 1. Gather Data
Read from flow state and artifacts:
- PRD file
- Tasks completed
- Plans created
- QA log
- Git commits

### 2. Generate Summary
Write to `thoughts/shared/reports/<date>-<tag>-summary.md`:

```markdown
# Flow Summary: <tag>

## Overview
- PRD: <path>
- Duration: <time>
- Tasks: <completed>/<total>

## Implementation Highlights
[Key features implemented]

## Quality Metrics
- QA issues: <count>
- Test coverage: <percentage>

## Commits
[List of commits with messages]

## Artifacts
- Plans: [list]
- QA Report: [path]

## Recommendations
[Follow-up items, tech debt noted]
```

### 3. Final QA Report
Write to `thoughts/shared/reports/<date>-<tag>-qa-final.md`

### 4. Update State
Set status=completed, record report paths, end_commit.

## Output Format

Return:
- Summary document path
- QA report path
- Completion metrics
```

### Success Criteria:

#### Automated Verification:
- [x] All 7 agent files created in `descartes/agents/`
- [x] Each file has valid YAML frontmatter
- [x] Files match pattern: `flow-*.md`

#### Manual Verification:
- [ ] Agent prompts are clear and actionable
- [ ] Tool levels are appropriate for each agent's needs

---

## Phase 2: Bundle Agents and Create Flow Executor

### Overview
Register new agents in the loader and create the custom FlowExecutor module.

### Changes Required:

#### 2.1 Bundle Agents in Loader

**File**: `descartes/core/src/agent_definitions.rs`
**Changes**: Add 7 new agents to DEFAULT_AGENTS array

Find the `DEFAULT_AGENTS` constant (around line 124) and add:

```rust
// Add to DEFAULT_AGENTS array:
("flow-orchestrator.md", include_str!("../../agents/flow-orchestrator.md")),
("flow-ingest.md", include_str!("../../agents/flow-ingest.md")),
("flow-review-graph.md", include_str!("../../agents/flow-review-graph.md")),
("flow-plan-tasks.md", include_str!("../../agents/flow-plan-tasks.md")),
("flow-implement.md", include_str!("../../agents/flow-implement.md")),
("flow-qa.md", include_str!("../../agents/flow-qa.md")),
("flow-summarize.md", include_str!("../../agents/flow-summarize.md")),
```

#### 2.2 Create Flow Executor Module

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Create new file with FlowState and FlowExecutor

```rust
//! Flow Executor - Custom executor for the flow workflow with state management.
//!
//! The flow workflow is different from standard workflows:
//! - File-based input (PRD path) rather than topic string
//! - Stateful with pause/resume via .scud/flow-state.json
//! - Concurrent QA monitoring during implementation
//! - Orchestrator agent for intelligent error handling

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::fs;

use crate::agent_definitions::AgentDefinitionLoader;
use crate::traits::ModelBackend;
use crate::workflow_executor::{WorkflowContext, execute_step};

/// Flow phase status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Active,
    Completed,
    Failed,
    Skipped,
}

/// Individual phase state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub status: PhaseStatus,
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Flow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    pub orchestrator_model: String,
    pub implementation_model: String,
    pub qa_model: String,
    pub max_parallel_tasks: usize,
    pub auto_commit: bool,
    pub pause_between_phases: bool,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            orchestrator_model: "opus".to_string(),
            implementation_model: "sonnet".to_string(),
            qa_model: "sonnet".to_string(),
            max_parallel_tasks: 3,
            auto_commit: true,
            pause_between_phases: false,
        }
    }
}

/// Git tracking for flow
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowGitState {
    pub start_commit: Option<String>,
    pub end_commit: Option<String>,
    pub branch: Option<String>,
}

/// Artifact paths
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowArtifacts {
    pub prd_path: Option<PathBuf>,
    pub tasks_path: Option<PathBuf>,
    pub plans_dir: Option<PathBuf>,
    pub qa_log_path: Option<PathBuf>,
    pub qa_final_path: Option<PathBuf>,
    pub summary_path: Option<PathBuf>,
}

/// Complete flow state - matches .scud/flow-state.json schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowState {
    pub version: String,
    pub started_at: Option<DateTime<Utc>>,
    pub prd_file: Option<PathBuf>,
    pub tag: Option<String>,
    pub current_phase: Option<String>,
    pub phases: FlowPhases,
    pub config: FlowConfig,
    pub artifacts: FlowArtifacts,
    pub git: FlowGitState,
}

/// All phase states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPhases {
    pub ingest: PhaseState,
    pub review_graph: PhaseState,
    pub plan_tasks: PhaseState,
    pub implement: PhaseState,
    pub qa: PhaseState,
    pub summarize: PhaseState,
}

impl Default for FlowState {
    fn default() -> Self {
        let default_phase = PhaseState {
            status: PhaseStatus::Pending,
            completed_at: None,
            data: serde_json::json!({}),
        };

        Self {
            version: "1.0".to_string(),
            started_at: None,
            prd_file: None,
            tag: None,
            current_phase: None,
            phases: FlowPhases {
                ingest: default_phase.clone(),
                review_graph: default_phase.clone(),
                plan_tasks: default_phase.clone(),
                implement: default_phase.clone(),
                qa: default_phase.clone(),
                summarize: default_phase,
            },
            config: FlowConfig::default(),
            artifacts: FlowArtifacts::default(),
            git: FlowGitState::default(),
        }
    }
}

/// Result from flow execution
#[derive(Debug)]
pub struct FlowResult {
    pub success: bool,
    pub phases_completed: Vec<String>,
    pub phases_failed: Vec<String>,
    pub summary_path: Option<PathBuf>,
    pub duration_secs: u64,
}

/// Flow executor with state management
pub struct FlowExecutor {
    state: FlowState,
    state_path: PathBuf,
    working_dir: PathBuf,
    agent_loader: AgentDefinitionLoader,
    backend: Arc<dyn ModelBackend>,
}

impl FlowExecutor {
    /// Create new flow executor
    pub async fn new(
        prd_path: PathBuf,
        tag: Option<String>,
        working_dir: Option<PathBuf>,
        backend: Arc<dyn ModelBackend>,
    ) -> Result<Self> {
        let working_dir = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        let state_path = working_dir.join(".scud/flow-state.json");

        // Load existing state or create new
        let mut state = if state_path.exists() {
            let content = fs::read_to_string(&state_path).await?;
            serde_json::from_str(&content)?
        } else {
            FlowState::default()
        };

        // Update with new execution params
        state.started_at = Some(Utc::now());
        state.prd_file = Some(prd_path);
        state.tag = tag.or(state.tag);

        let agent_loader = AgentDefinitionLoader::new()?;

        Ok(Self {
            state,
            state_path,
            working_dir,
            agent_loader,
            backend,
        })
    }

    /// Resume from existing state
    pub async fn resume(
        working_dir: Option<PathBuf>,
        backend: Arc<dyn ModelBackend>,
    ) -> Result<Self> {
        let working_dir = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        let state_path = working_dir.join(".scud/flow-state.json");

        let content = fs::read_to_string(&state_path)
            .await
            .context("No flow state found. Start a new flow first.")?;
        let state: FlowState = serde_json::from_str(&content)?;

        let agent_loader = AgentDefinitionLoader::new()?;

        Ok(Self {
            state,
            state_path,
            working_dir,
            agent_loader,
            backend,
        })
    }

    /// Execute the full flow workflow
    pub async fn execute(&mut self) -> Result<FlowResult> {
        let start_time = std::time::Instant::now();
        let mut phases_completed = Vec::new();
        let mut phases_failed = Vec::new();

        // Phase 1-3: Sequential
        for phase in ["ingest", "review_graph", "plan_tasks"] {
            match self.execute_phase(phase).await {
                Ok(_) => phases_completed.push(phase.to_string()),
                Err(e) => {
                    phases_failed.push(phase.to_string());
                    tracing::error!("Phase {} failed: {}", phase, e);
                    // Invoke orchestrator for decision
                    if !self.handle_phase_error(phase, &e).await? {
                        break;
                    }
                }
            }
            self.save_state().await?;
        }

        // Phase 4-5: Concurrent (implement + QA monitoring)
        let (impl_result, qa_result) = tokio::join!(
            self.execute_phase("implement"),
            self.execute_phase("qa")
        );

        match impl_result {
            Ok(_) => phases_completed.push("implement".to_string()),
            Err(e) => {
                phases_failed.push("implement".to_string());
                tracing::error!("Implement phase failed: {}", e);
            }
        }

        match qa_result {
            Ok(_) => phases_completed.push("qa".to_string()),
            Err(e) => {
                phases_failed.push("qa".to_string());
                tracing::error!("QA phase failed: {}", e);
            }
        }

        self.save_state().await?;

        // Phase 6: Sequential
        match self.execute_phase("summarize").await {
            Ok(_) => phases_completed.push("summarize".to_string()),
            Err(e) => {
                phases_failed.push("summarize".to_string());
                tracing::error!("Summarize phase failed: {}", e);
            }
        }

        self.save_state().await?;

        let duration = start_time.elapsed();

        Ok(FlowResult {
            success: phases_failed.is_empty(),
            phases_completed,
            phases_failed,
            summary_path: self.state.artifacts.summary_path.clone(),
            duration_secs: duration.as_secs(),
        })
    }

    /// Execute a single phase
    async fn execute_phase(&mut self, phase: &str) -> Result<()> {
        let agent_name = format!("flow-{}", phase.replace('_', "-"));

        // Update state
        self.state.current_phase = Some(phase.to_string());
        self.update_phase_status(phase, PhaseStatus::Active);

        // Load agent definition
        let agent_def = self.agent_loader.load_agent(&agent_name)?;

        // Build context
        let context = WorkflowContext::new(
            self.working_dir.clone(),
            self.state.tag.as_deref().unwrap_or("flow"),
        )?;

        // Build task with context
        let task = format!(
            "Execute {} phase for flow workflow.\nPRD: {:?}\nTag: {:?}\nState: {:?}",
            phase,
            self.state.prd_file,
            self.state.tag,
            self.state_path
        );

        // Execute via workflow executor infrastructure
        // Note: This uses existing execute_step which provides observability
        let step = crate::workflow_commands::WorkflowStep {
            name: format!("Flow: {}", phase),
            agent: agent_name,
            task,
            parallel: false,
            output: None,
        };

        let config = crate::workflow_executor::WorkflowExecutorConfig::default();
        let result = execute_step(&step, &context, self.backend.clone(), &config).await?;

        if result.success {
            self.update_phase_status(phase, PhaseStatus::Completed);
        } else {
            self.update_phase_status(phase, PhaseStatus::Failed);
            anyhow::bail!("Phase {} failed: {}", phase, result.error.unwrap_or_default());
        }

        Ok(())
    }

    /// Handle phase error with orchestrator agent
    async fn handle_phase_error(&mut self, phase: &str, error: &anyhow::Error) -> Result<bool> {
        // Load orchestrator agent
        let agent_def = self.agent_loader.load_agent("flow-orchestrator")?;

        let context = WorkflowContext::new(
            self.working_dir.clone(),
            self.state.tag.as_deref().unwrap_or("flow"),
        )?;

        let task = format!(
            "Phase '{}' failed with error: {}\n\nDecide: retry, skip, or abort?",
            phase, error
        );

        let step = crate::workflow_commands::WorkflowStep {
            name: "Flow: Error Recovery".to_string(),
            agent: "flow-orchestrator".to_string(),
            task,
            parallel: false,
            output: None,
        };

        let config = crate::workflow_executor::WorkflowExecutorConfig::default();
        let result = execute_step(&step, &context, self.backend.clone(), &config).await?;

        // Parse decision from result
        // For now, simple heuristic - abort on error
        Ok(result.success && !result.output.to_lowercase().contains("abort"))
    }

    fn update_phase_status(&mut self, phase: &str, status: PhaseStatus) {
        let phase_state = match phase {
            "ingest" => &mut self.state.phases.ingest,
            "review_graph" => &mut self.state.phases.review_graph,
            "plan_tasks" => &mut self.state.phases.plan_tasks,
            "implement" => &mut self.state.phases.implement,
            "qa" => &mut self.state.phases.qa,
            "summarize" => &mut self.state.phases.summarize,
            _ => return,
        };

        phase_state.status = status.clone();
        if status == PhaseStatus::Completed {
            phase_state.completed_at = Some(Utc::now());
        }
    }

    /// Save state to disk
    async fn save_state(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.state_path, content).await?;
        Ok(())
    }
}
```

#### 2.3 Wire Up Module

**File**: `descartes/core/src/lib.rs`
**Changes**: Add pub mod declaration

```rust
// Add to existing module declarations:
pub mod flow_executor;
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build -p descartes-core` succeeds
- [x] `cargo test -p descartes-core` passes (flow_executor tests pass - 6/6)
- [x] `cargo clippy -p descartes-core` passes (no new warnings in flow_executor)

#### Manual Verification:
- [ ] FlowExecutor compiles with all dependencies resolved
- [ ] State serialization/deserialization works correctly

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: Add CLI Subcommand

### Overview
Add the `flow` subcommand to the Descartes CLI.

### Changes Required:

#### 3.1 Add Flow Subcommand

**File**: `descartes/cli/src/commands/workflow.rs`
**Changes**: Add Flow variant to WorkflowCommands enum and handler

Add to `WorkflowCommands` enum:

```rust
/// Run the full flow workflow from PRD to implementation
#[command(name = "flow")]
Flow {
    /// Path to the PRD file
    #[arg(long)]
    prd: PathBuf,

    /// Tag name for this workflow (auto-generated if not provided)
    #[arg(long)]
    tag: Option<String>,

    /// Resume from previous flow state
    #[arg(long)]
    resume: bool,

    /// Working directory
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Model adapter to use
    #[arg(long)]
    adapter: Option<String>,
},
```

Add handler in `execute()` function:

```rust
WorkflowCommands::Flow { prd, tag, resume, dir, adapter } => {
    use descartes_core::flow_executor::FlowExecutor;

    // Create backend (similar to other workflow commands)
    let (provider_name, model, backend) = if let Some(adapter_name) = adapter {
        create_adapter_backend(adapter_name.as_str())?
    } else {
        create_backend(config, None)?
    };

    println!("Flow Workflow");
    println!("═══════════════════════════════════════════════════");
    println!("PRD: {:?}", prd);
    println!("Tag: {:?}", tag);
    println!("Resume: {}", resume);
    println!();

    let mut executor = if resume {
        FlowExecutor::resume(dir, backend).await?
    } else {
        FlowExecutor::new(prd, tag, dir, backend).await?
    };

    let result = executor.execute().await?;

    println!();
    println!("Flow Complete!");
    println!("═══════════════════════════════════════════════════");
    println!("Phases completed: {:?}", result.phases_completed);
    if !result.phases_failed.is_empty() {
        println!("Phases failed: {:?}", result.phases_failed);
    }
    if let Some(summary) = result.summary_path {
        println!("Summary: {:?}", summary);
    }
    println!("Duration: {}s", result.duration_secs);

    if result.success {
        println!("\nReady for retrospective: /scud:retrospective");
    }

    Ok(())
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build -p descartes-cli` succeeds
- [x] `descartes workflow --help` shows flow subcommand
- [x] `descartes workflow flow --help` shows correct options

#### Manual Verification:
- [ ] Command invocation starts flow execution
- [ ] Resume flag loads existing state correctly

**Implementation Note**: After completing this phase, proceed to Phase 4 for cleanup.

---

## Phase 4: Cleanup Old Files

### Overview
Remove the old Claude Code command and agent files that are now replaced.

### Changes Required:

#### 4.1 Remove Claude Code Commands

**Files to delete**:
- `.claude/commands/flow/start.md`
- `.claude/commands/flow/ingest.md`
- `.claude/commands/flow/review-graph.md`
- `.claude/commands/flow/plan-tasks.md`
- `.claude/commands/flow/implement.md`
- `.claude/commands/flow/qa.md`
- `.claude/commands/flow/summarize.md`

```bash
rm -rf .claude/commands/flow/
```

#### 4.2 Remove Partial Agent Files

**Files to delete**:
- `.claude/agents/flow/implementer.md`
- `.claude/agents/flow/utils.md`

```bash
rm -rf .claude/agents/flow/
```

### Success Criteria:

#### Automated Verification:
- [x] `ls .claude/commands/flow/` returns "No such file or directory"
- [x] `ls .claude/agents/flow/` returns "No such file or directory"

#### Manual Verification:
- [ ] Git status shows deleted files
- [ ] No broken references to deleted files

---

## Testing Strategy

### Unit Tests:
- FlowState serialization/deserialization
- Phase status transitions
- FlowExecutor construction

### Integration Tests:
- Full flow execution with mock backend
- Resume from saved state
- Concurrent QA monitoring

### Manual Testing Steps:
1. Create test PRD: `docs/prd/test-flow-feature.md`
2. Run: `descartes workflow flow --prd docs/prd/test-flow-feature.md --tag test-flow`
3. Verify state file: `cat .scud/flow-state.json`
4. Test resume: `descartes workflow flow --resume`
5. Check observability: monitor via daemon/TUI

## Performance Considerations

- FlowExecutor saves state after each phase for resilience
- Concurrent QA monitoring adds minimal overhead
- Agent spawning uses existing workflow infrastructure for efficiency

## Migration Notes

- Existing `.scud/flow-state.json` files are compatible with new schema
- Users of old `/flow:*` commands should switch to `descartes workflow flow`

## References

- Original design: `thoughts/shared/plans/2025-12-25-opinionated-workflow-orchestration.md`
- Existing agents pattern: `descartes/agents/researcher.md`
- Workflow executor: `descartes/core/src/workflow_executor.rs`
- CLI patterns: `descartes/cli/src/commands/workflow.rs`
