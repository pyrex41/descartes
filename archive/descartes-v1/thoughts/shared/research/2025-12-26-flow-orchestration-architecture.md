---
date: 2025-12-26T18:58:46Z
researcher: Claude
git_commit: 95f077e8e4c616d646bed9fcfe01a0e8d4b107f8
branch: master
repository: descartes
topic: "How Flow Orchestration Works in Descartes"
tags: [research, codebase, flow-workflow, orchestration, multi-agent]
status: complete
last_updated: 2025-12-26
last_updated_by: Claude
---

# Research: How Flow Orchestration Works in Descartes

**Date**: 2025-12-26T18:58:46Z
**Researcher**: Claude
**Git Commit**: 95f077e8e4c616d646bed9fcfe01a0e8d4b107f8
**Branch**: master
**Repository**: descartes

## Research Question

How does the flow orchestration work in the Descartes codebase?

## Summary

The Flow Orchestration system in Descartes is a stateful, multi-agent workflow that transforms Product Requirement Documents (PRDs) into implemented code through a series of 6 sequential phases. Each phase is handled by a specialized agent with specific responsibilities and tool capabilities. The system provides pause/resume functionality via persistent state storage and intelligent error recovery through a meta-orchestrator agent.

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                         CLI Entry Point                               │
│              descartes workflow flow --prd <file>                     │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                         FlowExecutor                                  │
│  - State management (.scud/flow-state.json)                          │
│  - Phase sequencing                                                   │
│  - Error recovery via orchestrator                                    │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│ Phase 1:      │    │ Phase 2:      │    │ Phase 3:      │
│ Ingest        │ →  │ Review Graph  │ →  │ Plan Tasks    │
│ (researcher)  │    │ (researcher)  │    │ (planner)     │
└───────────────┘    └───────────────┘    └───────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│ Phase 4:      │    │ Phase 5:      │    │ Phase 6:      │
│ Implement     │ →  │ QA            │ →  │ Summarize     │
│ (orchestrator)│    │ (researcher)  │    │ (readonly)    │
└───────────────┘    └───────────────┘    └───────────────┘
```

## Detailed Findings

### 1. Flow Executor - Core Engine

**Location**: `descartes/core/src/flow_executor.rs`

The `FlowExecutor` struct is the central engine that orchestrates the entire flow workflow.

#### Key Components

**FlowExecutor struct** (lines 177-183):
```rust
pub struct FlowExecutor {
    state: FlowState,           // Current workflow state
    state_path: PathBuf,        // Path to .scud/flow-state.json
    working_dir: PathBuf,       // Project working directory
    agent_loader: AgentDefinitionLoader,  // Loads agent definitions
    backend: Arc<dyn ModelBackend + Send + Sync>, // Model provider
}
```

**FlowState struct** (lines 124-144):
```rust
pub struct FlowState {
    pub version: String,        // Schema version ("1.0")
    pub started_at: Option<DateTime<Utc>>,
    pub prd_file: Option<PathBuf>,
    pub tag: Option<String>,    // Workflow identifier
    pub current_phase: Option<String>,
    pub phases: FlowPhases,     // Status of each phase
    pub config: FlowConfig,     // Execution configuration
    pub artifacts: FlowArtifacts, // Generated file paths
    pub git: FlowGitState,      // Git tracking info
}
```

#### Initialization Methods

**New Flow** (`new()` at lines 187-220):
1. Determines state file path: `.scud/flow-state.json`
2. Loads existing state or creates default
3. Updates state with PRD path, timestamp, and tag
4. Creates `AgentDefinitionLoader` for loading agents

**Resume Flow** (`resume()` at lines 223-245):
1. Reads existing state from `.scud/flow-state.json`
2. Recreates executor with preserved state
3. Allows continuation from interrupted workflow

#### Execution Flow

**Main execute() method** (lines 253-342):

```
Phases 1-3: Sequential
┌─────────┐    ┌──────────────┐    ┌─────────────┐
│ ingest  │ → │ review_graph │ → │ plan_tasks  │
└─────────┘    └──────────────┘    └─────────────┘

