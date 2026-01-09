---
date: 2026-01-03T20:57:52Z
researcher: pyrex41
git_commit: 38e9c4bd2146608c4a37f60992638ebdba6dbef4
branch: master
repository: cap
topic: "Comparison of Gastown and Descartes Agent Orchestration Systems"
tags: [research, codebase, gastown, descartes, agent-orchestration, comparison]
status: complete
last_updated: 2026-01-03
last_updated_by: pyrex41
---

# Research: Comparison of Gastown and Descartes Agent Orchestration Systems

**Date**: 2026-01-03T20:57:52Z
**Researcher**: pyrex41
**Git Commit**: 38e9c4bd2146608c4a37f60992638ebdba6dbef4
**Branch**: master
**Repository**: cap

## Research Question

Compare and contrast the Gastown project (from repomix-output.xml) with the current Descartes codebase in the cap repository.

## Summary

Both Gastown and Descartes are AI agent orchestration systems designed to coordinate multiple autonomous AI agents working on software development tasks. While they share similar goals, they differ significantly in implementation language, architecture, communication patterns, and coordination mechanisms.

| Aspect | Gastown | Descartes |
|--------|---------|-----------|
| **Language** | Go | Rust |
| **File Count** | 356 files | ~100 files |
| **CLI Name** | `gt` | `descartes` |
| **Session Manager** | tmux | Unix sockets + ZMQ |
| **State Storage** | JSONL files (beads) | SQLite database |
| **Communication** | Mail protocol | ZMQ pub/sub + RPC |
| **Workflow Model** | Molecules/Wisps | State machines + DAGs |
| **Agent Hierarchy** | Overseer→Mayor→Deacon→Witness→Polecats | Flat (daemon→agents) |

## Detailed Findings

### 1. Core Architecture Differences

#### Gastown Architecture

Gastown implements a **hierarchical supervision model** with specialized agent roles:

```
Overseer (Human)
    └── Mayor (Cross-rig coordinator)
        └── Deacon (Infrastructure, daemon-like)
            └── Witness (Per-rig monitor)
                └── Polecats (Ephemeral workers)
```

Key characteristics:
- **tmux-based**: Agents run in tmux sessions (`gt-mayor`, `gt-deacon`, `gt-{rig}-witness`)
- **JSONL-based state**: Beads stored in `.beads/issues.jsonl` files
- **Bare repo + worktrees**: Shared git refs via `.repo.git/`
- **ZFC (Zero-First-Principles Compliance)**: Go code is "dumb transport," agents self-report state

File locations:
- `internal/cmd/*.go` - CLI commands
- `internal/beads/*.go` - Issue/task tracking
- `internal/daemon/*.go` - Background process
- `internal/rig/*.go` - Repository workspace management

#### Descartes Architecture

Descartes implements a **daemon-centric model** with remote agent management:

```
Daemon (descartes-daemon)
    ├── RPC Server (Unix socket)
    ├── HTTP/WebSocket Server
    ├── ZMQ Publisher (log streaming)
    └── Agent Monitor (lifecycle tracking)

CLI (descartes)
    └── RPC Client → Daemon
```

Key characteristics:
- **Daemon-based**: Central service manages all agents via RPC
- **SQLite state**: State machines and agent state in SQLite
- **ZeroMQ**: Distributed communication for remote agents
- **TUI attachment**: External clients (Claude Code) attach to paused agents

File locations:
- `descartes/core/src/*.rs` - Core library (state machines, DAGs, runners)
- `descartes/daemon/src/*.rs` - Background daemon service
- `descartes/cli/src/*.rs` - CLI commands
- `descartes/gui/` - GUI components

### 2. Communication Patterns

#### Gastown: Mail Protocol

Inter-agent communication uses a structured mail system:

```bash
gt mail send gastown/witness -s "MERGE_READY" -m "Issue gt-abc ready for merge"
gt mail check --inject  # Auto-inject on session start
```

Message types:
- `POLECAT_DONE` - Work completion signal
- `MERGE_READY` - Submit to merge queue
- `MERGE_SUCCESS/FAILED` - Merge results
- `NEEDS_REBASE` - Conflict resolution needed
- `HANDOFF` - Session cycling context

