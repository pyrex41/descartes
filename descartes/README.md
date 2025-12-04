<p align="center">
  <img src="docs/assets/logo.svg" alt="Descartes" width="120">
</p>

<h1 align="center">Descartes</h1>

<p align="center">
  <strong>The anti-bloat AI coding agent.</strong><br>
  4 tools. Full observability. Zero MCP baggage.
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> •
  <a href="#philosophy">Philosophy</a> •
  <a href="#tools">Tools</a> •
  <a href="#skills">Skills</a> •
  <a href="docs/">Documentation</a>
</p>

<p align="center">
  <a href="https://github.com/anthropics/descartes/actions"><img src="https://img.shields.io/github/actions/workflow/status/anthropics/descartes/ci.yml?style=flat-square" alt="Build"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License"></a>
  <a href="https://crates.io/crates/descartes"><img src="https://img.shields.io/crates/v/descartes?style=flat-square" alt="Crates.io"></a>
</p>

---

> *"If you don't need it, don't build it."* — [Pi Philosophy](https://marioslab.io/posts/pi/building-a-coding-agent/)

## The Problem

Modern AI coding agents are **bloated**:

- 🔴 **40+ tools** nobody uses
- 🔴 **10,000+ token** system prompts
- 🔴 **MCP servers** injecting 2-5k tokens *every single message*
- 🔴 **Recursive agent spawning** chaos with no control
- 🔴 **Opaque execution** — what did the agent actually do?

## The Solution

Descartes strips AI agents down to their essence:

|                        | Bloated Agents | Descartes |
|------------------------|----------------|-----------|
| **Tools**              | 40+            | **4**     |
| **System prompt**      | 10k tokens     | **200 tokens** |
| **Per-message overhead** | 2-5k tokens  | **0**     |
| **Recursive spawning** | Uncontrolled   | **Prevented** |
| **Observability**      | Opaque         | **Full JSON transcripts** |

## Quickstart

```bash
# Install
cargo install descartes

# Run your first agent
descartes spawn --task "Fix the type error in main.rs"

# Check system status
descartes doctor

# View what happened
cat .scud/sessions/*.json | jq '.entries[-5:]'
```

That's it. Watch it work. See every action in the transcript.

## Philosophy

Descartes follows the **Pi philosophy** of minimal, observable tooling:

### 1. Four Tools Are Enough

```
read   → Read file contents
write  → Write file contents
edit   → Surgical text replacement
bash   → Execute any command
```

With `bash`, your agent can run *any* CLI tool. No need to wrap everything in a custom tool definition.

### 2. Skills, Not MCP Servers

MCP servers inject **2,000-5,000 tokens** into every message, whether used or not.

Descartes uses **skills** — CLI tools invoked via bash:

```bash
# Agent discovers the skill exists, then calls it
bash: web-search "rust async patterns"
```

| Approach | Context Cost | When Paid |
|----------|-------------|-----------|
| MCP Server | ~2,000-5,000 tokens | Every message |
| **Skill** | ~50-100 tokens | Only when used |

### 3. Full Observability

Every session produces a JSON transcript:

```json
{
  "metadata": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "provider": "anthropic",
    "model": "claude-sonnet-4-20250514",
    "tool_level": "orchestrator"
  },
  "entries": [
    {"role": "user", "content": "Fix the bug..."},
    {"role": "tool_call", "tool_name": "read", "args": {"path": "src/main.rs"}},
    {"role": "tool_result", "content": "...file contents..."},
    {"role": "assistant", "content": "I found the issue..."}
  ]
}
```

### 4. Controlled Delegation

Orchestrator agents can spawn sub-sessions, but sub-sessions **cannot spawn their own children**:

```
Orchestrator (can spawn)
    └── Sub-session (cannot spawn) ✓
         └── Sub-sub-session ✗ BLOCKED
```

No recursive agent explosions.

## Tools

### Tool Levels

| Level | Tools | Use Case |
|-------|-------|----------|
| **Orchestrator** | read, write, edit, bash, spawn_session | Top-level agents that delegate |
| **Minimal** | read, write, edit, bash | Focused tasks, sub-sessions |
| **ReadOnly** | read, bash | Exploration, planning, analysis |

### Core Tools

| Tool | Description |
|------|-------------|
| `read` | Read file with optional offset/limit for large files |
| `write` | Write content, creating directories as needed |
| `edit` | Replace exact text matches (surgical edits) |
| `bash` | Execute any command in working directory |

### Orchestrator Tool

| Tool | Description |
|------|-------------|
| `spawn_session` | Delegate a task to a sub-agent |

## Skills

Skills are CLI tools that agents invoke via bash. They cost tokens only when used.

**Example: Web Search Skill**

```bash
# In .descartes/skills/web-search
#!/bin/bash
curl -s "https://api.search.com?q=$1" | jq '.results[].title'
```

**Agent usage:**
```
bash: .descartes/skills/web-search "rust error handling best practices"
```

See [docs/SKILLS.md](docs/SKILLS.md) for creating custom skills.

## Commands

```bash
descartes spawn     # Spawn an agent with a task
descartes ps        # List running agents
descartes kill      # Terminate an agent
descartes pause     # Pause an agent
descartes resume    # Resume a paused agent
descartes attach    # Get credentials to attach external TUI
descartes logs      # View agent logs
descartes doctor    # Check system health
descartes init      # Initialize a new project
```

### Spawn Options

```bash
descartes spawn --task "Your task here" \
  --provider anthropic \           # anthropic, openai, ollama, deepseek, groq
  --model claude-sonnet-4-20250514 \
  --tool-level orchestrator \      # orchestrator, minimal, readonly
  --no-spawn \                     # Prevent sub-session spawning
  --transcript-dir ./logs          # Custom transcript location
```

## Configuration

```bash
# Environment variables
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."

# Or config file (~/.descartes/config.toml)
```

```toml
[providers]
primary = "anthropic"

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-sonnet-4-20250514"
```

## Providers

| Provider | Type | Configuration |
|----------|------|---------------|
| **Anthropic** | Cloud | `ANTHROPIC_API_KEY` |
| **OpenAI** | Cloud | `OPENAI_API_KEY` |
| **DeepSeek** | Cloud | `DEEPSEEK_API_KEY` |
| **Groq** | Cloud | `GROQ_API_KEY` |
| **Ollama** | Local | `OLLAMA_ENDPOINT` (default: localhost:11434) |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  descartes spawn --task "..." --tool-level orchestrator     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Tool Registry                                              │
│  → Returns [read, write, edit, bash, spawn_session]         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Model Backend (Anthropic/OpenAI/Ollama/...)                │
│  → Streaming conversation with tool calls                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Tool Executors                                             │
│  → Execute tools, return results                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Transcript Writer                                          │
│  → .scud/sessions/2025-12-04-session-abc123.json           │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
descartes/
├── core/           # Core library (tools, providers, transcripts)
├── cli/            # Command-line interface
├── daemon/         # Background RPC server (optional)
├── gui/            # Native GUI with Iced (optional)
└── docs/           # Documentation
```

## Development

```bash
# Build
cargo build --release

# Test
cargo test

# Run from source
cargo run -p descartes-cli -- spawn --task "Hello world"
```

## Comparison

| Feature | Claude Code | Aider | Descartes |
|---------|-------------|-------|-----------|
| Tools | 40+ | 10+ | **4** |
| System prompt | 10k+ tokens | 5k+ tokens | **200 tokens** |
| Extensibility | MCP (heavy) | Custom (complex) | **Skills (simple)** |
| Observability | Limited | Logs | **Full transcripts** |
| Recursive control | None | None | **Built-in** |

## References

- [Pi: Building a Coding Agent](https://marioslab.io/posts/pi/building-a-coding-agent/) — Philosophy inspiration
- [12-Factor Agents](https://github.com/humanlayer/12-factor-agents) — Design principles

## License

MIT

---

<p align="center">
  <strong>Minimal tools. Maximum observability. No recursive agents.</strong>
</p>
