# Flow Workflow

The **Flow Workflow** is a multi-agent orchestration system that transforms Product Requirement Documents (PRDs) into implemented code through a structured 6-phase process.

## Overview

```
descartes workflow flow --prd requirements.md
         │
         ▼
    ┌─────────────────────────────────────────────────────────┐
    │                     FlowExecutor                         │
    │  State: .scud/flow-state.json                           │
    │  Pause/Resume: --resume flag                            │
    │  Error Recovery: flow-orchestrator agent                │
    └─────────────────────────────────────────────────────────┘
         │
         ├──▶ Phase 1: Ingest        (parse PRD → SCUD tasks)
         ├──▶ Phase 2: Review Graph  (optimize dependencies)
         ├──▶ Phase 3: Plan Tasks    (generate implementation plans)
         ├──▶ Phase 4: Implement     (execute tasks wave-by-wave)
         ├──▶ Phase 5: QA            (monitor quality)
         └──▶ Phase 6: Summarize     (generate documentation)
```

## Quick Start

```bash
# Start a new flow from a PRD
descartes workflow flow --prd docs/requirements.md

# Resume an interrupted flow
descartes workflow flow --prd docs/requirements.md --resume

# Use a specific tag and working directory
descartes workflow flow --prd docs/requirements.md --tag my-feature --dir /path/to/project

# Use Claude Code as the backend adapter
descartes workflow flow --prd docs/requirements.md --adapter claude-code
```

## Command Options

| Option | Description |
|--------|-------------|
| `--prd <PATH>` | **Required.** Path to the PRD file |
| `--tag <NAME>` | Tag name for this workflow (auto-generated if not provided) |
| `--resume` | Resume from previous state in `.scud/flow-state.json` |
| `-d, --dir <PATH>` | Working directory (defaults to current directory) |
| `--adapter <NAME>` | Model adapter: `claude-code`, `opencode`, or direct provider |

## The 6 Phases

### Phase 1: Ingest

**Agent:** `flow-ingest` (tool level: researcher)

Parses the PRD into SCUD tasks with dependency mapping.

**What it does:**
1. Reads and analyzes the PRD file
2. Initializes flow state in `.scud/flow-state.json`
3. Creates/selects a tag for this workflow
4. Parses PRD into tasks using `scud parse-prd`
5. Expands complex tasks (complexity >= 13)
6. Computes initial waves

**Output:** SCUD tasks in `.scud/tasks/tasks.scg`

### Phase 2: Review Graph

**Agent:** `flow-review-graph` (tool level: researcher)

Analyzes and optimizes the task dependency graph.

**What it does:**
1. Loads and visualizes the task graph
2. Checks for missing dependencies
3. Detects circular dependencies
4. Identifies parallelization opportunities
5. Suggests and applies wave optimizations

**Output:** Optimized dependency graph

### Phase 3: Plan Tasks

**Agent:** `flow-plan-tasks` (tool level: planner)

Generates implementation plans for complex tasks.

**What it does:**
1. Identifies tasks needing plans (complexity >= 8, has subtasks, critical path)
2. Researches codebase context for each task
3. Creates implementation plan documents
4. Links plans to tasks in flow state

**Output:** Plan files in `thoughts/shared/plans/<date>-<tag>-task-<id>.md`

### Phase 4: Implement

**Agent:** `flow-implement` (tool level: orchestrator)

Executes SCUD tasks following implementation plans.

**What it does:**
1. Processes tasks wave-by-wave
2. Spawns implementation sub-agents for each task
3. Monitors progress and handles failures
4. Commits changes after each wave
5. Tracks completion in flow state

**Output:** Implemented code with wave-by-wave commits

### Phase 5: QA

**Agent:** `flow-qa` (tool level: researcher)

Monitors implementation quality and documents the intent trail.

**What it does:**
1. Watches for new commits during implementation
2. Reviews changes for quality and consistency
3. Documents intent-to-implementation trail
4. Logs issues for follow-up

**Output:** QA log in `.scud/qa-log.json`

### Phase 6: Summarize

**Agent:** `flow-summarize` (tool level: readonly)

Generates comprehensive workflow summary and documentation.

**What it does:**
1. Aggregates data from all phases
2. Generates comprehensive summary document
3. Creates final QA report
4. Updates flow state with completion

**Output:** Summary in `thoughts/shared/reports/<date>-<tag>-summary.md`

## State Management

### Flow State File

The workflow state is persisted to `.scud/flow-state.json`:

```json
{
  "version": "1.0",
  "started_at": "2025-12-26T18:00:00Z",
  "prd_file": "docs/requirements.md",
  "tag": "user-auth",
  "current_phase": "implement",
  "phases": {
    "ingest": { "status": "completed", "completed_at": "2025-12-26T18:05:00Z" },
    "review_graph": { "status": "completed", "completed_at": "2025-12-26T18:07:00Z" },
    "plan_tasks": { "status": "completed", "completed_at": "2025-12-26T18:15:00Z" },
    "implement": { "status": "active" },
    "qa": { "status": "pending" },
    "summarize": { "status": "pending" }
  },
  "config": {
    "orchestrator_model": "opus",
    "implementation_model": "sonnet",
    "qa_model": "sonnet",
    "max_parallel_tasks": 3,
    "auto_commit": true
  },
  "artifacts": {
    "prd_path": "docs/requirements.md",
    "tasks_path": ".scud/tasks/tasks.scg",
    "plans_dir": "thoughts/shared/plans"
  }
}
```

