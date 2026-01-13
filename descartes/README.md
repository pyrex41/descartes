# Descartes v2

A focused Rust implementation of the Ralph Wiggum loop pattern for AI agent orchestration.

## Architecture

```
┌─────────────────────────────────────────┐
│           Ralph Loop (outer)            │
│  while :; do descartes run ; done       │
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

- **Ralph Wiggum Loop**: Fresh context each iteration (prevents drift), two modes (Plan/Build)
- **SCUD**: DAG-driven task management with token-efficient SCG format
- **Visible Subagents**: Full transcript capture for every subagent - no black boxes
- **BAML**: Type-safe LLM prompts with compile-time Rust codegen

## BAML Integration

Descartes uses [BAML](https://boundaryml.com) for structured LLM interactions with **native Rust codegen**.

### How It Works

1. Define prompts in `.baml` files with typed inputs/outputs
2. Run `npx @boundaryml/baml generate` to generate Rust code
3. Call functions directly: `B.DecideNextAction.call(args).await`

### No Server Required

BAML compiles directly to Rust - no HTTP server, no REST API, no runtime overhead.

```rust
// Generated usage pattern
use crate::baml_client::async_client::B;
use crate::baml_client::types::NextAction;

// Type-safe function call
let decision = B.DecideNextAction.call(
    &completed_tasks,
    None::<&str>,
    &remaining_tasks,
    &blockers,
    "recent output",
    None::<&str>,
).await?;

match decision.action {
    NextAction::Continue => { /* keep going */ }
    NextAction::Complete => { /* done */ }
    NextAction::Replan => { /* switch modes */ }
    // ...
}
```

### BAML Functions

| Function | Purpose | Used In |
|----------|---------|---------|
| `DecideNextAction` | Loop flow control | `ralph_loop.rs` |
| `SelectSubagent` | Route tasks to agents | `ralph_loop.rs` |
| `CreatePlan` | Generate implementation plans | `ralph_loop.rs` |
| `GenerateCommitMessage` | Conventional commit messages | `ralph_loop.rs` |

### Build-Time Code Generation

The `baml_client/` directory is generated automatically at build time via `build.rs`.
No manual regeneration needed - just run `cargo build`.

The build script:
1. Checks if `.baml` files are newer than generated code
2. Runs `npx @boundaryml/baml generate` if needed
3. Falls back to `baml` CLI if npx unavailable

**Requirements**: Node.js with npx, or `npm install -g @boundaryml/baml`

## Project Structure

```
descartes-v2/
├── src/
│   ├── lib.rs              # Module exports
│   ├── main.rs             # CLI entry point
│   ├── ralph_loop.rs       # Main orchestration loop
│   ├── agent/              # Subagent spawning
│   ├── baml_client -> ../baml_client/baml_client  # Generated BAML code
│   ├── config.rs           # Configuration
│   ├── handoff/            # Stage handoffs
│   ├── harness/            # LLM harnesses (Claude Code, Codex, etc.)
│   ├── interactive/        # REPL mode with slash commands
│   ├── scud/               # Task graph management
│   ├── transcript/         # SCG format transcripts
│   └── workflow/           # Multi-stage workflows
├── baml_src/               # BAML prompt definitions
│   ├── generator.baml      # Codegen config
│   ├── clients.baml        # LLM client definitions
│   ├── orchestrator.baml   # DecideNextAction, SelectSubagent
│   ├── planning.baml       # CreatePlan, BreakdownTask
│   ├── handoff.baml        # GenerateCommitMessage, etc.
│   └── ...
├── baml_client/            # Generated Rust code
│   └── baml_client/
│       ├── mod.rs
│       ├── async_client.rs # B.FunctionName.call() pattern
│       ├── types/          # Generated types
│       └── ...
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
- `protoc` (protobuf compiler) - required by `baml` crate
- Node.js/npm - for BAML CLI (`npx @boundaryml/baml`)

