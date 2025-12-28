# Getting Started with Descartes

*From zero to your first AI agent in 10 minutes*

---

## Prerequisites

Before installing Descartes, ensure you have:

- **Rust toolchain** (1.75+) — Install via [rustup](https://rustup.rs)
- **An API key** for at least one provider:
  - Anthropic (`ANTHROPIC_API_KEY`)
  - OpenAI (`OPENAI_API_KEY`)
  - xAI/Grok (`XAI_API_KEY`)
  - Or a local Ollama installation

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-org/descartes.git
cd descartes/descartes

# Build in release mode
cargo build --release

# Add to your PATH
export PATH="$PATH:$(pwd)/target/release"
```

### Verify Installation

```bash
descartes --version
# descartes 0.1.0

descartes doctor
# ✓ Rust toolchain found
# ✓ Database initialized
# ✓ Anthropic API key configured
# ...
```

## Configuration

### Quick Setup

Create your configuration file:

```bash
descartes init
```

This creates `~/.descartes/config.toml` with sensible defaults.

### API Keys

Set your API keys via environment variables:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export XAI_API_KEY="xai-..."
```

Or add them to your config file:

```toml
# ~/.descartes/config.toml

[providers.anthropic]
enabled = true
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"

[providers.openai]
enabled = true
api_key = "sk-..."
model = "gpt-4-turbo"
```

## Your First Agent

### Simple Task

Navigate to any project directory and spawn an agent:

```bash
cd ~/my-project

descartes spawn --task "Explain the structure of this codebase"
```

The agent will:
1. Read key files (README, package.json, Cargo.toml, etc.)
2. Explore the directory structure
3. Provide a summary of the codebase architecture

### With Streaming Output

See the agent's work in real-time:

```bash
descartes spawn --task "Find and fix any TODO comments" --stream
```

### Specify a Provider

Use a specific LLM provider:

```bash
# Use OpenAI
descartes spawn --task "Write unit tests" --provider openai

# Use local Ollama
descartes spawn --task "Review this code" --provider ollama --model codellama
```

## Understanding Tool Levels

Descartes agents operate at different capability levels:

### Orchestrator (Default)

Full capabilities including sub-agent spawning:

```bash
descartes spawn --task "Implement feature X" --tool-level orchestrator
```

Tools: `read`, `write`, `edit`, `bash`, `spawn_session`

### Minimal

Focused work without delegation:

```bash
descartes spawn --task "Fix this specific bug" --tool-level minimal
```

Tools: `read`, `write`, `edit`, `bash`

### Read-Only

Safe exploration mode:

```bash
descartes spawn --task "Audit security practices" --tool-level readonly
```

Tools: `read`, `bash` (read-only commands only)

## Managing Sessions

### List Running Agents

```bash
descartes ps
# ID       STATUS    TASK                          STARTED
# a1b2c3   running   Implement feature X           2 min ago
# d4e5f6   paused    Review authentication code    15 min ago
```

### View Logs

```bash
# Follow logs in real-time
descartes logs a1b2c3 --follow

# View full transcript
descartes logs a1b2c3 --format json
```

### Pause and Resume

```bash
# Pause an agent
descartes pause a1b2c3

# Resume later
descartes resume a1b2c3
```

### Terminate

```bash
# Graceful shutdown
descartes kill a1b2c3

# Force kill
descartes kill a1b2c3 --force
```

## Project Structure

After running Descartes in a project, you'll see:

```
my-project/
├── .descartes/
│   ├── config.toml         # Project-specific config
│   └── session.json        # Session metadata
├── .scud/
│   ├── sessions/           # Conversation transcripts
│   │   └── 2025-01-15-10-30-00-abc123.json
│   └── workflow-state.json # Workflow progress
└── ... your project files
```

## Transcript Format

Every session creates a JSON transcript:

```json
{
  "session_id": "abc123",
  "started_at": "2025-01-15T10:30:00Z",
  "task": "Explain the codebase",
  "provider": "anthropic",
  "model": "claude-3-5-sonnet",
  "entries": [
    {
      "role": "user",
      "content": "Explain the codebase",
      "timestamp": "2025-01-15T10:30:00Z"
    },
    {
      "role": "assistant",
      "content": "I'll analyze the codebase structure...",
      "tool_calls": [
        {
          "name": "read",
          "arguments": {"path": "README.md"}
        }
      ]
    }
  ]
}
```

## Common Workflows

### Bug Fixing

```bash
descartes spawn --task "Fix the bug in user authentication where sessions expire too quickly"
```

### Code Review

```bash
descartes spawn --task "Review the recent changes in src/api/ for security issues" --tool-level readonly
```

### Documentation

```bash
descartes spawn --task "Add docstrings to all public functions in src/lib/"
```

### Testing

```bash
descartes spawn --task "Write integration tests for the payment module"
```

## Troubleshooting

### Agent Not Starting

```bash
# Check system health
descartes doctor

# Common issues:
# - Missing API key
# - Invalid configuration
# - Database not initialized
```

### API Errors

```bash
# Test provider connectivity
descartes spawn --task "Say hello" --provider anthropic

# Check rate limits and quotas in your provider dashboard
```

### Session Recovery

If an agent crashes, find the transcript:

```bash
ls .scud/sessions/
# Review the most recent JSON file
```

---

## Next Steps

Now that you're up and running:

- **[CLI Commands →](03-cli-commands.md)** — Full command reference
- **[Agent Types →](06-agent-types.md)** — Understanding tool levels
- **[Flow Workflow →](07-flow-workflow.md)** — Automate multi-phase projects

---

*Happy coding with your new AI pair programmer!*