Phases 4-6: Sequential (with QA monitoring implementation progress)
┌───────────┐    ┌────┐    ┌───────────┐
│ implement │ → │ qa │ → │ summarize │
└───────────┘    └────┘    └───────────┘
```

Each phase:
1. Checks if already completed (skip if so)
2. Calls `execute_phase(phase_name)`
3. Saves state after each phase
4. On error, invokes orchestrator for decision

### 2. Phase Execution

**execute_phase() method** (lines 359-415):

1. **Construct agent name**: `format!("flow-{}", phase.replace('_', "-"))`
   - `"ingest"` → `"flow-ingest"`
   - `"review_graph"` → `"flow-review-graph"`

2. **Verify agent exists**: `agent_loader.agent_exists(&agent_name)`

3. **Create workflow context**: `WorkflowContext::new(working_dir, tag)`

4. **Build task prompt** (lines 381-387):
   ```
   Execute {phase} phase for flow workflow.

   PRD: {prd_file}
   Tag: {tag}
   State file: {state_path}

   Follow your agent instructions to complete this phase.
   ```

5. **Create WorkflowStep** (lines 390-396):
   ```rust
   let step = WorkflowStep {
       name: format!("Flow: {}", phase),
       agent: agent_name,
       task: task.clone(),
       parallel: false,
       output: None,
   };
   ```

6. **Execute via workflow_executor**: `execute_step(&step, &task, &context, backend, &config)`

7. **Update phase status** based on result (Completed or Failed)

### 3. Error Recovery - Orchestrator Agent

**handle_phase_error() method** (lines 418-463):

When a phase fails:
1. Checks for `flow-orchestrator` agent existence
2. Sends error context to orchestrator with decision prompt:
   ```
   Phase '{phase}' failed with error: {error}

   Decide: retry, skip, or abort?
   ```
3. Parses orchestrator's response for decision keywords
4. Returns `true` to continue or `false` to abort

The orchestrator agent (`flow-orchestrator.md`) provides:
- Decision framework (retry, skip, abort)
- Severity assessment (critical, recoverable, ignorable)
- Structured response format

### 4. Flow Agents - Specialized Handlers

All 7 agents are bundled in `descartes/agents/flow-*.md`:

| Agent | Tool Level | Phase | Primary Responsibilities |
|-------|------------|-------|-------------------------|
| `flow-ingest` | researcher | 1 | Parse PRD into SCUD tasks via `scud parse-prd` |
| `flow-review-graph` | researcher | 2 | Analyze/optimize task dependency graph |
| `flow-plan-tasks` | planner | 3 | Generate implementation plans for complex tasks |
| `flow-implement` | orchestrator | 4 | Execute tasks wave-by-wave with sub-agents |
| `flow-qa` | researcher | 5 | Monitor quality, document intent trail |
| `flow-summarize` | readonly | 6 | Generate comprehensive workflow summary |
| `flow-orchestrator` | orchestrator | N/A | Error recovery and decision making |

#### Agent Definition Structure

Each agent is a markdown file with YAML frontmatter:

```markdown
---
name: flow-ingest
description: Parse PRD into SCUD tasks with dependency mapping
model: claude-3-sonnet
tool_level: researcher
tags: [flow, workflow, prd, parsing]
---

# Flow Ingest