Install protoc:
```bash
# Ubuntu/Debian
apt-get install protobuf-compiler

# macOS
brew install protobuf

# Or download from https://github.com/protocolbuffers/protobuf/releases
```

## Configuration

Create `descartes.toml` in your project root:

```toml
[harness.claude_code]
model = "opus"
working_dir = "."

[scud]
file = ".scud/scud.scg"
```

## Usage

### Ralph Command

The `ralph` command is the main entry point for executing SCUD tasks using the Ralph Wiggum loop pattern. It provides fresh-context-per-task execution with wave-based parallelism and backpressure validation.

#### Basic Usage

```bash
# Execute tasks from an existing SCUD tag
descartes ralph --scud-tag my-feature

# Initialize from a PRD and execute
descartes ralph --prd ./docs/prd.md

# Preview execution plan without running agents
descartes ralph --scud-tag my-feature --dry-run
```

#### PRD Initialization

Initialize tasks directly from a Product Requirements Document:

```bash
# Basic PRD initialization (creates tag from filename)
descartes ralph --prd ./docs/feature-prd.md

# Custom tag name and task count
descartes ralph --prd ./docs/prd.md --tag my-feature --num-tasks 15

# Skip expansion or dependency checks
descartes ralph --prd ./docs/prd.md --no-expand --no-check-deps
```

When using `--prd`, Descartes automatically runs:
1. `scud parse <prd> --tag <tag>` - Generate tasks from PRD
2. `scud expand --tag <tag>` - Break complex tasks into subtasks (unless `--no-expand`)
3. `scud check-deps --fix --tag <tag>` - Validate dependencies (unless `--no-check-deps`)

#### Spec Configuration

Provide additional context for each task using the "fixed spec allocation" pattern (~5k tokens):

```bash
# Include an implementation plan document
descartes ralph --scud-tag my-feature --plan ./docs/IMPLEMENTATION.md

# Include multiple spec files
descartes ralph --scud-tag my-feature \
    --spec-file ./docs/ARCHITECTURE.md \
    --spec-file ./docs/API_CONTRACTS.md

# Adjust token budget for specs
descartes ralph --scud-tag my-feature --max-spec-tokens 8000
```

The spec is built from:
- **Task details** from SCUD (ID, title, description, dependencies)
- **Plan section** extracted from the plan document matching the task ID
- **Additional specs** from `--spec-file` arguments

#### Execution Options

```bash
# Custom verification command (overrides backpressure config)
descartes ralph --scud-tag my-feature --verify "npm test"

# Use a different harness
descartes ralph --scud-tag my-feature --harness opencode  # or: codex

# Override the model
descartes ralph --scud-tag my-feature --model opus

# Adjust tasks per round (for rate limiting)
descartes ralph --scud-tag my-feature --round-size 3

# Skip validation between waves
descartes ralph --scud-tag my-feature --no-validate

# Specify working directory
descartes ralph --scud-tag my-feature --working-dir /path/to/project
```

#### Complete Example

```bash
# Full workflow: PRD → Tasks → Execution
descartes ralph \
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

The Ralph Wiggum loop implements a fresh-context-per-task execution pattern:

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
# Run a single build iteration
descartes run

# Run a single planning iteration
descartes plan

# Run the continuous loop (legacy)
descartes loop [--plan] [--max N]

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

### Adding a New BAML Function

1. Define the function in `baml_src/*.baml`:
   ```baml
   function MyNewFunction(input: string) -> MyOutput {
     client ClaudeClient
     prompt #"..."#
   }
   ```

2. Regenerate code:
   ```bash
   npx @boundaryml/baml generate --from baml_src
   ```

3. Use in Rust:
   ```rust
   use crate::baml_client::async_client::B;

   let result = B.MyNewFunction.call("input").await?;
   ```

### Modifying the Ralph Loop

The main loop is in `src/ralph_loop.rs`. Key functions:

- `run()` - Entry point
- `build_iteration()` - Single build cycle
- `plan_iteration()` - Single plan cycle
- `git_commit_baml()` - Commit with generated message

## License

MIT
