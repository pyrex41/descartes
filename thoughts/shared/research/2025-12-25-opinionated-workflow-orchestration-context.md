---
date: 2025-12-25T12:00:00-08:00
researcher: Claude
git_commit: e48e933b5290cf55b2f177cbc5821d9cad5145de
branch: master
repository: backbone
topic: "Opinionated Workflow Orchestration - Existing Components Analysis"
tags: [research, scud, orchestration, workflow, agents, cl-skills, prd]
status: complete
last_updated: 2025-12-25
last_updated_by: Claude
---

# Research: Opinionated Workflow Orchestration - Existing Components Analysis

**Date**: 2025-12-25T12:00:00-08:00
**Researcher**: Claude
**Git Commit**: e48e933b5290cf55b2f177cbc5821d9cad5145de
**Branch**: master
**Repository**: backbone

## Research Question

The user wants to understand the existing codebase components that could support building an opinionated workflow system that:
1. Takes a PRD file and uses SCUD to parse/expand it
2. Uses a frontier model to review tasks.scg, scud list, scud waves and fix dependency issues
3. Uses an orchestrator agent with SCUD CLI to manage subagents within context limits
4. Uses cl:create_plan and cl:implement_plan prompts for task review and implementation
5. Potentially has dual orchestrators (manager/delegator and monitor/QA)
6. Documents the path from intent to implementation

## Summary

The codebase contains a comprehensive set of components that could be leveraged for this workflow:

1. **SCUD Role-Based Workflow System** - A 5-phase workflow (ideation → planning → architecture → implementation → retrospective) with role-playing agents (PM, SM, Architect, Dev, Retrospective)
2. **cl: Skill System** - Research, planning, and implementation skills that spawn parallel subagents for thorough work
3. **Descartes Orchestration Engine** - State machines, DAG-based dependency management, workflow execution, and swarm configuration
4. **SCG Task Storage** - File-based task format with phase/wave organization for SCUD CLI integration
5. **Thoughts/Research Documentation** - Structured documentation system for capturing plans and research

---

## Detailed Findings

### 1. SCUD Role-Based Workflow System

**Location**: `.claude/commands/scud/` and `.opencode/command/scud/`

The SCUD system implements a complete software development lifecycle through five role-playing agents:

#### Phase Flow
```
ideation (PM) → planning (SM) → architecture (Architect) →
implementation (Dev) → retrospective (Retro) → [cycle repeats]
```

#### Role Agents

| Role | File | Phase | Primary Responsibilities |
|------|------|-------|-------------------------|
| Product Manager | `pm.md` | ideation/planning | Create PRD, define tags, discovery interviews |
| Scrum Master | `sm.md` | planning | Parse PRD to tasks, estimate complexity, compute waves |
| Architect | `architect.md` | architecture | System design, ADRs, wave-aware planning |
| Developer | `dev.md` | implementation | Task claiming, implementation, wave execution |
| Retrospective | `retrospective.md` | retrospective | Metrics gathering, learning capture |

#### Key SCUD CLI Commands

**PRD Parsing (SM role)**:
```bash
scud parse-prd docs/prd/[file].md --tag=<tag>
```

**Wave Computation**:
```bash
scud waves --tag <tag>
scud waves --tag <tag> --max-parallel 5
```

**Task Management**:
```bash
scud list --tag <tag>
scud show <task-id> --tag <tag>
scud stats --tag <tag>
scud next --tag <tag>
scud claim <task-id> --name dev --tag <tag>
scud set-status <task-id> done --tag <tag>
```

#### Workflow State Tracking
- State stored in `.scud/workflow-state.json`
- Phases tracked with status (active/completed) and timestamps
- Active tag tracked for current work context

---

### 2. cl: Workflow Skills System

**Location**: `.claude/commands/cl/`

Four primary skills that form a comprehensive development workflow:

