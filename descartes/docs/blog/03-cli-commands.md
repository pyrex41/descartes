# CLI Commands Reference

*Master the Descartes command line*

---

The Descartes CLI is your primary interface for spawning, managing, and monitoring AI agents. This guide covers every command with practical examples.

## Command Overview

```bash
descartes <COMMAND>

Commands:
  spawn       Create and run an AI agent
  ps          List running agents
  logs        View agent session logs
  kill        Terminate an agent
  pause       Pause a running agent
  resume      Resume a paused agent
  attach      Attach a TUI client to an agent
  init        Initialize Descartes in a directory
  doctor      Check system health
  tasks       Manage SCUD tasks
  workflow    Execute multi-phase workflows
  loop        Run iterative execution loops
  gui         Launch the native GUI
  completions Generate shell completions
```

---

## spawn — Create and Run Agents

The `spawn` command is how you start new agent sessions.

### Basic Usage

```bash
descartes spawn --task "Your task description"
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--task` | `-t` | Task/prompt for the agent | Required |
| `--provider` | `-p` | LLM provider to use | Primary from config |
| `--model` | `-m` | Specific model ID | Provider default |
| `--tool-level` | `-l` | Agent capability level | `orchestrator` |
| `--stream` | `-s` | Stream output in real-time | `false` |
| `--output` | `-o` | Custom transcript output path | Auto-generated |
| `--agent` | `-a` | Agent definition file | None |
| `--context` | `-c` | Additional context file | None |
| `--no-spawn` | | Disable sub-agent spawning | `false` |
| `--attachable` | | Create attach socket for TUI | `false` |

### Examples

```bash
# Basic task
descartes spawn --task "Add input validation to the signup form"

# With streaming output
descartes spawn -t "Refactor the database module" --stream

# Use specific provider and model
descartes spawn -t "Write documentation" -p openai -m gpt-4-turbo

# Read-only exploration
descartes spawn -t "Analyze code quality" --tool-level readonly

# Custom agent definition
descartes spawn -t "Design the API" --agent ~/.descartes/agents/architect.md

# Prevent sub-agent spawning
descartes spawn -t "Simple fix" --no-spawn

# Make attachable for external TUI
descartes spawn -t "Long task" --attachable
```

### Tool Levels

| Level | Capabilities |
|-------|-------------|
| `orchestrator` | Full access + sub-agent spawning |
| `minimal` | read, write, edit, bash (no spawning) |
| `readonly` | read, bash (no modifications) |
| `researcher` | Optimized for codebase research |
| `planner` | Can write to thoughts/ only |

---

## ps — List Running Agents

View all active agent sessions.

### Basic Usage

```bash
descartes ps
```

### Options

| Option | Description |
|--------|-------------|
| `--all` | Include completed/failed sessions |
| `--format` | Output format: `table` (default) or `json` |

### Example Output

```
ID       STATUS    TASK                              STARTED      PROVIDER
a1b2c3   running   Add authentication system         2 min ago    anthropic
d4e5f6   paused    Review security practices         15 min ago   openai
g7h8i9   thinking  Implement payment integration     30 sec ago   anthropic
```

### Status Values

| Status | Description |
|--------|-------------|
| `running` | Actively executing |
| `thinking` | Processing/generating response |
| `paused` | Suspended, can be resumed |
| `completed` | Finished successfully |
| `failed` | Encountered error |
| `terminated` | Manually killed |

---

## logs — View Session Logs

Access conversation transcripts and agent output.

### Basic Usage

```bash
descartes logs <SESSION_ID>
```

### Options

| Option | Short | Description |
|--------|-------|-------------|
| `--follow` | `-f` | Stream logs in real-time |
| `--format` | | Output format: `text` or `json` |
| `--limit` | `-n` | Number of entries to show |
| `--tail` | | Show last N entries |

### Examples

```bash
# View logs for session
descartes logs a1b2c3

# Follow logs in real-time
descartes logs a1b2c3 --follow

# JSON format for parsing
descartes logs a1b2c3 --format json

# Last 10 entries
descartes logs a1b2c3 --tail 10
```

---

## kill — Terminate Agents

Stop running agents gracefully or forcefully.

### Basic Usage

```bash
descartes kill <SESSION_ID>
```

### Options

| Option | Description |
|--------|-------------|
| `--force` | Send SIGKILL instead of SIGTERM |
| `--all` | Kill all running agents |

### Examples

```bash
# Graceful shutdown
descartes kill a1b2c3

# Force kill unresponsive agent
descartes kill a1b2c3 --force

# Kill all agents
descartes kill --all
```

---

