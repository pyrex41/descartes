[Home](index.md) | [Getting Started](getting-started.md) | [Configuration](configuration.md) | [Swarm](swarm.md) | [Harnesses](harnesses.md) | [Workflows](workflows.md)

---

# Swarm Orchestration

Swarm is Descartes' fresh-context-per-task orchestration pattern, inspired by the Ralph Wiggum loop principles.

## Background: The Ralph Wiggum Pattern

The pattern is named after the Simpsons character who famously lives in the moment. The core principle: give each task a completely fresh context to prevent drift, error accumulation, and hallucination creep.

## The Problem with Traditional Agent Loops

Traditional AI agent loops accumulate context over time:

```
Task 1 → Context grows
Task 2 → Context grows more
Task 3 → Context grows even more
...
Task N → Context is huge, agent confused
```

This leads to:
- **Context drift**: The agent loses focus on the current goal
- **Error accumulation**: Mistakes compound as context grows
- **Token exhaustion**: Eventually hits context limits
- **Hallucination creep**: Agent starts referencing things that don't exist

## The Swarm Solution

Each task gets a completely fresh context:

```
Task 1 → Fresh context → Complete → Forget
Task 2 → Fresh context → Complete → Forget
Task 3 → Fresh context → Complete → Forget
```

Each task gets:
- A clean slate
- Only the information it needs
- No accumulated baggage

## How It Works

### 1. Task Graph (SCUD)

Tasks are organized in a DAG (Directed Acyclic Graph):

```
     [Task A]
        │
   ┌────┴────┐
   ▼         ▼
[Task B]  [Task C]
   │         │
   └────┬────┘
        ▼
     [Task D]
```

SCUD determines which tasks are "ready" based on completed dependencies.

### 2. Wave Computation

Tasks are grouped into waves:

- **Wave 1**: A (no dependencies)
- **Wave 2**: B, C (depend only on Wave 1)
- **Wave 3**: D (depends on Wave 2)

Tasks within a wave can run in parallel.

### 3. Fresh Context Per Task

For each task, Descartes builds a focused context:

```
┌─────────────────────────────────────┐
│ Task Spec (~5k tokens)              │
│ ├── Task ID and title               │
│ ├── Task description                │
│ ├── Dependency status               │
│ ├── Relevant plan section           │
│ └── Additional spec files           │
└─────────────────────────────────────┘
```

This is all the agent sees - no history from previous tasks.

### 4. Agent Execution

The agent (builder, fast-builder, etc.) executes with:
- Fresh harness session
- Task-specific prompt
- Full tool access
- Complete independence

### 5. Backpressure Validation

After each wave completes, validation runs:

```bash
# Default: whatever's in the validator category
cargo test && cargo clippy

# Or custom via --verify
npm test && npm run lint
```

If validation fails, tasks in the wave are marked for retry.

## Execution Flow

```
┌─────────────────────────────────────────────────────────┐
│ 1. Load SCUD tag and compute waves                      │
│    scud waves --tag my-feature                          │
└──────────────────────┬──────────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────────┐
│ 2. For each wave:                                       │
│    ┌─────────────────────────────────────────────────┐  │
│    │ For each task (in rounds of --round-size):     │  │
│    │   • Build fresh spec (task + plan + files)     │  │
│    │   • Select harness based on category           │  │
│    │   • Spawn agent with fresh session             │  │
│    │   • Execute and capture transcript             │  │
│    │   • Update SCUD status (done/failed/blocked)   │  │
│    └─────────────────────────────────────────────────┘  │
│                                                         │
│    • Run backpressure validation                        │
│    • If failed: mark recent tasks for retry             │
└──────────────────────┬──────────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────────┐
│ 3. Repeat until:                                        │
│    • All tasks complete, OR                             │
│    • No progress made (all remaining blocked/failed)    │
└─────────────────────────────────────────────────────────┘
```

## CLI Usage

### Basic Usage

Execute tasks for a SCUD tag:

```bash
descartes swarm --scud-tag my-feature
```

### With Plan Document

Provide an implementation plan for better context:

```bash
descartes swarm --scud-tag feature --plan ./docs/plan.md
```

### With Additional Spec Files

Add more context files:

```bash
descartes swarm --scud-tag feature \
    --plan ./docs/IMPLEMENTATION.md \
    --spec-file ./docs/ARCHITECTURE.md \
    --spec-file ./docs/CONVENTIONS.md
```

### With Validation

Enable backpressure validation:

```bash
descartes swarm --scud-tag feature \
    --verify "cargo test && cargo clippy -- -D warnings"
```

### Dry Run

Preview execution without running:

```bash
descartes swarm --scud-tag feature --dry-run
```

## Best Practices

### 1. Keep Tasks Small

The fresh context pattern works best with focused tasks:

```
# Good: Small, focused tasks
- "Add User model with email and password fields"
- "Create users table migration"
- "Add password hashing to User model"

# Bad: Large, vague tasks
- "Implement user authentication system"
```

Use `scud expand` to break down large tasks.

### 2. Provide Good Specs

The spec is all the agent sees. Make it count:

```bash
descartes swarm --scud-tag feature \
    --plan ./docs/IMPLEMENTATION.md \
    --spec-file ./docs/ARCHITECTURE.md \
    --spec-file ./docs/CONVENTIONS.md
```

### 3. Use Backpressure

Enable validation to catch issues early:

```bash
descartes swarm --scud-tag feature \
    --verify "cargo test && cargo clippy -- -D warnings"
```

### 4. Review Transcripts

All agent work is captured in transcripts:

```bash
# List today's transcripts
descartes transcripts --today

# View a specific transcript
descartes show <session-id>
```

## Comparison with Other Patterns

| Pattern | Context | Best For |
|---------|---------|----------|
| **Swarm** | Fresh per task | Many focused tasks |
| **Continuous Loop** | Accumulating | Single complex task |
| **Human-in-loop** | Manual checkpoints | High-stakes work |

Swarm excels when you have:
- Well-defined task graph
- Many independent tasks
- Need for reproducibility
- Quality validation gates

## Learn More

- Original Ralph Wiggum technique: https://ghuntley.com/ralph/

---

← [Configuration](configuration.md) | **Next:** [Harnesses](harnesses.md) →