Addressing format: `{rig}/{role}/{name}` (e.g., `gastown/polecats/toast`)

#### Descartes: ZMQ + RPC

Communication uses two channels:

1. **Unix Socket RPC** (CLI ↔ Daemon):
   ```rust
   // JSON-RPC 2.0 over Unix socket
   {"jsonrpc": "2.0", "method": "agent.spawn", "params": {...}, "id": 1}
   ```

2. **ZeroMQ Pub/Sub** (Daemon → Clients):
   - REP socket for request/response (port 5555)
   - PUB socket for log streaming (port 5556)
   - Topics: `agent_id` for filtering

### 3. Workflow Models

#### Gastown: Molecules and Wisps

Workflows follow a physical chemistry metaphor:

```
Formula (TOML template) → "Ice-9"
    ↓ bd cook
Protomolecule (frozen) → Solid
    ↓ bd mol pour
Molecule (persistent) → Liquid → bd squash → Digest
    ↓ bd mol wisp
Wisp (ephemeral) → Vapor → bd burn → (gone)
```

Key concepts:
- **Formula**: Source TOML template with steps
- **Molecule**: Active workflow instance (synced to git)
- **Wisp**: Ephemeral workflow (never synced, for patrol loops)
- **Hook**: Pinned work assignment on agent's handoff bead

#### Descartes: State Machines and DAGs

Workflows use formal state machines and dependency graphs:

```rust
// State Machine States
enum WorkflowState {
    Idle, Running, Paused, Completed, Failed, Cancelled
}

// DAG Edge Types
enum EdgeType {
    Dependency,      // Hard blocking
    SoftDependency,  // Suggested order
    DataFlow,        // Data passing
    Trigger,         // Event-based
}
```

Key concepts:
- **WorkflowStateMachine**: Event-driven state transitions with history
- **DAG**: Directed acyclic graph for task dependencies
- **SCUD Loop**: Wave-based task execution (parallel where possible)
- **Flow Executor**: PRD → Implementation multi-phase workflow

### 4. Agent Management

#### Gastown: Role-Based Agents

Each agent has a specific role with predefined responsibilities:

| Role | Location | Purpose |
|------|----------|---------|
| Mayor | `~/gt/mayor/` | Cross-rig coordination |
| Deacon | `~/gt/deacon/` | Infrastructure, patrol loops |
| Witness | `~/gt/{rig}/witness/` | Per-rig monitoring |
| Refinery | `~/gt/{rig}/refinery/` | Merge queue processing |
| Polecat | `~/gt/{rig}/polecats/{name}/` | Ephemeral workers |
| Crew | `~/gt/{rig}/crew/{name}/` | Persistent workers |

Agent identity via `BD_ACTOR` environment variable:
```bash
BD_ACTOR="gastown/polecats/toast"
GIT_AUTHOR_NAME="gastown/polecats/toast"
```

#### Descartes: Generic Agents

Agents are generic processes managed by the daemon:

```rust
struct AgentConfig {
    name: String,
    model_backend: String,  // "claude", "opencode", etc.
    task: String,
    tool_level: Option<String>,
    environment: HashMap<String, String>,
}
```

Agent lifecycle:
1. CLI: `descartes spawn --task "..." --model claude`
2. Daemon: Spawns process, tracks in registry
3. Monitor: Parses JSON stream output, publishes events
4. Attach: TUI clients connect to paused agents

### 5. Task Tracking

#### Gastown: Beads

Issues/tasks stored in JSONL files:

```json
{
  "id": "gt-abc",
  "title": "Implement feature X",
  "status": "open",
  "created_by": "gastown/crew/joe",
  "hook_bead": "gt-abc.1",
  "dependencies": ["gt-xyz"]
}
```

Storage locations:
- Town-level: `~/gt/.beads/issues.jsonl`
- Rig-level: `~/gt/{rig}/mayor/rig/.beads/issues.jsonl`
- Routes: `~/gt/.beads/routes.jsonl`

#### Descartes: SCUD Tasks

Tasks tracked via SCUD CLI integration:

```json
{
  "id": 1,
  "title": "Implement feature X",
  "status": "pending",
  "dependencies": [2, 3],
  "wave": 1
}
```

