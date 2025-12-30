# Product Requirements Document: Descartes Unified Platform
## *CLI Tools, Plugin Ecosystem, and Guided Webapp for AI Agent Orchestration*

---

## Version History
| Version | Date | Author | Status | Notes |
|---------|------|--------|--------|-------|
| 1.0 | December 30, 2025 | AI-assisted | Draft | Initial unified platform PRD |

---

## Executive Summary

**Descartes Unified Platform** extends the existing Descartes AI agent framework into a three-tier ecosystem:

1. **Standalone CLI Tools** - Extractable, composable command-line utilities
2. **Claude Code Plugin System** - Ralph-Wiggum-style plugins for iterative loops
3. **Guided Webapp** - Browser-based orchestration with Fly.io Machines backend

The platform transforms Descartes from a local development tool into a **cloud-native, guided workflow system** that democratizes AI-assisted development through progressive disclosure, comprehensive tutorials, and managed agent orchestration.

### Core Value Proposition

- **CLI Users**: Get focused, single-purpose tools that compose via Unix pipes
- **Claude Code Users**: Get powerful plugins for autonomous development loops
- **Webapp Users**: Get a guided experience with step-by-step workflows and managed infrastructure

---

## 1. Problem Statement

### 1.1 Current State Analysis

Descartes currently exists as a monolithic Rust workspace with tightly coupled components:

```
descartes/
├── core/     # ~70 modules, all interdependent
├── cli/      # Commands tightly coupled to core
├── daemon/   # Monolithic RPC server
└── gui/      # Native desktop only
```

**Limitations:**

| Problem | Impact |
|---------|--------|
| **Monolithic CLI** | Users must install entire system for simple tasks |
| **No plugin architecture** | Cannot extend Claude Code without forking |
| **Desktop-only** | No browser access; no collaborative workflows |
| **Expert-only UX** | No guidance for new users; steep learning curve |
| **Local execution only** | Cannot leverage cloud compute for heavy workloads |

### 1.2 Market Opportunity

| Segment | Need | Current Gap |
|---------|------|-------------|
| **Individual developers** | Quick AI task automation | Too complex to set up |
| **Teams** | Shared workflows and visibility | No collaborative features |
| **Enterprises** | Audit trails, managed infra | Self-hosted complexity |
| **New users** | Guided onboarding | No tutorial system |

### 1.3 Competitive Landscape

| Competitor | Strengths | Weaknesses |
|------------|-----------|------------|
| **Claude Code** | Integrated, polished | Limited extensibility |
| **Cursor** | IDE integration | Model lock-in, closed |
| **Aider** | CLI-focused, open | Single-agent only |
| **Devin** | Autonomous, cloud | Black box, expensive |

**Opportunity**: A unified platform offering CLI flexibility, plugin extensibility, AND guided webapp experience.

---

## 2. Vision & Objectives

### 2.1 Product Vision

> **"Every developer deserves access to AI agent orchestration—from command line to cloud, from novice to expert."**

Descartes Unified Platform creates a spectrum of access:

```
                      Complexity
        Low ◄─────────────────────────► High

        Webapp          CLI Tools        Raw Library
        (Guided)        (Composable)     (Maximum Control)
           │                │                  │
           ▼                ▼                  ▼
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │ Step-by-step │  │ dc-spawn     │  │ use descartes│
    │ wizards with │  │ dc-parse     │  │   ::core::*  │
    │ explanations │  │ dc-waves     │  │              │
    │ and defaults │  │ dc-flow      │  │ Full Rust API│
    └──────────────┘  └──────────────┘  └──────────────┘
```

### 2.2 Strategic Objectives

1. **Modularize** - Extract 8-10 standalone CLI tools from monolithic codebase
2. **Pluginize** - Create Claude Code plugin system following Ralph-Wiggum patterns
3. **Webify** - Build guided webapp with Fly.io Machines for cloud agent execution
4. **Guide** - Implement progressive disclosure with embedded tutorials
5. **Scale** - Enable horizontal scaling via ephemeral cloud workers

### 2.3 Success Metrics

| Category | Metric | Target |
|----------|--------|--------|
| **Adoption** | Weekly active users (webapp) | 500+ within 6 months |
| **Engagement** | Avg. tasks per session | 5+ |
| **Activation** | Tutorial completion rate | 70%+ |
| **Retention** | 30-day return rate | 40%+ |
| **Performance** | Agent spawn time (cloud) | <500ms |
| **Reliability** | Successful task completion | 95%+ |