## pause — Suspend Agents

Pause running agents to free resources or attach a TUI.

### Basic Usage

```bash
descartes pause <SESSION_ID>
```

### Options

| Option | Description |
|--------|-------------|
| `--force` | Use SIGSTOP for uncooperative agents |

### Examples

```bash
# Cooperative pause
descartes pause a1b2c3

# Force pause
descartes pause a1b2c3 --force
```

---

## resume — Continue Paused Agents

Resume a previously paused agent session.

### Basic Usage

```bash
descartes resume <SESSION_ID>
```

### Options

| Option | Description |
|--------|-------------|
| `--attach` | Resume and attach TUI immediately |

### Examples

```bash
# Simple resume
descartes resume a1b2c3

# Resume with TUI attachment
descartes resume a1b2c3 --attach
```

---

## attach — Connect External TUI

Attach an external terminal UI (Claude Code, OpenCode) to a paused agent.

### Basic Usage

```bash
descartes attach <SESSION_ID>
```

### Options

| Option | Description |
|--------|-------------|
| `--tui` | TUI type: `claude`, `opencode`, or `custom` |
| `--command` | Custom command for TUI |

### Examples

```bash
# Attach Claude Code
descartes attach a1b2c3 --tui claude

# Attach OpenCode
descartes attach a1b2c3 --tui opencode

# Custom TUI
descartes attach a1b2c3 --tui custom --command "my-tui --session"
```

### Attach Protocol

When attaching:
1. Agent must be paused
2. Descartes generates temporary credentials
3. TUI connects via WebSocket/Unix socket
4. Bidirectional I/O established
5. Previous output replayed to TUI

---

## init — Initialize Project

Set up Descartes in a directory.

### Basic Usage

```bash
descartes init
```

### Options

| Option | Description |
|--------|-------------|
| `--name` | Project name |
| `--dir` | Target directory |
| `--global` | Initialize global config only |

### What It Creates

```
.descartes/
├── config.toml       # Project configuration
└── session.json      # Session metadata

.scud/
├── sessions/         # Transcript storage
└── workflow-state.json
```

---

## doctor — Health Check

Diagnose system configuration and connectivity.

### Basic Usage

```bash
descartes doctor
```

### Checks Performed

```
✓ Rust toolchain found (1.75.0)
✓ SQLite database initialized
✓ Configuration file valid
✓ Anthropic API key configured
✓ OpenAI API key configured
✗ Grok API key not configured
✓ Ollama server reachable
✓ Storage directories writable
✓ Skills directory exists
```

---

## tasks — SCUD Task Management

Manage tasks in SCUD format for workflow execution.

### Subcommands

```bash
descartes tasks <SUBCOMMAND>

Subcommands:
  list    List all tasks
  show    Show task details
  next    Get next actionable task
  stats   Show task statistics
  use     Mark task as in-progress
  phases  List workflow phases
```

### Examples

```bash
# List all tasks
descartes tasks list

# Filter by status
descartes tasks list --status pending

# Show specific task
descartes tasks show TASK-001

# Get next task to work on
descartes tasks next

# Start working on a task
descartes tasks use TASK-001

# View workflow phases
descartes tasks phases
```

---

## workflow — Multi-Phase Execution

Execute structured workflows from PRD to implementation.

### Subcommands

```bash
descartes workflow <SUBCOMMAND>

Subcommands:
  flow      Execute full PRD-to-code workflow
  research  Run codebase research workflow
  plan      Create implementation plan
  implement Execute implementation plan
  list      List available workflows
  info      Show workflow details
```

### Flow Workflow

The flagship workflow: transform a PRD into working code.

```bash
descartes workflow flow --prd requirements.md
```

**Phases:**
1. **Ingest** — Parse PRD into tasks
2. **Review** — Optimize task graph
3. **Plan** — Generate implementation plans
4. **Implement** — Execute tasks wave-by-wave
5. **QA** — Monitor quality
6. **Summarize** — Generate documentation

### Options

| Option | Description |
|--------|-------------|
| `--prd` | Path to PRD file |
| `--resume` | Resume from saved state |
| `--tag` | Tag for this workflow run |
| `--dir` | Working directory |
| `--phase` | Start from specific phase |

### Examples

```bash
# Full workflow
descartes workflow flow --prd docs/prd.md

# Resume interrupted workflow
descartes workflow flow --prd docs/prd.md --resume

# Start from planning phase
descartes workflow flow --prd docs/prd.md --phase plan

# Research workflow only
descartes workflow research --topic "authentication patterns"

# Create implementation plan
descartes workflow plan --task "Add OAuth support"
```

---

