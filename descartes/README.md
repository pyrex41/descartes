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

## What Descartes Actually Does

Descartes is a **CLI tool that runs AI agents**. You give it a task, it spawns an agent, the agent uses tools to complete the task, and you get a full transcript of everything that happened.

```
You                          Descartes                       AI Model
 │                              │                               │
 │  "Fix the bug in auth.rs"   │                               │
 │─────────────────────────────>│                               │
 │                              │   [task + 4 tools available]  │
 │                              │──────────────────────────────>│
 │                              │                               │
 │                              │   tool_call: read auth.rs     │
 │                              │<──────────────────────────────│
 │                              │                               │
 │                              │   [file contents]             │
 │                              │──────────────────────────────>│
 │                              │                               │
 │                              │   tool_call: edit auth.rs     │
 │                              │<──────────────────────────────│
 │                              │                               │
 │  [streams output to you]    │                               │
 │<─────────────────────────────│                               │
 │                              │                               │
 │  [transcript saved to disk] │                               │
 │<─────────────────────────────│                               │
```

**That's it.** Descartes is the loop that:
1. Sends your task to an AI model
2. Gives the model 4 tools (read, write, edit, bash)
3. Executes tool calls and returns results
4. Streams the conversation to your terminal
5. Saves everything to a JSON transcript

## When to Use What

| Situation | Command | Tool Level |
|-----------|---------|------------|
| **Quick fix** — "fix this bug" | `descartes spawn --task "..."` | orchestrator (default) |
| **Exploration** — "explain this code" | `descartes spawn --task "..." --tool-level readonly` | readonly |
| **Focused work** — no sub-agents | `descartes spawn --task "..." --tool-level minimal` | minimal |
| **Check what's running** | `descartes ps` | — |
| **See what an agent did** | `descartes logs <id>` | — |
| **Kill a runaway agent** | `descartes kill <id>` | — |
| **Verify your setup works** | `descartes doctor` | — |

### Tool Levels Explained

**orchestrator** (default) — Full power. Can spawn sub-agents to delegate work.
```bash
descartes spawn --task "Refactor the auth module"
# Agent might spawn sub-sessions for different files
```

**minimal** — Same tools, but cannot spawn sub-agents. Use when you want focused work.
```bash
descartes spawn --task "Fix line 42 in auth.rs" --tool-level minimal
# Agent works alone, no delegation
```

**readonly** — Can only read files and run bash. Cannot modify anything.
```bash
descartes spawn --task "Explain how the auth flow works" --tool-level readonly
# Safe exploration, no changes to your code
```

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

### Essential (daily use)

```bash
descartes spawn --task "..."   # Run an agent
descartes ps                   # What's running?
descartes logs <id>            # What did it do?
descartes kill <id>            # Stop it
descartes doctor               # Is my setup working?
```

### Spawn Options

```bash
# Minimal — just the task
descartes spawn --task "Fix the bug in main.rs"

# With options
descartes spawn --task "Refactor auth" \
  --provider anthropic \              # anthropic, openai, ollama, deepseek, groq
  --model claude-sonnet-4-20250514 \  # specific model
  --tool-level minimal                # orchestrator, minimal, readonly
```

### Other Commands

```bash
descartes pause <id>    # Pause an agent
descartes resume <id>   # Resume a paused agent
descartes attach <id>   # Attach external TUI client
descartes init          # Initialize project directory
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

## Common Workflows

### Fix a Bug
```bash
descartes spawn --task "Fix the null pointer exception in src/parser.rs:142"
# Watch it work, check the transcript when done
```

### Understand Code Before Changing It
```bash
descartes spawn --task "Explain how the authentication middleware works" --tool-level readonly
# Safe exploration — agent can read but not modify
```

### Refactor with Delegation
```bash
descartes spawn --task "Refactor the database module to use connection pooling"
# Orchestrator level (default) — agent can spawn sub-agents for different files
```

### Quick Local Testing (No API Key)
```bash
# Start Ollama first
ollama serve

# Use a local model
descartes spawn --task "Add error handling to main.rs" --provider ollama --model llama3
```

### Check What Happened
```bash
# See recent activity
descartes logs

# Full transcript as JSON
cat .scud/sessions/*.json | jq '.'

# Just the tool calls
cat .scud/sessions/*.json | jq '.entries[] | select(.role == "tool_call")'
```

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
├── cli/            # Command-line interface ← start here
├── daemon/         # Background RPC server (optional)
├── gui/            # Native GUI with Iced (optional)
└── docs/           # Documentation
```

### What You Need

**Just the CLI** — Most users only need `descartes`. Install it, set an API key, run agents.

**Optional: Daemon** — Long-running background service for persistent sessions and RPC access. Only needed if you're building integrations or want agents to survive terminal closure.

**Optional: GUI** — Native desktop app for visual monitoring, session management, and debugging. Useful for complex multi-agent workflows, but not required for daily use.

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