---

## 3. Feature Requirements

### 3.1 Standalone CLI Tools

Extract focused tools from the monolithic Descartes codebase.

#### 3.1.1 Tool Catalog

| Tool | Source | Purpose | Priority |
|------|--------|---------|----------|
| `dc-spawn` | cli/spawn.rs | Spawn minimal agent with 4 tools | P0 |
| `dc-parse` | core/scud_plugin.rs | PRD → SCUD task graph | P0 |
| `dc-waves` | core/scud_loop.rs | Visualize parallel execution | P0 |
| `dc-next` | core/scud_plugin.rs | Get next ready task | P1 |
| `dc-flow` | core/flow_executor.rs | Run 6-phase workflow | P1 |
| `dc-transcript` | core/session_transcript.rs | View/query session JSON | P1 |
| `dc-attach` | cli/attach.rs | Attach TUI to remote agent | P2 |
| `dc-doctor` | cli/doctor.rs | Diagnose system setup | P2 |

#### 3.1.2 Tool Design Principles

```bash
# Unix philosophy: do one thing well
dc-parse requirements.md | dc-waves --format=mermaid

# Composable via pipes
dc-next --project=myapp | dc-spawn --task=-

# JSON-first for scripting
dc-spawn --task="fix bug" --output=json | jq '.session_id'

# Human-friendly defaults
dc-waves  # Pretty-prints to terminal
dc-waves --format=json  # Machine-readable
```

#### 3.1.3 Functional Requirements

**dc-spawn** (P0)
- MUST spawn agent with configurable tool level (minimal/orchestrator/readonly)
- MUST support multiple providers (Anthropic, OpenAI, Ollama)
- MUST stream output to stdout with progress indicators
- MUST generate session transcript in .scud/sessions/
- SHOULD support --dry-run for task preview
- COULD support --attach for interactive mode

**dc-parse** (P0)
- MUST accept PRD document (markdown, plain text)
- MUST generate SCUD-compatible task graph (SCG format)
- MUST detect task dependencies automatically
- MUST validate output against SCG schema
- SHOULD support --model flag for different AI providers
- COULD support incremental parsing for large documents

**dc-waves** (P0)
- MUST read SCUD task state from .scud/
- MUST visualize parallel execution waves
- MUST show task status (pending/in_progress/completed)
- MUST support multiple output formats (terminal, mermaid, json)
- SHOULD highlight critical path
- COULD animate real-time progress

#### 3.1.4 Distribution Strategy

```yaml
# Cargo workspace with feature flags
[workspace]
members = ["descartes-cli", "dc-spawn", "dc-parse", "dc-waves", ...]

# Individual tool installation
cargo install dc-spawn
cargo install dc-parse

# Full suite
cargo install descartes-tools

# Homebrew formula
brew install pyrex41/tap/descartes-tools
```

---

### 3.2 Claude Code Plugin System

Implement Ralph-Wiggum-style plugins for Claude Code integration.

#### 3.2.1 Plugin Architecture

```
.claude/plugins/descartes/
├── .claude-plugin/
│   └── manifest.json          # Plugin metadata
├── commands/
│   ├── scud-loop.md           # /scud-loop command
│   ├── flow-start.md          # /flow-start command
│   ├── wave-execute.md        # /wave-execute command
│   └── cancel-flow.md         # /cancel-flow command
├── hooks/
│   ├── stop-hook.sh           # Intercept session exit
│   ├── pre-tool-hook.sh       # Validate tool calls
│   └── completion-hook.sh     # Detect task completion
└── scripts/
    ├── scud-bridge.sh         # Bridge to SCUD CLI
    ├── wave-scheduler.sh      # Parallel task scheduling
    └── progress-tracker.sh    # Track completion state
```

#### 3.2.2 Plugin Catalog

| Plugin | Purpose | Complexity |
|--------|---------|------------|
| **scud-loop** | Iterative SCUD task execution with completion detection | Medium |
| **flow-orchestrator** | PRD→code 6-phase pipeline | High |
| **wave-executor** | Parallel wave-based task runner | Medium |
| **transcript-viewer** | Session observability and replay | Low |
| **guidance-mode** | Step-by-step tutorial overlay | Medium |