## loop — Iterative Execution Loops

Commands for iterative execution loops that run a command repeatedly until completion. Useful for automating multi-step AI agent workflows that require repeated passes.

### Subcommands

```bash
descartes loop <SUBCOMMAND>

Subcommands:
  start   Start a new iterative loop
  status  Show current loop state
  resume  Resume an interrupted loop
  cancel  Cancel a running loop
```

### loop start

Start a new iterative loop that runs a command repeatedly until a completion condition is met.

```bash
descartes loop start [OPTIONS]
```

**Options:**

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--command` | `-c` | Command to run (e.g., "claude", "opencode") | Required |
| `--prompt` | `-p` | Task prompt for the agent | Required |
| `--completion-promise` | | Text that signals completion | `COMPLETE` |
| `--max-iterations` | `-m` | Maximum iterations (safety limit) | `20` |
| `--working-dir` | `-w` | Working directory | Current dir |
| `--backend` | | Backend type: `claude`, `opencode`, or `generic` | `generic` |
| `--auto-commit` | | Git commit after each iteration | `false` |
| `--timeout` | | Timeout per iteration in seconds | None |

**Example:**

```bash
# Start a loop with Claude
descartes loop start \
  --command claude \
  --prompt "Implement the user authentication feature" \
  --max-iterations 10 \
  --backend claude \
  --auto-commit

# Generic loop with custom completion text
descartes loop start \
  --command "python agent.py" \
  --prompt "Process all pending items" \
  --completion-promise "ALL_DONE" \
  --timeout 300
```

### loop status

Show the current state of a running or completed loop.

```bash
descartes loop status [OPTIONS]
```

**Options:**

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--state-file` | `-s` | Path to state file | `.descartes/loop-state.json` |

**Example Output:**

```
Loop Status
===========
  State file: ".descartes/loop-state.json"
  Iteration: 5
  Started: 2024-01-15T10:30:00Z
  Completed: No
  Last iteration: 2024-01-15T10:45:00Z

Config:
  Command: claude
  Max iterations: 20
  Completion promise: "COMPLETE"
```

### loop resume

Resume an interrupted loop from its saved state.

```bash
descartes loop resume [OPTIONS]
```

**Options:**

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--state-file` | `-s` | Path to state file | `.descartes/loop-state.json` |

**Example:**

```bash
# Resume from default state file
descartes loop resume

# Resume from custom state file
descartes loop resume --state-file ./custom-loop-state.json
```

### loop cancel

Cancel a running loop by marking it as completed in the state file.

```bash
descartes loop cancel [OPTIONS]
```

**Options:**

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--state-file` | `-s` | Path to state file | `.descartes/loop-state.json` |

**Example:**

```bash
descartes loop cancel
```

### How Loops Work

1. **Initialization** — The loop starts with your command and prompt
2. **Iteration** — Each iteration runs the command with context about the current iteration
3. **Completion Detection** — The loop checks output for the completion promise text
4. **Exit Conditions** — Loop exits when:
   - Completion promise is detected
   - Maximum iterations reached
   - User cancels with Ctrl+C
   - Timeout exceeded (if configured)
5. **State Persistence** — State is saved after each iteration for resumability

---

## gui — Launch Desktop GUI

Start the native graphical interface.

### Basic Usage

```bash
descartes gui
```

The GUI provides:
- Real-time agent monitoring
- Visual DAG workflow editor
- Chat interface with streaming
- Time-travel debugging
- Session management

---

## completions — Shell Completions

Generate shell completion scripts.

### Usage

```bash
# Bash
descartes completions bash > ~/.local/share/bash-completion/completions/descartes

# Zsh
descartes completions zsh > ~/.zfunc/_descartes

# Fish
descartes completions fish > ~/.config/fish/completions/descartes.fish
```

---

## Global Options

Available on all commands:

| Option | Description |
|--------|-------------|
| `--config` | Path to config file |
| `--verbose` | Increase output verbosity |
| `--quiet` | Suppress non-essential output |
| `--help` | Show help information |
| `--version` | Show version |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DESCARTES_CONFIG` | Path to config file |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `XAI_API_KEY` | xAI/Grok API key |
| `DEEPSEEK_API_KEY` | DeepSeek API key |
| `GROQ_API_KEY` | Groq API key |

---

## Next Steps

- **[Providers & Config →](04-providers-configuration.md)** — Configure LLM providers
- **[Session Management →](05-session-management.md)** — Deep dive into sessions
- **[Flow Workflow →](07-flow-workflow.md)** — Master the PRD-to-code pipeline

---

*Now you're ready to command your AI agents like a pro.*