State files:
- `.scud/tasks/{tag}.json` - Task definitions
- `.scud/scud-loop-state.json` - Execution state
- `.scud/flow-state.json` - Flow executor state

### 6. Session Management

#### Gastown: tmux Sessions

Sessions managed via tmux with handoff support:

```bash
gt handoff -s "Brief" -m "Details"  # Cycle session with notes
gt prime                             # Load context on start
gt nudge deacon "Check inbox"        # Send message to session
```

Hooks integration:
```json
{
  "SessionStart": [{
    "command": "gt prime && gt mail check --inject"
  }]
}
```

#### Descartes: Daemon + TUI Attach

Sessions managed via daemon with attach capability:

```bash
descartes spawn --task "..."        # Start agent
descartes pause <agent-id>          # Pause for attach
descartes attach <agent-id>         # Get credentials
claude --attach-token <token>       # TUI connects
```

Attach flow:
1. Agent paused (SIGSTOP or cooperative)
2. CLI requests attach credentials
3. Daemon returns token + ZMQ endpoint
4. TUI connects for interactive control

### 7. Persistence and Recovery

#### Gastown: Git-Native

State synced via git:
- Beads committed to JSONL files
- Molecules create audit trails
- Wisps never synced (ephemeral)
- Session handoff via mail

#### Descartes: SQLite + State Files

State persisted locally:
- `state_machine_store.rs` - Workflow state in SQLite
- `.scud/` directory - Loop and flow state as JSON
- Agent registry in DashMap (in-memory with persistence)

## Conceptual Mapping

| Gastown Concept | Descartes Equivalent | Notes |
|-----------------|---------------------|-------|
| Beads | SCUD Tasks / State Store | Different granularity |
| Molecule | Workflow State Machine | Formal vs. informal |
| Wisp | (none) | Ephemeral workflows unique to Gastown |
| Polecat | Agent (via runner) | Ephemeral workers |
| Rig | Working directory | Repository workspace |
| Hook | (none) | Work assignment mechanism |
| Convoy | DAG | Batch work tracking |
| Mail | ZMQ messages / RPC | Inter-agent communication |
| Deacon | Daemon | Background supervision |
| Witness | Agent Monitor | Lifecycle tracking |
| BD_ACTOR | Agent ID | Identity attribution |
| Overseer | Human user | Not modeled in Descartes |

## Architecture Documentation

### Gastown Design Principles

1. **Zero-First-Principles Compliance (ZFC)**: Go code is dumb transport; agents self-report state
2. **Propulsion Principle**: "If you find something on your hook, YOU RUN IT"
3. **Molecule/Wisp Decision**: Auditable work → Molecule; Patrol loops → Wisp
4. **Bare Repo + Worktrees**: Shared refs for parallel work

### Descartes Design Principles

1. **Daemon-Centric**: Central service manages all agent lifecycles
2. **Formal State Machines**: Event-driven transitions with validation
3. **DAG Dependencies**: Topological sort for execution order
4. **ZMQ Distribution**: Remote agent management capability

## Key Differences Summary

### Philosophy

| Aspect | Gastown | Descartes |
|--------|---------|-----------|
| Control model | Decentralized (agents self-govern) | Centralized (daemon controls) |
| State ownership | Agents own their state | Daemon owns all state |
| Communication | Async mail protocol | Sync RPC + async pub/sub |
| Session management | tmux + handoff | Process control + attach |

### Technical

| Aspect | Gastown | Descartes |
|--------|---------|-----------|
| Language | Go | Rust |
| Storage | JSONL files | SQLite |
| IPC | tmux send-keys | Unix sockets + ZMQ |
| Workflow | Formula/Molecule/Wisp | State Machine + DAG |
| Distribution | Single machine (tmux) | Multi-machine (ZMQ) |

### Workflow

| Aspect | Gastown | Descartes |
|--------|---------|-----------|
| Task model | Issues with dependencies | DAG nodes with edges |
| Execution | Hook-based propulsion | SCUD loop waves |
| Completion | Mail signals (POLECAT_DONE) | State transition events |
| Audit | Molecules sync to git | SQLite history tables |

## Similarities

