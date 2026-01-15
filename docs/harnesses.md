# Harnesses Guide

A harness is the execution environment for an AI agent. Descartes supports multiple harnesses, each with different strengths.

## Available Harnesses

### OpenCode

[OpenCode](https://github.com/opencode-ai/opencode) is a fast, lightweight CLI for AI coding tasks.

**Best for:**
- Fast parallel operations
- Simple code generation
- Search and analysis tasks
- Cost-sensitive workflows

**Default model:** `xai/grok-code-fast-1`

```bash
# Use OpenCode explicitly
descartes swarm --scud-tag feature --harness opencode

# With custom model
descartes swarm --scud-tag feature --harness opencode --model xai/grok-3-fast
```

**Configuration:**

```toml
[harness.opencode]
model = "xai/grok-code-fast-1"
binary = "/path/to/opencode"  # Optional, defaults to PATH
```

**Supported models:**
- `xai/grok-code-fast-1` (recommended for speed)
- `xai/grok-3-fast`
- `anthropic/claude-sonnet`
- `openai/gpt-4o`
- Any model supported by OpenCode

### Claude Code (CLI Default)

[Claude Code](https://claude.ai/code) is Anthropic's official CLI for Claude. This is the default harness when using the CLI.

**Best for:**
- Complex reasoning tasks
- Multi-step implementations
- Code review and refactoring
- Tasks requiring deep understanding

**Default model:** `opus`

```bash
# Use Claude Code explicitly
descartes swarm --scud-tag feature --harness claude-code

# With specific model
descartes swarm --scud-tag feature --harness claude-code --model sonnet
```

**Configuration:**

```toml
[harness.claude_code]
model = "opus"
binary = "/path/to/claude"  # Optional, defaults to PATH
headless = true             # Run without TUI
dangerously_skip_permissions = false  # Require tool approval
```

**Supported models:**
- `opus` - Claude Opus (most capable)
- `sonnet` - Claude Sonnet (balanced)
- `haiku` - Claude Haiku (fastest)

### Codex

OpenAI's Codex-style API harness.

**Best for:**
- OpenAI model users
- Specific model requirements
- API-based workflows

**Default model:** `gpt-4o`

```bash
descartes swarm --scud-tag feature --harness codex --model gpt-4o
```

**Configuration:**

```toml
[harness.codex]
model = "gpt-4o"
api_base = "https://api.openai.com/v1"  # Optional
api_key = "..."  # Optional, prefer env var
```

## Mixed Harness Strategy

Descartes defaults to a mixed strategy that uses the right harness for each task type:

```
┌─────────────────────────────────────────────────────────┐
│                    Task Categories                       │
├─────────────────────────────────────────────────────────┤
│  FAST (OpenCode + grok-code-fast-1)                     │
│  ├── searcher     - Parallel code search                │
│  ├── analyzer     - Code analysis                       │
│  ├── validator    - Test/lint gates                     │
│  └── fast-builder - Quick implementations               │
├─────────────────────────────────────────────────────────┤
│  SMART (Claude Code + Opus)                             │
│  ├── builder          - Complex implementations         │
│  ├── planner          - Task planning                   │
│  ├── orchestrator     - Loop control                    │
│  └── builder-reviewer - Code review                     │
└─────────────────────────────────────────────────────────┘
```

### Why Mixed?

1. **Cost efficiency**: Fast models are cheaper for simple tasks
2. **Speed**: grok-code-fast-1 has lower latency than Opus
3. **Quality when needed**: Opus handles complex tasks better
4. **Parallelization**: Fast models handle parallel execution better

### Configuring Categories

Each category can specify its harness:

```toml
# Fast category - uses default harness (opencode)
[categories.searcher]
model = "xai/grok-code-fast-1"
# harness not specified = uses default

# Smart category - explicitly uses claude-code
[categories.builder]
model = "opus"
harness = "claude-code"
```

## Choosing a Harness

### Use OpenCode When:
- Tasks are simple and well-defined
- You need fast turnaround
- Running many tasks in parallel
- Cost is a concern

### Use Claude Code When:
- Tasks require complex reasoning
- Quality is critical
- Tasks involve multi-file refactoring
- You need the best output quality

### Use Codex When:
- You prefer OpenAI models
- You have specific API requirements
- You're already in an OpenAI ecosystem

## Environment Variables

```bash
# Default harness
DESCARTES_HARNESS=opencode

# Model per harness
DESCARTES_OPENCODE_MODEL=xai/grok-code-fast-1
DESCARTES_CLAUDE_MODEL=opus
DESCARTES_CODEX_MODEL=gpt-4o

# Shorthand for fast/smart tiers
DESCARTES_FAST_MODEL=xai/grok-code-fast-1
DESCARTES_SMART_MODEL=opus
```

## Harness-Specific Features

### OpenCode
- Streaming output
- Built-in tool support
- Multi-provider support

### Claude Code
- MCP (Model Context Protocol) support
- Rich tool ecosystem
- Session persistence (optional)

### Codex
- Pure API-based
- Maximum flexibility
- Works with any OpenAI-compatible endpoint

## Troubleshooting

### "Harness not found"

Ensure the harness binary is in your PATH:

```bash
# Check OpenCode
which opencode

# Check Claude Code
which claude

# Or specify full path in config
[harness.opencode]
binary = "/usr/local/bin/opencode"
```

### "Model not supported"

Check that the model is valid for the provider:

```bash
# OpenCode models require provider prefix
model = "xai/grok-code-fast-1"  # Correct
model = "grok-code-fast-1"       # Wrong for OpenCode

# Claude Code uses simple names
model = "opus"    # Correct
model = "claude-opus-4-5-20251101"  # Wrong for Claude Code
```

### Slow Performance

If tasks are slow:

1. Switch to a faster model for simple tasks
2. Use OpenCode for parallelizable work
3. Check network connectivity to API endpoints
4. Consider using `--round-size` to batch requests
