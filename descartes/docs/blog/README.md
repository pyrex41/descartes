# Descartes Documentation

*A comprehensive guide to the minimal, observable AI coding agent framework*

---

## Quick Links

| If you want to... | Read this |
|-------------------|-----------|
| Understand the philosophy | [The Pi Philosophy](01-introduction-the-pi-philosophy.md) |
| Get started quickly | [Getting Started](02-getting-started.md) |
| Learn all CLI commands | [CLI Commands](03-cli-commands.md) |
| Configure providers | [Providers & Configuration](04-providers-configuration.md) |
| Manage sessions | [Session Management](05-session-management.md) |
| Understand agent types | [Agent Types & Tool Levels](06-agent-types.md) |
| Automate PRD to code | [Flow Workflow](07-flow-workflow.md) |
| Extend with skills | [Skills System](08-skills-system.md) |
| Use the GUI | [GUI Features](09-gui-features.md) |
| Track sub-agents | [Sub-Agent Tracking](10-subagent-tracking.md) |
| Explore advanced features | [Advanced Features](11-advanced-features.md) |

---

## The Blog Series

### Foundations

1. **[The Pi Philosophy: Why Less is More in AI Agents](01-introduction-the-pi-philosophy.md)**

   Discover why 4 tools are all you need, and how Descartes strips AI agents down to their essence while maintaining full capability.

2. **[Getting Started with Descartes](02-getting-started.md)**

   Install Descartes, configure your first provider, and spawn your first AI agent in under 10 minutes.

3. **[CLI Commands Reference](03-cli-commands.md)**

   Complete reference for all Descartes commands: spawn, ps, logs, kill, pause, resume, attach, workflow, and more.

### Configuration

4. **[Providers and Configuration](04-providers-configuration.md)**

   Connect to Anthropic, OpenAI, xAI/Grok, Ollama, and custom providers. Learn about API configuration, environment variables, and secrets management.

5. **[Session Management](05-session-management.md)**

   Understand session lifecycle, transcripts, pause/resume functionality, and external TUI attachment.

6. **[Agent Types and Tool Levels](06-agent-types.md)**

   Choose the right capabilities: Orchestrator, Minimal, Planner, Researcher, Read-Only, and Lisp Developer levels.

### Workflows

7. **[The Flow Workflow: PRD to Production](07-flow-workflow.md)**

   Transform requirements documents into working code with the six-phase Flow workflow: Ingest, Review, Plan, Implement, QA, Summarize.

8. **[The Skills System](08-skills-system.md)**

   Extend agent capabilities without token bloat using CLI-based skills that cost nothing when not in use.

### Interfaces

9. **[The Descartes GUI](09-gui-features.md)**

   Visual control and monitoring with the native desktop application: chat interface, agent monitoring, DAG editor, and time-travel debugging.

### Advanced Topics

10. **[Sub-Agent Shadow Tracking](10-subagent-tracking.md)**

    Non-invasive monitoring of agent hierarchies when orchestrators delegate to sub-agents.

11. **[Advanced Features](11-advanced-features.md)**

    Time-travel debugging, brain/body restoration, state machines, distributed execution with ZeroMQ, persistent memory, and more.

---

## Core Concepts

### The 4 Tools

| Tool | Purpose |
|------|---------|
| `read` | Read any file |
| `write` | Create/overwrite files |
| `edit` | Surgical text replacement |
| `bash` | Execute any command |

### Tool Levels

| Level | Capabilities |
|-------|-------------|
| Orchestrator | Full access + sub-agent spawning |
| Minimal | 4 tools, no spawning |
| Planner | Read + write to thoughts/ only |
| Researcher | Read + bash (read-only) |
| Read-Only | Safe observation only |

### Flow Phases

```
PRD → Ingest → Review → Plan → Implement → QA → Summarize → Code
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI / GUI                             │
├─────────────────────────────────────────────────────────────┤
│                    Session Manager                           │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │Anthropic │  │  OpenAI  │  │   Grok   │  │  Ollama  │    │
│  │ Provider │  │ Provider │  │ Provider │  │ Provider │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
├─────────────────────────────────────────────────────────────┤
│              4 Core Tools + Skills System                    │
├─────────────────────────────────────────────────────────────┤
│              JSON Transcript Storage                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Quick Start

```bash
# Install
git clone https://github.com/your-org/descartes.git
cd descartes/descartes
cargo build --release

# Configure
export ANTHROPIC_API_KEY="sk-ant-..."
descartes init

# Run
descartes spawn --task "Explain this codebase" --stream
```

---

## Getting Help

- **CLI Help:** `descartes --help` or `descartes <command> --help`
- **Health Check:** `descartes doctor`
- **Issues:** [GitHub Issues](https://github.com/your-org/descartes/issues)

---

*Built with the Pi Philosophy: Minimal tools. Maximum capability. Full observability.*