You are the ingest agent for the flow workflow...
```

**Key fields**:
- `name`: Agent identifier
- `description`: Purpose summary
- `model`: Preferred model (all use claude-3-sonnet)
- `tool_level`: Determines available tools
- `tags`: Categorization
- Body: System prompt instructions

### 5. Agent Bundling System

**Location**: `descartes/core/src/agent_definitions.rs`

#### Compile-Time Bundling

Agents are embedded at compile time using `include_str!` (lines 124-140):

```rust
const DEFAULT_AGENT_FLOW_INGEST: &str = include_str!("../../agents/flow-ingest.md");
const DEFAULT_AGENT_FLOW_IMPLEMENT: &str = include_str!("../../agents/flow-implement.md");
// ... etc
```

Registered in `DEFAULT_AGENTS` array (lines 143-161):
```rust
const DEFAULT_AGENTS: &[(&str, &str)] = &[
    ("flow-ingest.md", DEFAULT_AGENT_FLOW_INGEST),
    ("flow-implement.md", DEFAULT_AGENT_FLOW_IMPLEMENT),
    // ...
];
```

#### Runtime Installation

`AgentDefinitionLoader::ensure_default_agents()` (lines 196-205):
- Iterates through `DEFAULT_AGENTS`
- Writes to `~/.descartes/agents/{filename}` if not present
- Existing files are preserved (allows user customization)

#### Agent Loading

`load_agent(name)` (lines 230-249):
1. Normalizes name (adds `.md` if missing)
2. Reads from `~/.descartes/agents/{name}.md`
3. Parses markdown with frontmatter via `AgentDefinition::from_markdown()`
4. Returns `AgentDefinition` struct with:
   - `name`, `description`, `model`
   - `tool_level` (mapped to `ToolLevel` enum)
   - `system_prompt` (markdown body)

### 6. Workflow Executor - Step Execution

**Location**: `descartes/core/src/workflow_executor.rs`

#### execute_step() Function (lines 89-192)

1. **Load agent definition**: `context.agent_loader.load_agent(&step.agent)`
2. **Get tools**: `get_tools(agent_def.tool_level)`
3. **Format task**: Combines topic, task, and context
4. **Create ModelRequest**: User message + system prompt + tools
5. **Execute**: `backend.complete(request).await`
6. **Save output**: Optional save to thoughts directory

#### Tool Levels

`ToolLevel` enum determines available tools:

| Level | Tools |
|-------|-------|
| `Minimal` | read, write, edit, bash |
| `Orchestrator` | minimal + spawn_session |
| `ReadOnly` | read, bash (restricted) |
| `Researcher` | read, bash (read-only) |
| `Planner` | read, bash, write (to thoughts) |
| `LispDeveloper` | swank_eval/compile/inspect + read, bash |

### 7. CLI Integration

**Location**: `descartes/cli/src/commands/workflow.rs`

#### Flow Command Definition (lines 85-106)

```rust
#[command(name = "flow")]
Flow {
    #[arg(long)]
    prd: PathBuf,           // Required: PRD file path

    #[arg(long)]
    tag: Option<String>,    // Optional: workflow tag

    #[arg(long)]
    resume: bool,           // Resume from previous state

    #[arg(short, long)]
    dir: Option<PathBuf>,   // Working directory

    #[arg(long)]
    adapter: Option<String>, // Model adapter (claude-code, opencode)
}
```

#### execute_flow() Function (lines 393-467)

1. **Display info**: PRD path, tag, resume status
2. **Create backend**: Direct provider or headless CLI adapter
3. **Create executor**:
   - `FlowExecutor::resume()` if `--resume` flag
   - `FlowExecutor::new()` otherwise
4. **Execute**: `executor.execute().await`
5. **Display results**: Phases completed/failed, duration, summary path

### 8. State Persistence

**State file**: `.scud/flow-state.json`

**FlowPhases struct** (lines 107-121):
```rust
pub struct FlowPhases {
    pub ingest: PhaseState,
    pub review_graph: PhaseState,
    pub plan_tasks: PhaseState,
    pub implement: PhaseState,
    pub qa: PhaseState,
    pub summarize: PhaseState,
}
```

**PhaseState struct** (lines 35-42):
```rust
pub struct PhaseState {
    pub status: PhaseStatus,  // Pending, Active, Completed, Failed, Skipped
    pub completed_at: Option<DateTime<Utc>>,
    pub data: serde_json::Value,  // Phase-specific data
}
```

**save_state() method** (lines 483-494):
1. Ensures `.scud` directory exists
2. Serializes state to pretty JSON
3. Writes to `.scud/flow-state.json`
4. Called after each phase execution

## Code References

### Core Implementation
- `descartes/core/src/flow_executor.rs:1-540` - FlowExecutor and state types
- `descartes/core/src/workflow_executor.rs:89-192` - Step execution
- `descartes/core/src/workflow_commands.rs:218-257` - WorkflowContext
- `descartes/core/src/agent_definitions.rs:124-205` - Agent bundling

### Agent Definitions
- `descartes/agents/flow-orchestrator.md` - Error recovery meta-agent
- `descartes/agents/flow-ingest.md` - PRD parsing
- `descartes/agents/flow-review-graph.md` - Dependency optimization
- `descartes/agents/flow-plan-tasks.md` - Implementation planning
- `descartes/agents/flow-implement.md` - Task execution
- `descartes/agents/flow-qa.md` - Quality monitoring
- `descartes/agents/flow-summarize.md` - Documentation generation

### CLI Integration
- `descartes/cli/src/commands/workflow.rs:85-106` - Flow command definition
- `descartes/cli/src/commands/workflow.rs:393-467` - Flow execution handler

### Re-exports
- `descartes/core/src/lib.rs:192-195` - Public API exports

## Architecture Documentation

### Design Patterns

1. **State Machine Pattern**: FlowExecutor manages phase transitions with persistent state
2. **Agent Registry Pattern**: Compile-time bundling with runtime installation
3. **Strategy Pattern**: Different tool levels provide different capabilities
4. **Template Method Pattern**: All phases follow same execute_phase() flow

### Phase Dependencies

```
ingest → review_graph → plan_tasks → implement → qa → summarize
   ↑                                      |
   └──────── Error Recovery ──────────────┘
                   ↓
            flow-orchestrator
```

### Tool Level Hierarchy

```
ReadOnly < Researcher < Planner < Orchestrator
                                      ↓
                              spawn_session (sub-agents)
```

## Related Research

- `thoughts/shared/plans/2025-12-26-flow-workflow-descartes-agents.md` - Implementation plan

## Open Questions

1. **Concurrent QA**: The code notes that implement and qa could run concurrently but currently run sequentially due to borrow checker constraints
2. **Retry Logic**: Phase retry after orchestrator decision is not yet implemented
3. **Wave Parallelism**: The implement agent notes processing up to 3 concurrent tasks per wave, but the orchestration of sub-agents is delegated to the agent prompt rather than executor code
