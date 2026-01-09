# Building on the Pi Philosophy

*How Descartes implements Mario Zechner's minimalist approach to AI coding agents*

---

## Credit Where It's Due

Descartes is heavily inspired by [Mario Zechner's PI Coding Agent](https://mariozechner.at/posts/2025-11-30-pi-coding-agent/). Mario articulated something many of us felt but couldn't put into words: modern AI coding tools have become bloated, and the solution is radical simplicity.

His core insight: **You don't need 40 tools. You need 4.**

```
read   → Read any file
write  → Write any file
edit   → Surgical text replacement
bash   → Execute any command
```

This isn't our idea. It's Mario's. We're just building on it.

## Why We Built Descartes

After reading Mario's post, we wanted to use PI. But we also had specific needs that required building our own implementation:

1. **Multiple LLM providers** — We switch between Anthropic, OpenAI, DeepSeek, and local models depending on the task
2. **Workflow automation** — We needed a way to go from PRD documents to implemented code with multiple coordinated agents
3. **Task management integration** — Our projects use SCUD for tracking work, and we wanted agents that understand that
4. **Team tooling** — GUI for debugging, pause/resume for long tasks, session management

So Descartes isn't a replacement for PI—it's our implementation of the same philosophy, with features we needed for our workflow.

## The Problem (Mario's Framing)

Mario described the problem perfectly. Modern agent frameworks come with:

- **40+ specialized tools** for every conceivable operation
- **10,000+ token system prompts** trying to anticipate every edge case
- **MCP servers** that add 2-5k tokens per message just for tool definitions
- **Opaque execution** where you can't see what the agent actually did

The result: bloated context windows, unpredictable behavior, and agents that feel like black boxes.

## The Pi Philosophy (Mario's Solution)

Mario's PI Coding Agent demonstrated that frontier models don't need extensive hand-holding. They've been trained on massive codebases. Give them basic file operations and shell access, and they figure out the rest.

Key principles from PI that we adopt:

### 1. Minimal Tools
Four primitives handle virtually any software engineering task. Need to search code? `bash` with grep. Need to run tests? `bash` with your test runner. Need to commit changes? `bash` with git.

### 2. Minimal System Prompts
PI's system prompt is under 1,000 tokens. Descartes aims for around 200. Frontier models don't need lengthy instructions—they've been RL-trained extensively on coding tasks.

### 3. Full Observability
Every action is logged. You can see exactly what happened, replay sessions, and debug failures. No magic, no hidden state.

### 4. CLI Tools Over MCP
As Mario put it, MCP servers inject thousands of tokens into every message whether you use them or not. CLI tools invoked via bash only cost tokens when actually used.

## What Descartes Adds

Where Descartes diverges from or extends PI:

### Multi-Provider Support
Switch between providers without changing your workflow:
```bash
descartes spawn --task "Fix the bug" --provider anthropic
descartes spawn --task "Fix the bug" --provider openai
descartes spawn --task "Fix the bug" --provider ollama --model llama3
```

### Controlled Sub-Agent Spawning
When an orchestrator agent needs to delegate, Descartes enforces a strict hierarchy:
```
Orchestrator Agent (can spawn)
    └── Sub-Agent (cannot spawn further)
```
This prevents the recursive explosion that can occur when agents spawn agents that spawn agents.

### Flow Workflow
A multi-phase system for turning PRDs into implemented code:
```bash
descartes workflow flow --prd docs/requirements.md
```

### SCUD Integration
Native integration with SCUD task tracking, including iterative loops that execute tasks wave-by-wave with verification.

### Pause/Resume
Long-running tasks can be paused and resumed:
```bash
descartes pause <session-id>
descartes resume <session-id>
```

### Optional GUI
A native GUI for visualizing sessions, monitoring progress, and debugging—useful but not required.

## The Architecture

```
┌─────────────────────────────────────────────────┐
│                   CLI / GUI                      │
├─────────────────────────────────────────────────┤
│              Session Manager                     │
├─────────────────────────────────────────────────┤
│   ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│   │ Anthropic│  │ OpenAI  │  │ Ollama  │  ...   │
│   │ Provider │  │ Provider│  │ Provider│        │
│   └─────────┘  └─────────┘  └─────────┘        │
├─────────────────────────────────────────────────┤
│           4 Core Tools (PI's design)            │
├─────────────────────────────────────────────────┤
│         JSON Transcript Storage                  │
└─────────────────────────────────────────────────┘
```

## Skills: Formalizing Mario's Suggestion

Mario rejected MCP servers in favor of using CLI tools directly. Descartes formalizes this as "skills"—CLI tools placed in a specific directory that agents can discover and invoke:

```bash
# In .descartes/skills/web-search
#!/bin/bash
curl -s "https://api.search.com?q=$1" | jq '.results[].title'
```

This isn't a new concept—it's just a convention for organizing the CLI tools Mario suggested using.

## What You Get

- **~200 token system prompts** instead of 10k
- **Full JSON transcripts** of every session
- **Multiple LLM providers** (Anthropic, OpenAI, DeepSeek, Groq, Ollama)
- **Native GUI** for visualization (optional)
- **Pause/Resume** for long-running tasks
- **Sub-agent tracking** with controlled delegation
- **Flow Workflow** for PRD-to-code automation
- **SCUD integration** for task management

## Further Reading

- **[Mario Zechner's PI Coding Agent](https://mariozechner.at/posts/2025-11-30-pi-coding-agent/)** — The original inspiration
- **[Getting Started →](02-getting-started.md)** — Install and run your first agent
- **[CLI Reference →](03-cli-commands.md)** — Master the command line
- **[Flow Workflow →](07-flow-workflow.md)** — Automate PRD to code

---

*Descartes: An implementation of Mario Zechner's Pi philosophy with multi-provider support and workflow automation.*