#### cl:create_plan
- **File**: `create_plan.md` (458 lines)
- **Purpose**: Creates implementation plans through interactive process
- **Process**:
  1. Read mentioned files FULLY in main context
  2. Spawn parallel research agents (codebase-locator, codebase-analyzer, codebase-pattern-finder)
  3. Present findings with file:line references
  4. Get user feedback on structure
  5. Write detailed plan to `thoughts/shared/plans/YYYY-MM-DD-description.md`
- **Success Criteria Split**:
  - **Automated Verification**: Commands agents can run
  - **Manual Verification**: Requires human testing

#### cl:implement_plan
- **File**: `implement_plan.md` (85 lines)
- **Purpose**: Executes plans with verification checkpoints
- **Process**:
  1. Read plan completely, check existing checkmarks
  2. Implement each phase fully before next
  3. Run automated verification (make check test)
  4. Pause for human manual verification
  5. Update checkboxes in plan as sections complete

#### cl:research_codebase
- **File**: `research_codebase.md` (191 lines)
- **Purpose**: Documents codebase through parallel research (documentation-only, no recommendations)
- **Output**: Research document in `thoughts/shared/research/YYYY-MM-DD-description.md`

#### cl:iterate_plan
- **File**: `iterate_plan.md` (239 lines)
- **Purpose**: Updates existing plans based on feedback
- **Features**: Surgical edits, research if needed, maintains quality standards

#### Specialized Subagents Used

| Agent | Purpose |
|-------|---------|
| codebase-locator | Finds WHERE files/components are located |
| codebase-analyzer | Documents HOW code works |
| codebase-pattern-finder | Finds existing patterns to model after |
| thoughts-analyzer | Analyzes thoughts/plans documents |
| thoughts-locator | Discovers relevant thoughts documents |
| web-search-researcher | External documentation research |

**Agent Definitions**: `.claude/agents/cl/`

---

### 3. Descartes Orchestration Engine

**Location**: `descartes/core/src/`

The Descartes system provides a comprehensive orchestration framework:

#### State Machine (`state_machine.rs`)

**WorkflowState Enum** (line 66):
- `Idle`, `Running`, `Paused`, `Completed`, `Failed`, `Cancelled`

**WorkflowStateMachine** (line 294):
- Thread-safe state management with `Arc<RwLock<>>`
- Event-driven transitions via `process_event()`
- Lifecycle hooks via `StateHandler` trait
- History tracking with configurable retention (default 1000 entries)
- Serialization/deserialization for persistence

**WorkflowOrchestrator** (line 616):
- Manages multiple workflow instances
- Thread-safe registry using `HashMap<String, Arc<WorkflowStateMachine>>`
- CRUD operations: register, get, list, unregister

#### Workflow Executor (`workflow_executor.rs`)

**WorkflowExecutorConfig** (line 29):
```rust
provider: "anthropic"
model: "claude-sonnet-4-20250514"
max_parallel: 3  // concurrent steps
save_outputs: true
```

**execute_workflow()** (line 195):
- Batches consecutive parallel steps
- Uses semaphore for concurrency control
- Saves outputs to thoughts directory

#### Swarm Parser (`swarm_parser.rs`)

Parses declarative TOML workflow configurations:

**SwarmConfig** (line 61):
- `agents: HashMap<String, AgentConfig>` - Agent definitions with model, tokens, temperature
- `workflows: Vec<Workflow>` - Multiple workflow definitions
- `resources: HashMap<String, ResourceConfig>` - HTTP, Webhook, Database, Custom

**State** (line 147):
- `agents: Vec<String>` - Assigned agents
- `handlers: Vec<Handler>` - Event-triggered transitions
- `parallel_execution: bool` - Enable parallel agent execution
- `timeout_seconds`, `timeout_target` - Timeout handling

**Validation**:
- Config structure validation
- Workflow semantics validation
- DAG cycle detection via DFS
- Reachability analysis from initial state

#### DAG Implementation (`dag.rs`)

**DAG** (line 322):
- Nodes with metadata, tags, 2D positions (for visual editor)
- Edges with types: Dependency, SoftDependency, OptionalDependency, DataFlow, Trigger
- Adjacency lists for efficient traversal

