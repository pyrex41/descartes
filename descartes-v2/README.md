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

### Run the Ralph Loop

```bash
# Build mode (default) - implement tasks
descartes run --mode build

# Plan mode - analyze gaps, update task graph
descartes run --mode plan

# Single iteration
descartes run --max-iterations 1
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

## Ralph Loop Flow

### Build Mode

1. **Get Next Task**: Query SCUD for ready task
2. **BAML Decision**: Ask `DecideNextAction` what to do
3. **Parallel Search**: Spawn searcher subagents for context
4. **Build**: Single builder subagent implements the task
5. **Validate**: Validator runs tests (backpressure gate)
6. **Commit**: If tests pass, commit with BAML-generated message

### Plan Mode

1. **Get State**: List completed and remaining tasks
2. **BAML Plan**: Call `CreatePlan` with context
3. **Update Graph**: Record plan in transcript

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