#### 3.2.3 Plugin Requirements

**scud-loop Plugin** (P0)
```markdown
# Command: /scud-loop
Usage: /scud-loop "<task-filter>" --max-iterations <n> --completion-promise "<tag>"

Initiates a SCUD-aware iterative loop that:
1. Queries SCUD for next ready task matching filter
2. Executes task with Claude agent
3. Detects completion via promise tag or task state change
4. Advances to next task or terminates after max iterations
```

- MUST integrate with existing SCUD CLI
- MUST respect task dependencies (only execute ready tasks)
- MUST detect completion via configurable promise tag
- MUST support max iteration limit
- MUST provide /cancel-scud-loop escape hatch
- SHOULD track and report iteration metrics
- COULD support parallel task execution within waves

**flow-orchestrator Plugin** (P1)
- MUST implement 6-phase Flow workflow (Ingest→Review→Plan→Implement→QA→Summarize)
- MUST generate handoff documents between phases
- MUST pause for human approval at phase boundaries
- MUST support resume from any phase
- SHOULD provide phase-specific prompts and guidance
- COULD integrate with external project management tools

**wave-executor Plugin** (P1)
- MUST read wave structure from SCUD
- MUST execute all tasks in a wave before proceeding
- MUST handle task failures gracefully
- MUST report wave completion status
- SHOULD support configurable parallelism
- COULD spawn sub-agents for parallel execution

#### 3.2.4 Hook Specifications

**stop-hook.sh** (Iteration Control)
```bash
#!/bin/bash
# Intercepts session exit to enable iteration loops

STATE_FILE=".claude/plugins/descartes/.loop-state"

if [[ -f "$STATE_FILE" ]]; then
    source "$STATE_FILE"
    if [[ "$LOOP_ACTIVE" == "true" && "$ITERATIONS" -lt "$MAX_ITERATIONS" ]]; then
        # Increment iteration counter
        echo "ITERATIONS=$((ITERATIONS + 1))" >> "$STATE_FILE"
        # Re-inject prompt (signal to continue)
        exit 1  # Non-zero prevents exit
    fi
fi
exit 0  # Allow normal exit
```

**completion-hook.sh** (Success Detection)
```bash
#!/bin/bash
# Checks if completion promise tag was emitted

PROMISE_TAG="${COMPLETION_PROMISE:-TASK_COMPLETE}"
TRANSCRIPT=$(cat .scud/sessions/current.json)

if echo "$TRANSCRIPT" | grep -q "$PROMISE_TAG"; then
    echo "LOOP_ACTIVE=false" > "$STATE_FILE"
fi
```

---

### 3.3 Guided Webapp

Browser-based orchestration platform with comprehensive guidance and Fly.io integration.

#### 3.3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           WEB FRONTEND (SvelteKit)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Wizard    │  │   Monitor   │  │  Task Board │  │    Guidance Panel   │ │
│  │    Flow     │  │  Dashboard  │  │   (Kanban)  │  │  (Step-by-step)     │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                │                     │           │
│         └────────────────┴────────────────┴─────────────────────┘           │
│                                    │                                         │
│                            WebSocket + REST                                  │
└────────────────────────────────────┼────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────────────────────┐
│                       ORCHESTRATOR API (Fly App - Always On)                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │    Auth     │  │   Project   │  │   Machine   │  │     Guidance        │ │
│  │  (Clerk/    │  │   Manager   │  │   Spawner   │  │     Content         │ │
│  │   Auth0)    │  │             │  │  (Fly API)  │  │     Service         │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                │                     │           │
│         └────────────────┴────────────────┴─────────────────────┘           │
│                                    │                                         │
│                       Fly Machines API (POST /v1/apps/{app}/machines)       │
└────────────────────────────────────┼────────────────────────────────────────┘
                                     │
          ┌──────────────────────────┼──────────────────────────┐
          │                          │                          │
          ▼                          ▼                          ▼
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│  Worker Machine  │      │  Worker Machine  │      │  Worker Machine  │
│    (Ephemeral)   │      │    (Ephemeral)   │      │    (Ephemeral)   │
│                  │      │                  │      │                  │
│  descartes-worker│      │  descartes-worker│      │  descartes-worker│
│  Docker image    │      │  Docker image    │      │  Docker image    │
│                  │      │                  │      │                  │
│  - 4 core tools  │      │  - 4 core tools  │      │  - 4 core tools  │
│  - Isolated      │      │  - Isolated      │      │  - Isolated      │
│    workspace     │      │    workspace     │      │    workspace     │
│  - Auto-shutdown │      │  - Auto-shutdown │      │  - Auto-shutdown │
└──────────────────┘      └──────────────────┘      └──────────────────┘
```

#### 3.3.2 Frontend Components

**Wizard Flow** (P0)
Progressive multi-step interface for task creation:

```
Step 1: Describe Your Goal
┌─────────────────────────────────────────────────────────────┐
│  What would you like to build?                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ I want to add user authentication to my Express app     ││
│  │                                                          ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  💡 Tips:                                                   │
│  • Be specific about the technology stack                  │
│  • Mention any constraints (no external deps, etc)         │
│  • Describe the expected outcome                           │
│                                                             │
│  [Examples ▾]  [Continue →]                                │
└─────────────────────────────────────────────────────────────┘