**Key Operations**:
- `topological_sort()` - Kahn's algorithm (line 817)
- `find_critical_path()` - Longest path through DAG (line 1041)
- `detect_cycles()` - Find all cycles (line 699)
- `find_dependencies()` / `find_dependents()` - Transitive closure (lines 607, 630)

**DAGWithHistory** (line 1308):
- Undo/redo functionality for visual editing
- Records AddNode, RemoveNode, UpdateNode, AddEdge, RemoveEdge operations

#### DAG ↔ Swarm Export (`dag_swarm_export.rs`)

Bidirectional conversion:
- `export_dag_to_swarm_toml()` (line 184) - Visual DAG → Declarative TOML
- `import_swarm_toml_to_dag()` (line 534) - TOML → Visual DAG

---

### 4. SCG Task Storage System

**Location**: `descartes/core/src/scg_task_storage.rs`, `descartes/core/src/scud_plugin.rs`

#### SCG File Format

**Location**: `.scud/tasks/tasks.scg`

```
@meta {
  name codelayer
  updated 2025-12-06T21:10:47.023020+00:00
}

@nodes
# id | title | status | complexity | priority
1 | Add subdirectory support | X | 3 | H
1.1 | Add subdirectory constants | P | 0 | H

@edges
# dependent -> dependency
1.2 -> 1.1

@parents
# parent: subtasks...
1: 1.1, 1.2, 1.3

@details
1 | description |
  Extended description here...
```

**Status Codes**: P (Pending), I (InProgress), X (Done), B (Blocked)
**Complexity**: Fibonacci (1, 2, 3, 5, 8, 13, 21)

#### ScgTaskStorage (`scg_task_storage.rs:19`)

- In-memory cache with `RwLock<HashMap<String, ScudPhase>>`
- Async operations wrapping synchronous SCUD storage
- Conversion between Descartes `Task` and SCUD `ScudTask` types

#### Key Operations

- `get_phases()` - Get all phases from cache
- `get_active_phase()` - Get current working phase
- `set_active_phase()` - Switch working context
- `save_phase()` - Persist phase to disk
- `get_next_task()` - Find first Pending task with met dependencies

#### Task Queries (`task_queries.rs`)

SQLite-based query builder for alternative storage:
- Filter by status, priority, complexity, assignee
- Sort by any field, ascending/descending
- Pagination with offset/limit
- Dependency traversal queries

---

### 5. Documentation and Logging Mechanisms

#### Thoughts Directory Structure

```
thoughts/shared/
├── research/   # 24 research documents
│   ├── 2025-12-25-project-progress-and-plan-completion.md
│   └── ...
└── plans/      # 27 plan documents
    ├── 2025-12-25-low-hanging-fruit-cleanup.md
    └── ...
```

#### Document Naming Convention
- Format: `YYYY-MM-DD-ENG-XXXX-description.md` (ticket optional)
- Location: `thoughts/shared/research/` or `thoughts/shared/plans/`

#### Frontmatter Structure

```yaml
---
date: [ISO format with timezone]
researcher: [name]
git_commit: [hash]
branch: [branch name]
repository: [repo name]
topic: "[topic]"
tags: [research, codebase, component-names]
status: complete
last_updated: [YYYY-MM-DD]
last_updated_by: [name]
---
```

#### Additional Documentation Locations

- `.scud/docs/` - SCUD-specific documentation
- `working_docs/reports/` - Phase completion reports
- `working_docs/implementation/` - Implementation guides
- `descartes/core/docs/` - Task query documentation

---

## Architecture Documentation

### Existing Orchestration Patterns

#### 1. Role-Based Sequential Workflow (SCUD)
- Phase gates enforce sequential progression
- Each role has explicit boundaries (I DO / I DO NOT)
- Handoff protocol with state JSON updates
- Tag-based organization for parallel work streams

#### 2. Parallel Subagent Delegation (cl: skills)
- Main agent synthesizes, subagents research
- TodoWrite for task tracking
- Parallel spawning with Task tool
- Wait-for-all-before-synthesize pattern

