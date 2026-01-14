# Descartes Documentation

Welcome to the Descartes documentation. Descartes is an AI agent orchestration tool implementing the Ralph Wiggum loop pattern for fresh-context-per-task execution.

## Quick Links

- [Getting Started](./getting-started.md) - Installation and first steps
- [Configuration](./configuration.md) - Environment variables, config files, categories
- [Ralph Loop](./ralph-loop.md) - Understanding the orchestration pattern
- [Harnesses](./harnesses.md) - OpenCode, Claude Code, and Codex
- [Workflows](./workflows.md) - Common usage patterns

## What is Descartes?

Descartes orchestrates AI agents to complete software development tasks. It works with [SCUD](https://github.com/pyrex41/scud) for task management and supports multiple AI backends (harnesses).

### Key Features

- **Fresh context per task**: Each task starts clean, preventing drift
- **Mixed harness strategy**: Fast models for simple tasks, smart models for complex ones
- **Wave-based execution**: Tasks run in dependency order with parallelization
- **Backpressure validation**: Run tests/lints between waves
- **Full transcript capture**: Every agent interaction is logged

### The Ralph Wiggum Pattern

Unlike traditional loops that accumulate context:

```
Traditional: Task 1 → Task 2 → Task 3 → ... → Context overflow
Ralph Loop:  Task 1 → Fresh → Task 2 → Fresh → Task 3 → ...
```

Each task gets only the context it needs, nothing more.

## Getting Help

```bash
# CLI help
descartes --help
descartes ralph --help

# Check configuration
descartes config

# View active harness
descartes harness
```

## Default Configuration

Out of the box, Descartes uses:

| Setting | Value | Purpose |
|---------|-------|---------|
| Default harness | `opencode` | Fast execution |
| Fast model | `xai/grok-code-fast-1` | Search, analysis, validation |
| Smart model | `opus` | Complex implementation |
| Smart harness | `claude-code` | Quality-critical tasks |

All defaults are configurable via environment variables or config files.

## Example Session

```bash
# Initialize project
descartes init

# Create tasks from PRD
descartes ralph --prd ./docs/feature.md --tag my-feature

# Check progress
scud stats --tag my-feature

# View transcripts
descartes transcripts
```

## Architecture Overview

```
┌─────────────────────────────────────────┐
│         Descartes CLI                   │
│  descartes ralph --scud-tag feature     │
└────────────────────┬────────────────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│  SCUD Tasks     │    │  Configuration  │
│  (DAG graph)    │    │  (.toml + env)  │
└────────┬────────┘    └────────┬────────┘
         │                      │
         └──────────┬───────────┘
                    ▼
┌─────────────────────────────────────────┐
│           Ralph Loop                    │
│  ┌─────────────────────────────────┐    │
│  │ Wave 1: Tasks A, B, C           │    │
│  │   → Spawn agents (fresh each)   │    │
│  │   → Capture transcripts         │    │
│  │   → Update SCUD status          │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ Validation: cargo test          │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ Wave 2: Tasks D, E (depend on   │    │
│  │         Wave 1)                 │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
                    │
         ┌──────────┴──────────┐
         ▼                     ▼
┌─────────────────┐   ┌─────────────────┐
│ OpenCode        │   │ Claude Code     │
│ (fast tasks)    │   │ (smart tasks)   │
└─────────────────┘   └─────────────────┘
```

## Contributing

Descartes is open source. Contributions welcome at:
https://github.com/pyrex41/descartes
