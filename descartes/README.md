# Descartes v2

A focused Rust implementation of the Swarm pattern (inspired by Ralph Wiggum) for AI agent orchestration.

**[Documentation](https://pyrex41.github.io/descartes/)** | [Getting Started](https://pyrex41.github.io/descartes/getting-started.html) | [Configuration](https://pyrex41.github.io/descartes/configuration.html)

## Architecture

```
┌─────────────────────────────────────────┐
│           Swarm Loop (outer)            │
│  descartes swarm --scud-tag feature     │
└────────────────────┬────────────────────┘
                     ▼
┌─────────────────────────────────────────┐
│           SCUD Task Graph               │
│  $ scud next → returns ready task       │
└────────────────────┬────────────────────┘
                     ▼
┌─────────────────────────────────────────┐
│    Subagents (1 level, visible)         │
│  searcher → builder → validator         │
│  All transcripts saved in SCG format    │
└─────────────────────────────────────────┘
```

### Key Concepts

- **Swarm**: Fresh context each task (inspired by Ralph Wiggum), prevents drift and error accumulation
- **SCUD**: DAG-driven task management with token-efficient SCG format
- **Visible Subagents**: Full transcript capture for every subagent - no black boxes
- **User Guidance**: Inject custom context into agent prompts via config

## Project Structure

```
descartes-v2/
├── src/
│   ├── lib.rs              # Module exports
│   ├── main.rs             # CLI entry point
│   ├── swarm_executor.rs   # Main orchestration loop
│   ├── swarm_tui.rs        # Terminal UI
│   ├── agent/              # Subagent spawning
│   ├── config.rs           # Configuration
│   ├── handoff/            # Stage handoffs
│   ├── harness/            # LLM harnesses (Claude Code, Codex, etc.)
│   ├── interactive/        # REPL mode with slash commands
│   ├── scud/               # Task graph management
│   ├── spec.rs             # Spec/prompt building
│   └── transcript/         # SCG format transcripts
└── Cargo.toml
```

## Building

```bash
# Build
cargo build --release

# Run tests
cargo test

# Check compilation
cargo check
```

### Dependencies

- Rust 1.75+

## Configuration

Create `.descartes/config.toml` in your project root:

```toml
[harness]
kind = "claude-code"

[harness.claude_code]
model = "opus"

[scud]
task_file = ".scud/scud.scg"

[swarm]
use_fast_first = true
always_review = false
heuristic = "prefer_speed"

# User guidance - inject custom context into agent prompts
[guidance]
global = "Always follow existing code patterns. Prefer small, focused changes."
builder = "Run tests after making changes. Use cargo check before cargo test."
review = "Check for security issues and edge cases."
validator = "Use cargo test --all-features for full coverage."
```

### User Guidance

The `[guidance]` section lets you inject custom context into agent prompts without modifying code:

- **global**: Included in all agent prompts
- **builder**: Specific to builder/fast-builder agents (implementation tasks)
- **review**: Specific to reviewer agents
- **validator**: Specific to validator agents (test running)

Global and context-specific guidance are combined when building prompts.

## Usage

### Swarm Command

The `swarm` command is the main entry point for executing SCUD tasks using the Swarm pattern. It provides fresh-context-per-task execution with wave-based parallelism and backpressure validation.

#### Basic Usage

```bash
# Execute tasks from an existing SCUD tag
descartes swarm --scud-tag my-feature

# Initialize from a PRD and execute
descartes swarm --prd ./docs/prd.md

# Preview execution plan without running agents
descartes swarm --scud-tag my-feature --dry-run
```

#### PRD Initialization

Initialize tasks directly from a Product Requirements Document:

```bash
# Basic PRD initialization (creates tag from filename)
descartes swarm --prd ./docs/feature-prd.md

# Custom tag name and task count
descartes swarm --prd ./docs/prd.md --tag my-feature --num-tasks 15

# Skip expansion or dependency checks
descartes swarm --prd ./docs/prd.md --no-expand --no-check-deps
```

When using `--prd`, Descartes automatically runs:
1. `scud parse <prd> --tag <tag>` - Generate tasks from PRD
2. `scud expand --tag <tag>` - Break complex tasks into subtasks (unless `--no-expand`)
3. `scud check-deps --fix --tag <tag>` - Validate dependencies (unless `--no-check-deps`)

#### Spec Configuration

Provide additional context for each task using the "fixed spec allocation" pattern (~5k tokens):

```bash
# Include an implementation plan document
descartes swarm --scud-tag my-feature --plan ./docs/IMPLEMENTATION.md

# Include multiple spec files
descartes swarm --scud-tag my-feature \
    --spec-file ./docs/ARCHITECTURE.md \
    --spec-file ./docs/API_CONTRACTS.md

# Adjust token budget for specs
descartes swarm --scud-tag my-feature --max-spec-tokens 8000
```

The spec is built from:
- **Task details** from SCUD (ID, title, description, dependencies)
- **Plan section** extracted from the plan document matching the task ID
- **Additional specs** from `--spec-file` arguments

#### Execution Options

```bash
# Custom verification command (overrides backpressure config)
descartes swarm --scud-tag my-feature --verify "npm test"

# Use a different harness
descartes swarm --scud-tag my-feature --harness opencode  # or: codex

# Override the model
descartes swarm --scud-tag my-feature --model opus

# Adjust tasks per round (for rate limiting)
descartes swarm --scud-tag my-feature --round-size 3

# Skip validation between waves
descartes swarm --scud-tag my-feature --no-validate

# Specify working directory
descartes swarm --scud-tag my-feature --working-dir /path/to/project
```

#### Complete Example

```bash
# Full workflow: PRD → Tasks → Execution
descartes swarm \
    --prd ./docs/auth-feature-prd.md \
    --tag auth-feature \
    --num-tasks 12 \
    --plan ./docs/auth-implementation-plan.md \
    --spec-file ./docs/security-guidelines.md \
    --verify "cargo test && cargo clippy" \
    --harness claude-code \
    --model sonnet \
    --round-size 5
```

### How It Works

The Swarm loop implements a fresh-context-per-task execution pattern:

```
┌─────────────────────────────────────────────────────────────┐
│  1. Load SCUD tag and compute execution waves (DAG order)  │
└────────────────────────────┬────────────────────────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────┐
│  2. For each wave:                                          │
│     ┌───────────────────────────────────────────────────┐   │
│     │  For each task (in rounds):                       │   │
│     │    • Build fresh spec (task + plan + custom)      │   │
│     │    • Spawn agent with fresh session               │   │
│     │    • Execute task implementation                  │   │
│     │    • Mark done/failed/blocked in SCUD             │   │
│     └───────────────────────────────────────────────────┘   │
│     • Run backpressure validation (if enabled)              │
│     • Mark failed tasks if validation fails                 │
└────────────────────────────┬────────────────────────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Repeat until all tasks complete or no progress          │
└─────────────────────────────────────────────────────────────┘
```

**Key principles:**
- **Fresh context each task**: No accumulated history, prevents drift
- **Wave-based execution**: Tasks execute in dependency order
- **Backpressure validation**: Build/test/lint between waves
- **Failed task tracking**: Validation failures mark tasks for retry

### Other Commands

```bash
# Get next ready task from SCUD
descartes next

# Show task waves
descartes waves

# Spawn a subagent manually
descartes spawn <category> "<prompt>"
```

### Interactive Mode

```bash
descartes interactive

# Available commands:
# /plan   - Switch to planning mode
# /build  - Switch to building mode
# /status - Show current state
# /quit   - Exit
```

## Environment Variables

```bash
ANTHROPIC_API_KEY=sk-ant-...   # For Claude models
OPENAI_API_KEY=sk-...          # For OpenAI models
```

## Development

### Modifying the Swarm Loop

The main loop is in `src/swarm_executor.rs`. Key functions:

- `new()` - Create executor with configuration
- `run()` - Execute tasks for a SCUD tag
- `dry_run()` - Preview execution plan
- `compute_waves()` - Calculate parallel execution waves

## License

MIT
