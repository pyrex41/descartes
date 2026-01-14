# Getting Started with Descartes

Descartes is an AI agent orchestration tool that implements the "Ralph Wiggum loop" pattern - a fresh-context-per-task approach that prevents drift and keeps agents focused.

## Installation

### Prerequisites

- Rust 1.75+
- `protoc` (protobuf compiler)
- Node.js/npm (for BAML CLI)

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
apt-get install protobuf-compiler
```

### Build from Source

```bash
git clone https://github.com/pyrex41/descartes
cd descartes/descartes
cargo build --release

# Add to PATH
cp target/release/descartes ~/.local/bin/
```

## Quick Start

### 1. Initialize a Project

```bash
cd your-project
descartes init
```

This creates:
- `.descartes/config.toml` - Configuration file
- `.descartes/transcripts/` - Agent transcript storage
- `prompts/` - Customizable prompt templates

### 2. Set Up Environment

Copy the example environment file and configure your API keys:

```bash
cp .env.example .env

# Edit .env with your keys
# XAI_API_KEY=xai-...        # For grok models (fast)
# ANTHROPIC_API_KEY=sk-ant-... # For Claude models (smart)
```

### 3. Create Tasks with SCUD

Descartes uses SCUD for task management. Initialize SCUD in your project:

```bash
scud init
```

Parse a PRD (Product Requirements Document) into tasks:

```bash
scud parse ./docs/prd.md --tag my-feature
```

### 4. Run the Ralph Loop

Execute tasks using the Ralph Wiggum loop:

```bash
# Execute all ready tasks in a tag
descartes ralph --scud-tag my-feature

# Or initialize from PRD and execute in one command
descartes ralph --prd ./docs/prd.md --tag my-feature
```

## Understanding the Output

When Descartes runs, you'll see:

```
[Wave 1] Executing 3 tasks...
  [TASK-001] Implement user model
    → Spawning builder agent (claude-code/opus)
    → Task completed successfully
  [TASK-002] Create database schema
    → Spawning builder agent (claude-code/opus)
    → Task completed successfully
  [TASK-003] Add migration scripts
    → Spawning fast-builder agent (opencode/grok-code-fast-1)
    → Task completed successfully

[Validation] Running: cargo test
  → All tests passing

[Wave 2] Executing 2 tasks...
...
```

## Key Concepts

### Fresh Context Per Task

Unlike traditional agent loops that accumulate context, Descartes starts each task with a fresh context. This prevents:
- Context drift (agent "forgetting" the goal)
- Accumulated errors compounding
- Token budget exhaustion

### Wave-Based Execution

Tasks are organized into waves based on their dependencies (DAG order):
- Wave 1: Tasks with no dependencies
- Wave 2: Tasks that depend on Wave 1
- And so on...

Tasks within a wave can execute in parallel (rate-limited).

### Backpressure Validation

After each wave, Descartes can run validation commands (tests, lints, builds). If validation fails, affected tasks are marked for retry.

## Next Steps

- [Configuration Guide](./configuration.md) - Customize harnesses, models, and categories
- [Ralph Loop Deep Dive](./ralph-loop.md) - Understand the orchestration pattern
- [Harnesses Guide](./harnesses.md) - Choose the right harness for your tasks
- [Workflows](./workflows.md) - Common usage patterns