1. **AI Agent Orchestration**: Both coordinate multiple AI agents for software development
2. **State Persistence**: Both persist workflow state for resume/recovery
3. **Dependency Tracking**: Both model task dependencies
4. **Background Supervision**: Both have daemon-like supervision (Deacon vs Daemon)
5. **Lifecycle Management**: Both track agent lifecycle (spawn, run, complete, fail)
6. **CLI Interface**: Both provide CLI tools for human interaction

## Code References

### Gastown (from repomix-output.xml)
- `internal/beads/beads.go` - Issue/task tracking
- `internal/daemon/daemon.go` - Background process
- `internal/cmd/sling.go` - Work assignment
- `internal/cmd/convoy.go` - Batch tracking
- `internal/mail/mailbox.go` - Mail system
- `docs/molecules.md` - Workflow documentation

### Descartes
- `descartes/core/src/lib.rs` - Core module exports
- `descartes/core/src/agent_runner.rs` - Agent spawning
- `descartes/core/src/state_machine.rs` - Workflow state
- `descartes/core/src/dag.rs` - Task dependencies
- `descartes/core/src/zmq_server.rs` - ZMQ server
- `descartes/daemon/src/server.rs` - Daemon server
- `descartes/cli/src/main.rs` - CLI entry point

## Descartes-Specific Advanced Features

### Flow Plugin (PRD → Implementation Workflow)

The Flow system is a 6-phase automation workflow that transforms Product Requirements Documents into implemented code. It exists in both Rust (`flow_executor.rs`) and Claude Code slash commands (`.claude/commands/flow/`).

#### Phases

| Phase | Agent | Tool Level | Purpose |
|-------|-------|------------|---------|
| **Ingest** | `flow-ingest` | researcher | Parse PRD, create SCUD tasks |
| **Review Graph** | `flow-review-graph` | researcher | Validate task dependencies |
| **Plan Tasks** | `flow-plan-tasks` | planner | Generate implementation plans |
| **Implement** | `flow-implement` | orchestrator | Execute tasks wave-by-wave |
| **QA** | `flow-qa` | researcher | Concurrent quality monitoring |
| **Summarize** | `flow-summarize` | readonly | Generate completion summary |

#### Handoff Documents

Flow uses structured handoff documents to enable clean context transfer between sessions:

```markdown
---
type: handoff
phase: research
timestamp: 2026-01-03T12:00:00Z
topic: "Feature implementation"
research_doc: "thoughts/shared/research/2026-01-03-feature.md"
git_commit: "abc123"
next_phase: plan
next_command: "/flow:plan {this-handoff-path}"
---
```

Handoff locations:
- Research: `thoughts/shared/handoffs/research/{date}_{time}_{topic}.md`
- Plan: `thoughts/shared/handoffs/plan/{date}_{time}_{tag}.md`
- Implement: `thoughts/shared/handoffs/implement/{date}_{time}_{tag}.md`

#### Slash Commands

| Command | Purpose |
|---------|---------|
| `/flow:research` | Start research phase with handoff generation |
| `/flow:plan {handoff}` | Continue to planning from research handoff |
| `/flow:implement {tag}` | Execute SCUD tasks wave-by-wave |
| `/flow:status` | Show flow progress across all phases |
| `/flow:resume {handoff}` | Resume from any handoff document |

#### Key Files

- `descartes/core/src/flow_executor.rs` - Rust implementation
- `descartes/core/src/flow_git.rs` - Git checkpoint integration
- `descartes/agents/flow-*.md` - Agent definitions
- `.claude/commands/flow/*.md` - Slash commands

### Ralph Wiggum (Iterative Loop Pattern)

The "Ralph Wiggum" pattern is an iterative, self-referential agent execution loop. Named after the technique of having an agent run repeatedly, seeing its own previous work, and improving until it signals completion.

#### Core Concept

```bash
descartes loop start \
  --command "claude -p" \
  --prompt "Implement feature. Output <promise>COMPLETE</promise> when done." \
  --max-iterations 20
```

The loop:
1. Executes command with prompt
2. Checks output for completion promise (`<promise>COMPLETE</promise>`)
3. If not found, runs again with iteration context
4. Repeats until promise detected or limit reached