Step 2: Review Generated Tasks
┌─────────────────────────────────────────────────────────────┐
│  We've broken your goal into 5 tasks:                      │
│                                                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                │
│  │ Task 1  │───▶│ Task 2  │───▶│ Task 3  │                │
│  │ Setup   │    │  Model  │    │  Routes │                │
│  └─────────┘    └─────────┘    └────┬────┘                │
│                                     │                       │
│                      ┌──────────────┴──────────────┐       │
│                      ▼                             ▼       │
│                 ┌─────────┐                   ┌─────────┐  │
│                 │ Task 4  │                   │ Task 5  │  │
│                 │  Tests  │                   │  Docs   │  │
│                 └─────────┘                   └─────────┘  │
│                                                             │
│  📖 Why this structure?                                    │
│  [Edit Tasks]  [Approve & Start →]                         │
└─────────────────────────────────────────────────────────────┘

Step 3: Watch & Guide
┌─────────────────────────────────────────────────────────────┐
│  Wave 1: Tasks 1-2 executing in parallel                   │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 67%            │
│                                                             │
│  Agent 1 (Task 1: Setup)                                   │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ ✓ Created package.json                                  ││
│  │ ✓ Installed dependencies                                ││
│  │ ⟳ Configuring Express middleware...                     ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  ⚠️ Approval Required:                                      │
│  Agent wants to install `passport` (npm package)           │
│  [Approve] [Reject] [Ask Why]                              │
└─────────────────────────────────────────────────────────────┘
```

**Monitor Dashboard** (P0)
Real-time visibility into all running agents:

- Agent status grid (active/idle/completed/failed)
- Resource utilization (CPU, memory per machine)
- Cost tracking (Fly.io compute costs)
- Log streaming with search/filter
- Emergency stop (kill all agents)

**Task Board** (P1)
Kanban-style view of SCUD tasks:

| Pending | In Progress | Completed | Blocked |
|---------|-------------|-----------|---------|
| Task 5  | Task 2      | Task 1    | Task 4  |
|         | Task 3      |           | (waiting)|

**Guidance Panel** (P0)
Contextual help that appears alongside main UI:

```
┌─────────────────────────────────────────┐
│ 📘 What's happening now?               │
│                                         │
│ The agent is setting up your project   │
│ structure. This typically involves:    │
│                                         │
│ 1. Creating configuration files        │
│ 2. Installing dependencies             │
│ 3. Setting up the directory structure  │
│                                         │
│ 💡 Pro tip: You can pre-configure      │
│ common dependencies in your project    │
│ template to speed this up.             │
│                                         │
│ [Learn more about project templates →] │
│ [Skip tutorial for this step]          │
└─────────────────────────────────────────┘
```

#### 3.3.3 Orchestrator API Endpoints

**Authentication & Projects**
```
POST   /api/auth/login          # OAuth login (Clerk/Auth0)
POST   /api/auth/logout         # Session termination
GET    /api/projects            # List user projects
POST   /api/projects            # Create new project
GET    /api/projects/:id        # Get project details
DELETE /api/projects/:id        # Delete project
```

**Task Management**
```
POST   /api/projects/:id/parse  # PRD → task graph
GET    /api/projects/:id/tasks  # List tasks
PATCH  /api/projects/:id/tasks/:tid  # Update task
GET    /api/projects/:id/waves  # Get wave structure
POST   /api/projects/:id/execute  # Start execution
```

**Agent Management**
```
POST   /api/agents              # Spawn new agent (Fly Machine)
GET    /api/agents              # List active agents
GET    /api/agents/:id          # Agent status
DELETE /api/agents/:id          # Kill agent
GET    /api/agents/:id/logs     # Stream logs (WebSocket upgrade)
POST   /api/agents/:id/approve  # Approve pending action
```

**Guidance System**
```
GET    /api/guidance/context    # Get guidance for current state
GET    /api/guidance/tutorials  # List available tutorials
POST   /api/guidance/feedback   # Submit guidance feedback
```

#### 3.3.4 Fly.io Integration

**Worker Machine Specification**
```json
{
  "config": {
    "image": "registry.fly.io/descartes-worker:latest",
    "env": {
      "ANTHROPIC_API_KEY": "${secrets.ANTHROPIC_API_KEY}",
      "TASK_ID": "${task.id}",
      "PROJECT_ID": "${project.id}",
      "CALLBACK_URL": "https://api.descartes.dev/agents/${agent.id}/callback"
    },
    "guest": {
      "cpu_kind": "shared",
      "cpus": 2,
      "memory_mb": 2048
    },
    "auto_destroy": true,
    "restart": {
      "policy": "no"
    },
    "services": [
      {
        "internal_port": 8080,
        "protocol": "tcp",
        "ports": [{"port": 443, "handlers": ["tls", "http"]}]
      }
    ]
  }
}
```

**Machine Lifecycle**
```
User Request → Orchestrator → POST /v1/machines → Machine Boots (~300ms)
                                                        │
                                                        ▼
                                              descartes-worker starts
                                                        │
                                                        ▼
                                              Connect to Orchestrator via WS
                                                        │
                                                        ▼
                                              Execute task with streaming
                                                        │
                                                        ▼
                                              Task complete → Machine stops
                                                        │
                                                        ▼
                                              Auto-destroyed (Fly auto_destroy)
