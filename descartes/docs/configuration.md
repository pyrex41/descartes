# Configuration Guide

Descartes is highly configurable through environment variables, configuration files, and CLI arguments.

## Configuration Hierarchy

1. **CLI arguments** (highest priority)
2. **Environment variables**
3. **`.descartes/config.toml`**
4. **Built-in defaults** (lowest priority)

## Environment Variables

Create a `.env` file in your project root (or use `.descartes/.env`):

```bash
# Default harness: opencode, claude-code, codex
DESCARTES_HARNESS=opencode

# Fast model (for parallel search, analysis, validation)
DESCARTES_FAST_MODEL=xai/grok-code-fast-1

# Smart model (for complex implementation, planning)
DESCARTES_SMART_MODEL=opus

# Harness-specific models
DESCARTES_OPENCODE_MODEL=xai/grok-code-fast-1
DESCARTES_CLAUDE_MODEL=opus
DESCARTES_CODEX_MODEL=gpt-5.1

# SCUD pass-through (optional, overrides SCUD's own config)
SCUD_PROVIDER=xai
SCUD_MODEL=grok-code-fast-1
SCUD_SMART_PROVIDER=claude-cli
SCUD_SMART_MODEL=opus

# API Keys
XAI_API_KEY=xai-...
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
```

## Configuration File

The `.descartes/config.toml` file provides full control:

```toml
prompts_dir = "prompts"

[harness]
kind = "opencode"  # Default harness

[harness.claude_code]
model = "opus"
headless = true
dangerously_skip_permissions = false

[harness.opencode]
model = "xai/grok-code-fast-1"

[harness.codex]
model = "gpt-4o"

# Agent categories define behavior for different task types
[categories.builder]
description = "Code implementation"
model = "opus"
harness = "claude-code"
tools = ["read", "write", "edit", "bash"]
parallel = false
backpressure = false

[categories.searcher]
description = "Fast parallel code search"
model = "xai/grok-code-fast-1"
# No harness specified = uses default (opencode)
tools = ["read", "bash"]
parallel = true
backpressure = false

[categories.validator]
description = "Test runner (backpressure gate)"
model = "xai/grok-code-fast-1"
tools = ["bash"]
parallel = false
backpressure = true  # Creates validation checkpoint

[ralph_loop]
use_fast_first = true    # Try fast-builder before smart builder
always_review = false    # Review fast-builder changes with smart model
heuristic = "prefer_speed"

[scud]
task_file = ".scud/scud.scg"
embedded = false

[transcripts]
directory = ".descartes/transcripts"
format = "scg"
max_keep = 0  # 0 = keep all
```

## Agent Categories

Categories define how different types of tasks are handled:

| Category | Model | Harness | Purpose |
|----------|-------|---------|---------|
| `searcher` | grok-code-fast-1 | opencode | Fast parallel code search |
| `analyzer` | grok-code-fast-1 | opencode | Deep code analysis |
| `validator` | grok-code-fast-1 | opencode | Test/lint/build gates |
| `fast-builder` | grok-code-fast-1 | opencode | Quick implementations |
| `builder` | opus | claude-code | Complex implementations |
| `planner` | opus | claude-code | Task planning |
| `orchestrator` | opus | claude-code | Loop orchestration |
| `builder-reviewer` | opus | claude-code | Code review and fixes |

### Custom Categories

Add your own categories:

```toml
[categories.security-reviewer]
description = "Security-focused code review"
model = "opus"
harness = "claude-code"
tools = ["read"]
parallel = false
backpressure = false
prompt_template = "prompts/security-review.md"
```

## CLI Arguments

Override any setting via CLI:

```bash
# Use a specific harness
descartes ralph --scud-tag feature --harness claude-code

# Override model
descartes ralph --scud-tag feature --model opus

# Custom validation command
descartes ralph --scud-tag feature --verify "npm test && npm run lint"

# Adjust parallelism
descartes ralph --scud-tag feature --round-size 5

# Dry run (preview without execution)
descartes ralph --scud-tag feature --dry-run
```

## Mixed Harness Strategy

The default configuration uses a mixed strategy:

- **Fast tasks** (search, analyze, validate, simple builds) use **OpenCode + grok-code-fast-1**
  - Lower latency, cheaper, good for parallelization

- **Smart tasks** (complex implementation, planning, review) use **Claude Code + Opus**
  - Higher quality output, better reasoning, handles complexity

This is controlled per-category via the `harness` field. Categories without an explicit harness use the global default (`harness.kind`).

## SCUD Integration

Descartes passes configuration to SCUD when spawning tasks:

```toml
[scud]
task_file = ".scud/scud.scg"
provider = "xai"           # Override SCUD's default provider
model = "grok-code-fast-1" # Override SCUD's default model
smart_provider = "claude-cli"
smart_model = "opus"
```

These values are passed as environment variables when Descartes invokes SCUD commands.

## Transcript Settings

Control how agent transcripts are stored:

```toml
[transcripts]
directory = ".descartes/transcripts"
format = "scg"    # Token-efficient format
max_keep = 100    # Keep last 100 transcripts (0 = unlimited)
```

Transcripts are stored per-agent with timestamps:
```
.descartes/transcripts/
├── 2024-01-14T12-30-00_builder_TASK-001.scg
├── 2024-01-14T12-31-00_validator_TASK-001.scg
└── ...
```
