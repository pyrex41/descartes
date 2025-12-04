# Descartes: CLI-First AI Agent Orchestration

**Version**: 0.2.0 - Pi-Style Minimal Tooling

Descartes is a Rust framework for building AI coding agents with minimal, observable tooling. Following [Pi's philosophy](https://marioslab.io/posts/pi/building-a-coding-agent/): "if you don't need it, don't build it."

## Core Philosophy

- **Minimal Tools**: 4 core tools (read, write, edit, bash) are sufficient for effective coding agents
- **Progressive Disclosure**: Additional capabilities via "skills" (CLI tools), not bloated MCP servers
- **Observability**: All tool use goes through bash, fully visible in transcripts
- **Recursive Prevention**: Sub-sessions cannot spawn their own sub-agents

## Quick Start

### Building

```bash
cd descartes
cargo build --release
```

### Running a Spawn

```bash
# Basic spawn with orchestrator tools
descartes spawn --task "List all Rust files in this project"

# Spawn with minimal tools (no sub-session spawning)
descartes spawn --task "Fix the type error" --tool-level minimal

# Pipe content to the agent
cat error.log | descartes spawn --task "Analyze this error"
```

### Transcripts

All sessions save transcripts to `.scud/sessions/` (or `~/.descartes/sessions/`):

```bash
# View recent transcripts
ls -lt .scud/sessions/ | head

# Transcripts are JSON with metadata and entries
cat .scud/sessions/2025-12-03-10-30-00-abc12345.json | jq '.metadata'
```

## Project Structure

```
descartes/
├── core/                          # Core library
│   └── src/
│       ├── lib.rs                 # Library root
│       ├── tools/                 # Minimal tool definitions
│       │   ├── definitions.rs     # 5 tools: read, write, edit, bash, spawn_session
│       │   ├── registry.rs        # ToolLevel enum and system prompts
│       │   └── executors.rs       # Tool execution implementations
│       ├── session_transcript.rs  # Transcript writer
│       └── providers.rs           # ModelBackend implementations
├── cli/                           # Command-line interface
│   └── src/
│       ├── main.rs               # CLI entry point with spawn command
│       └── commands/spawn.rs     # Spawn implementation
├── daemon/                        # Background daemon (optional)
├── gui/                           # Native GUI (optional)
├── docs/
│   └── SKILLS.md                 # Skills pattern documentation
└── examples/
    └── skills/
        └── web-search/           # Example skill implementation
```

## Tool Levels

Descartes uses capability-based tool levels:

| Level | Tools | Use Case |
|-------|-------|----------|
| **Minimal** | read, write, edit, bash | Sub-sessions, focused tasks |
| **Orchestrator** | minimal + spawn_session | Top-level agents that delegate |
| **ReadOnly** | read, bash | Exploration, planning |

```bash
# Orchestrator (default) - can spawn sub-sessions
descartes spawn --task "Review and fix all type errors" --tool-level orchestrator

# Minimal - cannot spawn sub-sessions
descartes spawn --task "Fix this one function" --tool-level minimal

# ReadOnly - exploration only
descartes spawn --task "Explain how authentication works" --tool-level readonly
```

## Tools

### Core Tools (Minimal Level)

| Tool | Description |
|------|-------------|
| **read** | Read file contents with optional offset/limit for large files |
| **write** | Write content to file, creating directories as needed |
| **edit** | Surgical text replacement (old_text must match exactly) |
| **bash** | Execute bash commands in working directory |

### Orchestrator Tool

| Tool | Description |
|------|-------------|
| **spawn_session** | Spawn a sub-session for delegated tasks |

Sub-sessions are spawned with `--no-spawn --tool-level minimal`, preventing recursive agent spawning.

## Spawn Command

```bash
descartes spawn [OPTIONS] --task <TASK>

Options:
  -t, --task <TASK>              Task or prompt for the agent (required)
  -p, --provider <PROVIDER>      Model provider: anthropic, openai, ollama, deepseek, groq
  -m, --model <MODEL>            Specific model to use
  -s, --system <SYSTEM>          Custom system prompt
      --stream                   Stream output in real-time (default: true)
      --tool-level <LEVEL>       Tool level: minimal, orchestrator, readonly (default: orchestrator)
      --no-spawn                 Prevent spawning sub-sessions (for recursive prevention)
      --transcript-dir <DIR>     Custom transcript directory (default: .scud/sessions/)
```

## Skills Pattern

Instead of MCP servers that inject 2000-5000 tokens into every session, Descartes uses "skills" - CLI tools invoked via bash:

```bash
# Agent discovers skill via system prompt or README
# Agent uses bash to invoke the skill
bash: web-search "rust async patterns"
```

See [docs/SKILLS.md](docs/SKILLS.md) for creating skills.

**Context cost comparison:**
| Approach | Context Cost | When Paid |
|----------|-------------|-----------|
| MCP Server | ~2000-5000 tokens | Every message |
| Skill (CLI) | ~50-100 tokens | Only when used |

## Session Transcripts

Every session saves a JSON transcript:

```json
{
  "metadata": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "started_at": "2025-12-03T10:30:00Z",
    "ended_at": "2025-12-03T10:35:00Z",
    "provider": "anthropic",
    "model": "claude-3-5-sonnet",
    "task": "Fix the type error in main.rs",
    "is_sub_session": false,
    "tool_level": "orchestrator"
  },
  "entries": [
    {"timestamp": "...", "role": "user", "content": "..."},
    {"timestamp": "...", "role": "assistant", "content": "..."},
    {"timestamp": "...", "role": "tool_call", "content": "...", "tool_name": "bash", "tool_id": "..."},
    {"timestamp": "...", "role": "tool_result", "content": "...", "tool_id": "..."}
  ]
}
```

## Providers

### Supported Providers

| Provider | Type | Configuration |
|----------|------|---------------|
| **Anthropic** | API | `ANTHROPIC_API_KEY` |
| **OpenAI** | API | `OPENAI_API_KEY` |
| **Ollama** | Local | `OLLAMA_ENDPOINT` (default: localhost:11434) |
| **DeepSeek** | API | `DEEPSEEK_API_KEY` |
| **Groq** | API | `GROQ_API_KEY` |

### Configuration

```bash
# Environment variables
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."

# Or via config file (~/.descartes/config.toml)
[providers]
primary = "anthropic"

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"
```

## System Prompts

Each tool level has a ~200 token system prompt (not 10,000+ like Claude Code):

**Minimal prompt emphasizes:**
- Using bash for file operations (ls, grep, find)
- Reading files before editing
- Edit requires exact text match
- Being concise

**Orchestrator prompt adds:**
- Using spawn_session for delegation
- Sub-sessions stream output and save transcripts

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     descartes spawn                          │
│  --task "..." --tool-level orchestrator                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Tool Registry                             │
│  get_tools(ToolLevel::Orchestrator)                          │
│  → [read, write, edit, bash, spawn_session]                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Model Backend                              │
│  ProviderFactory::create("anthropic", config)                │
│  → Sends request with tools to LLM                           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Tool Executors                              │
│  execute_tool("bash", args, working_dir)                     │
│  → Returns ToolResult { success, output, metadata }          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                Transcript Writer                             │
│  Saves all messages and tool calls to JSON                   │
│  → .scud/sessions/2025-12-03-10-30-00-abc12345.json         │
└─────────────────────────────────────────────────────────────┘
```

## Development

### Running Tests

```bash
# Core library tests (tools, transcripts)
cargo test -p descartes-core --lib

# CLI tests
cargo test -p descartes-cli

# All tests
cargo test
```

### Adding a New Tool

1. Add definition in `core/src/tools/definitions.rs`
2. Add executor in `core/src/tools/executors.rs`
3. Add to appropriate tool level in `core/src/tools/registry.rs`
4. Export from `core/src/tools/mod.rs`
5. Add tests

## References

- [Pi: Building a Coding Agent](https://marioslab.io/posts/pi/building-a-coding-agent/) - Philosophy inspiration
- [HumanLayer 12-Factor Agents](https://github.com/humanlayer/12-factor-agents) - Attach pattern inspiration

## License

MIT

---

**Minimal tools. Maximum observability. No recursive agents.**