```

**Cost Optimization**
- Shared CPU for most tasks ($0.007/hour)
- Performance CPU for complex reasoning ($0.06/hour)
- Auto-destroy on completion (no idle costs)
- Regional selection based on user location
- Spot instances for batch processing

#### 3.3.5 Guidance System

**Content Structure**
```yaml
# guidance/task-parsing.yaml
id: task-parsing
title: "Understanding Task Breakdown"
trigger:
  state: "task_review"
  first_time: true
content:
  summary: |
    We've analyzed your requirements and broken them into tasks.
    Each task has dependencies - some must complete before others can start.

  details:
    - title: "Why break into tasks?"
      body: |
        Breaking work into tasks allows:
        - Parallel execution (faster completion)
        - Better error isolation
        - Clearer progress tracking

    - title: "What are dependencies?"
      body: |
        Dependencies ensure tasks execute in the right order.
        For example, you can't test code that doesn't exist yet.

  actions:
    - label: "Show me an example"
      action: "show_example"
      example_id: "task-graph-simple"

    - label: "I understand, continue"
      action: "dismiss"

  learn_more: "https://docs.descartes.dev/concepts/task-graphs"
```

**Progressive Disclosure Levels**
1. **Beginner** - Full explanations, automatic suggestions, guardrails
2. **Intermediate** - Concise tips, optional explanations
3. **Expert** - Minimal guidance, keyboard shortcuts, advanced options

**Contextual Triggers**
- First-time actions (first task creation, first approval)
- Error states (task failure, timeout)
- Complex decisions (multiple valid approaches)
- Milestone completion (wave finished, project done)

---

## 4. Technical Architecture

### 4.1 System Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              DESCARTES UNIFIED PLATFORM                       │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────────────┐  │
│   │   CLI Tools     │   │  Claude Plugins │   │      Webapp             │  │
│   │                 │   │                 │   │                         │  │
│   │  dc-spawn       │   │  scud-loop      │   │  SvelteKit Frontend     │  │
│   │  dc-parse       │   │  flow-orch      │   │  Orchestrator API       │  │
│   │  dc-waves       │   │  wave-exec      │   │  Fly.io Workers         │  │
│   │  dc-flow        │   │  guidance       │   │  Guidance Service       │  │
│   │  dc-transcript  │   │                 │   │                         │  │
│   └────────┬────────┘   └────────┬────────┘   └───────────┬─────────────┘  │
│            │                     │                        │                 │
│            └─────────────────────┴────────────────────────┘                 │
│                                  │                                          │
│                                  ▼                                          │
│   ┌──────────────────────────────────────────────────────────────────────┐ │
│   │                        DESCARTES CORE LIBRARY                         │ │
│   │                                                                        │ │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │ │
│   │   │   Providers  │  │   Sessions   │  │    Tools     │               │ │
│   │   │  (Anthropic, │  │  (Transcripts│  │  (read,write │               │ │
│   │   │   OpenAI,    │  │   Lifecycle, │  │   edit,bash) │               │ │
│   │   │   Ollama)    │  │   Restore)   │  │              │               │ │
│   │   └──────────────┘  └──────────────┘  └──────────────┘               │ │
│   │                                                                        │ │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │ │
│   │   │  Flow        │  │  SCUD        │  │  Iterative   │               │ │
│   │   │  Executor    │  │  Plugin      │  │  Loop        │               │ │
│   │   │  (6-phase)   │  │  (waves,     │  │  (Ralph-     │               │ │
│   │   │              │  │   tasks)     │  │   style)     │               │ │
│   │   └──────────────┘  └──────────────┘  └──────────────┘               │ │
│   │                                                                        │ │
│   └──────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Component Interactions

**CLI Tools → Core Library**
```rust
// dc-spawn calls core library directly
use descartes_core::{spawn_agent, AgentConfig, ToolLevel};