#### Iteration Context Injection

After iteration 0, subsequent runs receive:
```
---
ITERATION CONTEXT:
- This is iteration 3 of 20
- Your previous work persists in files and git history
- Review what you've done and continue improving
- Output <promise>COMPLETE</promise> when done
---
```

#### Exit Conditions

| Exit Reason | Description |
|-------------|-------------|
| `CompletionPromiseDetected` | Promise text found in output |
| `MaxIterationsReached` | Safety limit hit |
| `UserCancelled` | Ctrl+C or cancel command |
| `ProcessSuccess` | Exit code 0 (when no promise configured) |
| `Error` | Command failed to execute |

#### Backend System

Three backends for different CLIs:

| Backend | Command | Output Format |
|---------|---------|---------------|
| `claude` | `claude -p --output-format stream-json` | Stream JSON |
| `opencode` | `opencode run --format json` | JSON |
| `generic` | Any command | Text |

#### State Persistence

Loop state persisted to `.descartes/loop-state.json`:
- Iteration count
- Start/last timestamps
- Exit reason
- Iteration summaries (exit codes, output previews)
- Git commit history (if auto-commit enabled)

#### SCUD Integration

For task-based workflows, `ScudIterativeLoop` extends the pattern:

```rust
ScudLoopConfig {
    tag: "feature-x",
    max_iterations_per_task: 3,
    max_total_iterations: 100,
    use_sub_agents: true,
    verification_command: "make check test",
    auto_commit_waves: true,
}
```

Wave-based execution:
1. Group tasks by dependency level into waves
2. Execute wave tasks in parallel (via sub-agents)
3. Run verification after each task
4. Commit after wave completion
5. Move to next wave

#### Key Files

- `descartes/core/src/iterative_loop.rs` - Core loop executor (1313 lines)
- `descartes/core/src/scud_loop.rs` - SCUD variant
- `descartes/cli/src/commands/loop_cmd.rs` - CLI commands
- `descartes/gui/src/loop_view.rs` - GUI visualization
- `descartes/docs/blog/12-iterative-loops.md` - Documentation

### Comparison: Flow vs Gastown Molecules

| Aspect | Descartes Flow | Gastown Molecules |
|--------|---------------|-------------------|
| **Definition** | 6-phase PRD→Code automation | Formula TOML templates |
| **State tracking** | `.scud/flow-state.json` | Beads JSONL + git |
| **Handoff** | Structured markdown documents | Mail protocol messages |
| **Phases** | ingest→review→plan→implement→qa→summarize | No phases (step-based) |
| **Ephemeral** | No (all phases tracked) | Wisps for patrol loops |
| **Parallel execution** | Wave-based via SCUD | Single step at a time |

### Comparison: Ralph Wiggum vs Gastown Session Handoff

| Aspect | Descartes Ralph Wiggum | Gastown Handoff |
|--------|----------------------|-----------------|
| **Purpose** | Iterate until completion | Transfer context to fresh session |
| **Trigger** | Automatic (no completion promise) | Manual (`gt handoff`) |
| **State** | `.descartes/loop-state.json` | Handoff mail + hook bead |
| **Resume** | `descartes loop resume` | `gt mail check --inject` |
| **Completion** | `<promise>COMPLETE</promise>` | Work off hook completed |
| **Context** | Iteration count + file changes | Mail message + hooked molecule |

## Open Questions

1. **Wisp equivalent in Descartes?** - Gastown's ephemeral workflows have no direct equivalent
2. **Hook mechanism in Descartes?** - Flow handoffs serve similar purpose but less automatic
3. **Cross-machine coordination?** - ZMQ enables this, but is it implemented?
4. **Git integration in Descartes?** - Flow has git checkpoints; less integrated than Gastown

## Related Research

- `descartes/docs/blog/07-flow-workflow.md` - Flow documentation
- `descartes/docs/blog/12-iterative-loops.md` - Ralph Wiggum documentation
- `thoughts/shared/research/2025-12-28-iterative-agent-loop-ralph-style.md` - Original research
- `thoughts/shared/research/2025-12-29-integrated-flow-prompt-patterns.md` - Flow patterns