### Phase Statuses

| Status | Description |
|--------|-------------|
| `pending` | Phase not yet started |
| `active` | Phase currently executing |
| `completed` | Phase finished successfully |
| `failed` | Phase encountered an error |
| `skipped` | Phase was skipped (by orchestrator decision) |

### Resume Capability

If a workflow is interrupted, use `--resume` to continue:

```bash
# Original run (interrupted)
descartes workflow flow --prd docs/requirements.md
# ... interrupted during Phase 4 ...

# Resume from where it left off
descartes workflow flow --prd docs/requirements.md --resume
# Skips Phases 1-3, continues from Phase 4
```

## Error Recovery

When a phase fails, the `flow-orchestrator` agent is invoked to make a decision:

```
Phase 'implement' failed with error: Connection timeout

Decision options:
- retry: Re-attempt the phase
- skip: Move to next phase
- abort: Stop the workflow
```

The orchestrator agent analyzes the error severity and context to make intelligent decisions:

| Severity | Typical Decision |
|----------|------------------|
| Critical (e.g., missing PRD) | Abort |
| Recoverable (e.g., network timeout) | Retry |
| Ignorable (e.g., optional step failed) | Skip |

## Agent Tool Levels

Each flow agent has a specific tool level determining its capabilities:

| Agent | Tool Level | Capabilities |
|-------|------------|--------------|
| `flow-ingest` | researcher | read, bash (read-only) |
| `flow-review-graph` | researcher | read, bash (read-only) |
| `flow-plan-tasks` | planner | read, bash, write (to thoughts) |
| `flow-implement` | orchestrator | read, write, edit, bash, spawn_session |
| `flow-qa` | researcher | read, bash (read-only) |
| `flow-summarize` | readonly | read, bash (restricted) |
| `flow-orchestrator` | orchestrator | read, write, edit, bash, spawn_session |

## Generated Artifacts

After a successful flow, you'll have:

```
project/
├── .scud/
│   ├── flow-state.json           # Workflow state
│   ├── qa-log.json               # QA monitoring log
│   └── tasks/
│       └── tasks.scg             # SCUD task definitions
│
└── thoughts/shared/
    ├── plans/
    │   ├── 2025-12-26-feature-task-1.md
    │   ├── 2025-12-26-feature-task-2.md
    │   └── ...
    └── reports/
        ├── 2025-12-26-feature-summary.md
        └── 2025-12-26-feature-qa-final.md
```

## Integration with SCUD

The flow workflow integrates with the SCUD task management system:

```bash
# View tasks created by flow
scud list --tag <flow-tag>

# View task waves
scud waves --tag <flow-tag>

# Check task status
scud show <task-id>
```

## Example: Full Flow Run

```bash
# 1. Create a PRD
cat > docs/prd.md << 'EOF'
# User Authentication Feature

## Requirements
1. Add login endpoint with JWT tokens
2. Add logout endpoint to invalidate tokens
3. Add middleware to protect routes
4. Add user session management
EOF

# 2. Run the flow
descartes workflow flow --prd docs/prd.md --tag user-auth

# Output:
# Flow Workflow
# ═══════════════════════════════════════════════════════
#
# PRD: docs/prd.md
# Tag: user-auth
# Resume: false
#
# Executing flow phases...
#
# ═══════════════════════════════════════════════════════
# Flow Complete!
#
# Phases completed: ["ingest", "review_graph", "plan_tasks", "implement", "qa", "summarize"]
# Phases failed: []
# Summary: thoughts/shared/reports/2025-12-26-user-auth-summary.md
# Duration: 847s

# 3. Review the results
cat thoughts/shared/reports/2025-12-26-user-auth-summary.md
```

## Troubleshooting

### Flow state not found on resume

```
Error: No flow state found. Start a new flow first.
```

**Solution:** Start a new flow without `--resume`, or ensure you're in the correct directory with an existing `.scud/flow-state.json`.

### Agent not found

```
Error: Agent 'flow-ingest' not found
```

**Solution:** Run `descartes init` to ensure default agents are installed in `~/.descartes/agents/`.

### Phase stuck in active state

If a phase shows `"status": "active"` but nothing is running:

1. Check if a process is still running: `descartes ps`
2. If not, manually edit `.scud/flow-state.json` to set status to `"pending"`
3. Resume the flow: `descartes workflow flow --prd <file> --resume`

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                              CLI                                     │
│                  workflow.rs: execute_flow()                         │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         FlowExecutor                                 │
│                    flow_executor.rs                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │ FlowState   │  │ FlowConfig  │  │ FlowPhases  │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Workflow Executor                               │
│                   workflow_executor.rs                               │
│                      execute_step()                                  │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Agent Definition Loader                           │
│                    agent_definitions.rs                              │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ ~/.descartes/agents/flow-*.md                                │   │
│  │   flow-orchestrator.md, flow-ingest.md, flow-review-graph.md │   │
│  │   flow-plan-tasks.md, flow-implement.md, flow-qa.md          │   │
│  │   flow-summarize.md                                          │   │
│  └──────────────────────────────────────────────────────────────┘   │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Model Backend                                 │
│              Anthropic / OpenAI / Ollama / CLI Adapter              │
└─────────────────────────────────────────────────────────────────────┘
```

## Related Documentation

- [Quickstart Guide](QUICKSTART.md) - Getting started with Descartes
- [Skills](SKILLS.md) - Creating custom CLI skills for agents