fn main() {
    let config = AgentConfig {
        task: args.task,
        provider: args.provider,
        tool_level: ToolLevel::Minimal,
        ..Default::default()
    };

    let session = spawn_agent(config)?;
    stream_output(session);
}
```

**Plugins → SCUD CLI + Core**
```bash
# Plugin commands invoke CLI tools
/scud-loop "auth-*" --max-iterations 10

# Internally:
dc-next --filter="auth-*" | dc-spawn --task=- --output=json
```

**Webapp → Fly Machines API**
```typescript
// Orchestrator spawns workers via Fly API
async function spawnWorker(task: Task): Promise<Machine> {
  const response = await fetch(`${FLY_API}/v1/apps/${APP}/machines`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${FLY_TOKEN}` },
    body: JSON.stringify({
      config: {
        image: 'descartes-worker:latest',
        env: { TASK_ID: task.id },
        guest: { cpus: 2, memory_mb: 2048 }
      }
    })
  });
  return response.json();
}
```

### 4.3 Data Flow

**Task Execution Flow**
```
1. User Input (PRD or task description)
           │
           ▼
2. Task Parsing (dc-parse / orchestrator)
           │
           ▼
3. Task Graph Generation (SCUD format)
           │
           ▼
4. Wave Calculation (dependency analysis)
           │
           ▼
5. Agent Spawning (local CLI / Fly Machine)
           │
           ▼
6. Task Execution (streaming output)
           │
           ▼
7. Completion Detection (promise tag / state change)
           │
           ▼
8. State Update (SCUD + transcript)
           │
           ▼
9. Next Wave (if dependencies satisfied)
```

### 4.4 Data Models

**Shared Models (Rust)**
```rust
/// Task representation (SCUD-compatible)
#[derive(Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub assigned_agent: Option<String>,
    pub estimated_complexity: Complexity,
}

#[derive(Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Ready,      // Dependencies satisfied
    InProgress,
    Completed,
    Failed,
    Blocked,
}

/// Wave = set of tasks executable in parallel
#[derive(Serialize, Deserialize)]
pub struct Wave {
    pub index: usize,
    pub tasks: Vec<String>,  // Task IDs
}

/// Project with task graph
#[derive(Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub prd: Option<String>,
    pub tasks: Vec<Task>,
    pub waves: Vec<Wave>,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**API Models (TypeScript)**
```typescript
// Guidance content
interface GuidanceContent {
  id: string;
  title: string;
  trigger: {
    state: string;
    firstTime?: boolean;
    errorType?: string;
  };
  content: {
    summary: string;
    details: GuidanceDetail[];
    actions: GuidanceAction[];
    learnMore?: string;
  };
}

// Machine state
interface AgentMachine {
  id: string;
  taskId: string;
  projectId: string;
  status: 'starting' | 'running' | 'completed' | 'failed';
  flyMachineId: string;
  createdAt: Date;
  logs: LogEntry[];
  pendingApprovals: ApprovalRequest[];
}
```

### 4.5 Security Architecture

**Authentication & Authorization**
```
┌──────────────────────────────────────────────────────────────┐
│                      AUTH FLOW                                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  User → OAuth (Clerk/Auth0) → JWT → Orchestrator API        │
│                                                              │
│  JWT Claims:                                                 │
│  {                                                           │
│    "sub": "user_123",                                        │
│    "org": "org_456",                                         │
│    "role": "developer",                                      │
│    "projects": ["proj_789", "proj_012"]                      │
│  }                                                           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Secrets Management**
- User API keys encrypted at rest (AES-256-GCM)
- Per-project key isolation
- Fly.io secrets for worker env vars
- Key rotation support

**Worker Isolation**
- Each Fly Machine is ephemeral and isolated
- No persistent storage (stateless workers)
- Network isolation between machines
- Firestore-based state (not local disk)

---

## 5. Implementation Roadmap

### Phase 1: CLI Tool Extraction (Weeks 1-3)

**Goals:**
- Extract 3 core tools as standalone binaries
- Establish shared library interface
- Create distribution infrastructure

**Deliverables:**
- [ ] dc-spawn standalone binary
- [ ] dc-parse standalone binary
- [ ] dc-waves standalone binary
- [ ] Cargo workspace restructure
- [ ] Homebrew formula
- [ ] README and CLI documentation

**Technical Work:**
1. Refactor descartes-core into library crate with minimal dependencies
2. Create thin CLI wrappers for each tool
3. Add feature flags for optional dependencies
4. Set up cross-compilation for macOS/Linux/Windows
5. Create GitHub releases workflow

### Phase 2: Plugin System (Weeks 4-6)

**Goals:**
- Implement Claude Code plugin structure
- Create scud-loop plugin with full iteration support
- Document plugin development pattern

**Deliverables:**
- [ ] .claude-plugin manifest specification
- [ ] scud-loop plugin (P0)
- [ ] wave-executor plugin (P1)
- [ ] Plugin installation documentation
- [ ] Example plugin template

**Technical Work:**
1. Define plugin directory structure
2. Implement stop-hook for iteration control
3. Create SCUD CLI bridge scripts
4. Build state persistence layer
5. Test with various completion scenarios

### Phase 3: Webapp MVP (Weeks 7-12)

**Goals:**
- Launch basic webapp with task creation and execution
- Integrate Fly.io Machines for cloud workers
- Implement core guidance system

**Deliverables:**
- [ ] SvelteKit frontend with wizard flow
- [ ] Orchestrator API (auth, projects, agents)
- [ ] Fly.io integration (spawn, monitor, destroy)
- [ ] Basic guidance panel
- [ ] Worker Docker image
- [ ] Deployment to Fly.io

**Technical Work:**
1. Set up SvelteKit project with TailwindCSS
2. Implement OAuth with Clerk
3. Build REST API with Hono/Express
4. Create Fly Machines API client
5. Build WebSocket streaming for logs
6. Design and implement guidance content
7. Containerize descartes-worker

### Phase 4: Advanced Features (Weeks 13-18)

**Goals:**
- Full guidance system with progressive disclosure
- Advanced monitoring and cost tracking
- Plugin marketplace foundation

**Deliverables:**
- [ ] Complete tutorial system
- [ ] Cost dashboard
- [ ] Team collaboration features
- [ ] Plugin registry
- [ ] API rate limiting and quotas
- [ ] Audit logging

### Phase 5: Scale & Polish (Weeks 19-24)

**Goals:**
- Performance optimization
- Enterprise features
- Documentation and community

**Deliverables:**
- [ ] Performance benchmarks
- [ ] SSO/SAML support
- [ ] Comprehensive documentation site
- [ ] Community Discord/forum
- [ ] Video tutorials

---

## 6. Risk Assessment & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Fly.io API changes** | High | Low | Abstract API client; pin versions |
| **Claude Code plugin API changes** | High | Medium | Follow Ralph-Wiggum patterns; minimal hook usage |
| **Cost overruns (cloud compute)** | Medium | Medium | Implement hard limits; show costs prominently |
| **User confusion (too many options)** | High | Medium | Progressive disclosure; strong defaults |
| **Security vulnerabilities** | High | Low | Sandboxed workers; audit trails; code review |
| **Performance at scale** | Medium | Medium | Load testing; horizontal scaling |
| **Low adoption** | High | Medium | Focus on guidance; lower barrier to entry |

---

## 7. Success Criteria

### Technical Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| CLI tool binary size | <10MB each | CI build output |
| Agent spawn time (Fly) | <500ms | P95 latency |
| Webapp page load | <2s | Lighthouse |
| API response time | <200ms | P95 latency |
| Worker uptime | 99.5% | Fly.io metrics |

### User Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| Tutorial completion | 70%+ | Analytics |
| Task success rate | 90%+ | Completion tracking |
| User retention (7-day) | 50%+ | Cohort analysis |
| NPS score | 40+ | In-app survey |
| Support tickets/user | <0.5/month | Helpdesk |

### Business Metrics
| Metric | Target | Measurement |
|--------|--------|-------------|
| Monthly active users | 500+ (6 months) | Auth analytics |
| Paid conversions | 5%+ | Billing |
| Monthly compute cost/user | <$10 | Fly.io billing |
| Churn rate | <10%/month | Subscription data |

---

## 8. Appendices

### A. Glossary

| Term | Definition |
|------|------------|
| **Agent** | AI model instance executing tasks via Descartes tools |
| **SCUD** | Task management system with dependency tracking (SCG format) |
| **Wave** | Set of tasks with satisfied dependencies, executable in parallel |
| **Flow** | 6-phase workflow: Ingest→Review→Plan→Implement→QA→Summarize |
| **Tool Level** | Permission tier (minimal/orchestrator/readonly) |
| **Guidance** | Contextual help and tutorial content |
| **Promise Tag** | Marker indicating task completion (e.g., "TASK_COMPLETE") |
| **Worker** | Ephemeral Fly.io machine running descartes-worker |

### B. External References

- [Fly.io Machines API](https://fly.io/docs/machines/api/)
- [Claude Code Plugin Pattern (Ralph-Wiggum)](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/ralph-wiggum)
- [SCUD Repository](https://github.com/pyrex41/scud)
- [Descartes Core Documentation](./descartes/docs/QUICKSTART.md)

### C. Cost Estimates (Fly.io)

| Resource | Cost | Usage Estimate | Monthly |
|----------|------|----------------|---------|
| Shared CPU (2 vCPU) | $0.007/hr | 100 hrs/user | $0.70/user |
| Memory (2GB) | $0.006/hr | 100 hrs/user | $0.60/user |
| Bandwidth | $0.02/GB | 5GB/user | $0.10/user |
| **Total per user** | | | **~$1.40** |

*At 500 users, estimated infra cost: $700/month*

### D. Alternative Architectures Considered

**1. AWS Lambda instead of Fly Machines**
- Pro: More mature, wider adoption
- Con: Cold start latency (1-3s vs 300ms)
- Decision: Fly for faster UX

**2. Self-hosted workers only**
- Pro: Lower cost for heavy users
- Con: Barrier to entry; no guided experience
- Decision: Cloud-first with self-host option

**3. Electron desktop app instead of webapp**
- Pro: Offline support; native feel
- Con: Installation friction; no collaboration
- Decision: Web-first for accessibility

---

## Conclusion

The Descartes Unified Platform transforms a powerful but complex AI agent framework into an accessible ecosystem spanning CLI tools, Claude Code plugins, and a guided webapp. By meeting users where they are—from command line to cloud—we can democratize AI-assisted development while maintaining the rigor and observability that professionals demand.

The phased approach ensures we deliver value incrementally:
1. **CLI Tools** give power users composable primitives immediately
2. **Plugins** extend Claude Code without forking
3. **Webapp** opens the door to new users through guidance

The estimated effort of 6 months to full platform represents a significant but achievable investment, with clear milestones and measurable outcomes at each phase.

---

*"Dubito, ergo cogito, ergo sum" - I doubt, therefore I think, therefore I am*

*With Descartes Unified Platform: From doubt to clarity, from complexity to guidance.*