#### 3. Event-Driven State Machine (Descartes)
- State transitions via events
- Lifecycle hooks for extensibility
- Multiple workflow instance management
- History for debugging and rollback

#### 4. DAG-Based Dependency Management
- Topological sort for execution order
- Critical path analysis
- Cycle detection and validation
- Wave computation from dependency graph

### Context Management

The codebase addresses context limits through:

1. **Subagent Delegation** - Offload research to specialized agents
2. **Phase-Based Work** - Focus on one phase at a time
3. **Tag Organization** - Separate work streams by feature/component
4. **Wave Execution** - Parallelize independent tasks within context budget

---

## Code References

### SCUD Role Commands
- `.claude/commands/scud/pm.md` - Product Manager role
- `.claude/commands/scud/sm.md` - Scrum Master role (PRD parsing)
- `.claude/commands/scud/architect.md` - Architect role
- `.claude/commands/scud/dev.md` - Developer role
- `.claude/commands/scud/status.md` - Workflow status display

### cl: Skills
- `.claude/commands/cl/create_plan.md` - Implementation planning
- `.claude/commands/cl/implement_plan.md` - Plan execution
- `.claude/commands/cl/research_codebase.md` - Codebase documentation
- `.claude/commands/cl/iterate_plan.md` - Plan iteration

### Orchestration Core
- `descartes/core/src/state_machine.rs:294` - WorkflowStateMachine
- `descartes/core/src/state_machine.rs:616` - WorkflowOrchestrator
- `descartes/core/src/workflow_executor.rs:195` - execute_workflow()
- `descartes/core/src/swarm_parser.rs:61` - SwarmConfig
- `descartes/core/src/dag.rs:322` - DAG implementation

### Task Storage
- `descartes/core/src/scg_task_storage.rs:19` - ScgTaskStorage
- `descartes/core/src/scud_plugin.rs` - SCUD CLI integration
- `descartes/core/src/task_queries.rs` - SQLite query builder

### Agent Definitions
- `.claude/agents/cl/codebase-locator.md`
- `.claude/agents/cl/codebase-analyzer.md`
- `.claude/agents/cl/codebase-pattern-finder.md`
- `descartes/agents/planner.md`
- `descartes/agents/researcher.md`

---

## Key Observations for Proposed Workflow

### What Already Exists

1. **PRD → Tasks Pipeline**: SCUD's `scud parse-prd` command handles PRD parsing
2. **Wave Computation**: `scud waves` computes parallel execution groups
3. **Dependency Analysis**: DAG system provides cycle detection, topological sort, critical path
4. **Subagent Spawning**: cl: skills demonstrate parallel subagent patterns
5. **Plan/Implement Cycle**: cl:create_plan and cl:implement_plan provide the framework
6. **Documentation System**: thoughts/ directory with structured markdown

### Gaps to Consider

1. **Orchestrator Agent**: No explicit "orchestrator" role in SCUD - could be added
2. **QA/Monitor Agent**: No dedicated monitoring agent - retrospective is post-hoc
3. **Continuous Diffing**: No automated diff/review during implementation
4. **Cross-System Integration**: cl: skills and SCUD are somewhat separate systems

### Potential Integration Points

1. **Use SCUD for task structure** + **cl:create_plan for per-task planning**
2. **Use WorkflowOrchestrator** for managing multiple agent sessions
3. **Use DAG validation** for dependency graph fixing
4. **Use thoughts/ system** for documentation trail

---

## Related Research

- `thoughts/shared/research/2025-12-17-plans-status-review.md` - Plan status review
- `thoughts/shared/research/2025-12-13-feature-completeness-review.md` - Feature completeness
- `thoughts/shared/plans/2025-12-02-scud-descartes-unification.md` - SCUD-Descartes unification

---

## Open Questions

1. Should the orchestrator be a SCUD role or a Descartes workflow component?
2. How to handle context limits across long implementation sessions?
3. Should QA agent have write access or be read-only?
4. How to handle failures and rollbacks in the workflow?
5. What's the best way to persist progress across sessions?
