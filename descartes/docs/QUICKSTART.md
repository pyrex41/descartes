# Descartes Quickstart Guide

A hands-on tutorial to get you from zero to running AI agents in 10 minutes.

## Prerequisites

- **Rust toolchain** — [Install Rust](https://rustup.rs/)
- **An API key** from one of: Anthropic, OpenAI, DeepSeek, Groq, or Ollama (local)

## Step 1: Install Descartes

```bash
# From crates.io (recommended)
cargo install descartes

# Or build from source
git clone https://github.com/anthropics/descartes.git
cd descartes
cargo build --release
# Binary is at ./target/release/descartes
```

Verify installation:

```bash
descartes --version
descartes doctor
```

## Step 2: Configure API Key

Choose your provider and set the API key:

```bash
# Anthropic (recommended)
export ANTHROPIC_API_KEY="sk-ant-..."

# OpenAI
export OPENAI_API_KEY="sk-..."

# DeepSeek
export DEEPSEEK_API_KEY="..."

# Groq
export GROQ_API_KEY="..."

# Local with Ollama (no API key needed)
ollama serve  # Start Ollama first
```

Pro tip: Add the export to your `~/.bashrc` or `~/.zshrc`.

## Step 3: Run Your First Agent

Create a test file to work with:

```bash
mkdir -p /tmp/descartes-demo
cd /tmp/descartes-demo

cat > hello.py << 'EOF'
def greet(name):
    print("Hello " + name)

greet("World")
EOF
```

Now spawn an agent:

```bash
descartes spawn --task "Add type hints to hello.py"
```

Watch it work! You'll see the agent:
1. Read the file
2. Analyze the code
3. Edit the file with type hints
4. (Possibly) verify the changes

## Step 4: Explore Tool Levels

Descartes has three tool levels. Try each one:

### Readonly Mode (Safe Exploration)

```bash
descartes spawn --task "Explain what hello.py does" --tool-level readonly
```

The agent can only read files and run bash — it cannot modify anything.

### Minimal Mode (Focused Work)

```bash
descartes spawn --task "Add a docstring to the greet function" --tool-level minimal
```

Full editing power, but cannot spawn sub-agents. Good for focused tasks.

### Orchestrator Mode (Default)

```bash
descartes spawn --task "Create a test file for hello.py and run the tests"
```

Can spawn sub-agents to delegate work. Best for complex multi-step tasks.

## Step 5: View Transcripts

Every agent run produces a JSON transcript. This is Descartes' killer feature — full observability.

```bash
# List all sessions
ls .scud/sessions/

# View the latest transcript
cat .scud/sessions/*.json | jq '.'

# See just the tool calls
cat .scud/sessions/*.json | jq '.entries[] | select(.role == "tool_call")'

# See what files were read
cat .scud/sessions/*.json | jq '.entries[] | select(.role == "tool_call" and .tool_name == "read") | .args.path'

# See what edits were made
cat .scud/sessions/*.json | jq '.entries[] | select(.role == "tool_call" and .tool_name == "edit")'
```

## Step 6: Process Management

Monitor and control running agents:

```bash
# What's running?
descartes ps

# See logs from a specific agent
descartes logs <session-id>

# Kill a runaway agent
descartes kill <session-id>

# Pause/resume an agent
descartes pause <session-id>
descartes resume <session-id>
```

The `descartes ps` command shows all active agents with their IDs, tasks, and status.

## Step 7: Doctor Check

Verify your environment is correctly configured:

```bash
descartes doctor
```

This checks:
- API key configuration
- Provider connectivity
- Required directories
- Skill availability

## Step 8: Using Skills

Skills are CLI tools that agents invoke via bash. They cost tokens only when used, unlike MCP servers which inject tokens into every message.

### Discover Available Skills

```bash
# List skills in your project
ls .descartes/skills/ 2>/dev/null || echo "No project skills yet"

# List global skills
ls ~/.descartes/skills/ 2>/dev/null || echo "No global skills yet"
```

### Create a Simple Skill

```bash
mkdir -p .descartes/skills

cat > .descartes/skills/word-count << 'EOF'
#!/bin/bash
# Count words in a file
wc -w "$1"
EOF

chmod +x .descartes/skills/word-count
```

Now agents can use it:

```bash
descartes spawn --task "Count the words in hello.py using the word-count skill"
```

See [SKILLS.md](SKILLS.md) for more skill examples.

## Step 9: Using Different Providers

### Switch Providers

```bash
# Use OpenAI
descartes spawn --task "..." --provider openai --model gpt-4o

# Use local Ollama
descartes spawn --task "..." --provider ollama --model llama3

# Use DeepSeek (good for code)
descartes spawn --task "..." --provider deepseek --model deepseek-coder

# Use Groq (fast inference)
descartes spawn --task "..." --provider groq --model llama-3.3-70b-versatile
```

### Configure Default Provider

Create `~/.descartes/config.toml`:

```toml
[providers]
primary = "anthropic"

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-sonnet-4-20250514"
```

## Step 10: Advanced Features (Optional)

### Daemon Mode

For persistent sessions and RPC access:

```bash
# Start the daemon
descartes-daemon --http-addr 127.0.0.1:19280

# Spawn via HTTP RPC
curl -X POST http://127.0.0.1:19280 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"agent.spawn","params":{"task":"Hello world"}}'
```

### Lisp/Swank Integration

If you work with Common Lisp (SBCL), Descartes includes Swank protocol support:

```bash
# Spawn a Lisp-capable agent
descartes spawn --task "Evaluate (+ 1 2)" --agent-type lisp

# The agent can:
# - Connect to SBCL's Swank server
# - Evaluate Lisp code
# - Handle debugger conditions
# - Inspect values interactively
```

Requirements:
- SBCL installed and in PATH
- Swank/SLIME loaded

### GUI Mode

For visual session management (requires building with GUI feature):

```bash
cargo install descartes --features gui
descartes-gui
```

## Common Patterns

### Bug Fixing

```bash
descartes spawn --task "Fix the error on line 42 of src/parser.rs"
```

### Code Review

```bash
descartes spawn --task "Review src/auth.rs for security issues" --tool-level readonly
```

### Refactoring

```bash
descartes spawn --task "Refactor the database module to use async/await"
```

### Documentation

```bash
descartes spawn --task "Add docstrings to all public functions in src/lib.rs"
```

### Testing

```bash
descartes spawn --task "Write unit tests for the Calculator class in calc.py"
```

## Troubleshooting

### "No API key found"

```bash
# Check your environment
echo $ANTHROPIC_API_KEY
echo $OPENAI_API_KEY

# Run doctor
descartes doctor
```

### "Connection refused"

For cloud providers, check:
- API key is valid
- Internet connection works
- No firewall blocking

For Ollama:
```bash
# Make sure Ollama is running
ollama serve

# Verify it's responding
curl http://localhost:11434/api/tags
```

### "Agent seems stuck"

```bash
# Check what it's doing
descartes logs <session-id>

# Kill it if needed
descartes kill <session-id>
```

### "Permission denied on skill"

```bash
chmod +x .descartes/skills/<skill-name>
```

## Next Steps

- Read the [full documentation](../README.md)
- Learn about [creating skills](SKILLS.md)
- Explore the [project structure](../README.md#project-structure)
- Check out [common workflows](../README.md#common-workflows)

---

**You're ready!** Start with simple tasks, review the transcripts to understand what agents do, and gradually try more complex workflows.
