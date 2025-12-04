---
date: 2025-12-03T21:54:41Z
researcher: Claude Code
git_commit: 9dec527e1a541e6cde651cc9c2c0442137f3bdd6
branch: master
repository: cap
topic: "Descartes vs Pi Coding Agent Comparison"
tags: [research, codebase, architecture, pi, descartes, agents, comparison]
status: complete
last_updated: 2025-12-03
last_updated_by: Claude Code
---

# Research: Descartes vs Pi Coding Agent Comparison

**Date**: 2025-12-03T21:54:41Z
**Researcher**: Claude Code
**Git Commit**: 9dec527e1a541e6cde651cc9c2c0442137f3bdd6
**Branch**: master
**Repository**: cap

## Research Question

Compare and contrast Descartes (this codebase) with Mario Zechner's Pi coding agent to glean ideas and insights for improvement.

## Summary

Descartes and Pi represent two philosophically different approaches to coding agent harnesses:

| Aspect | Descartes | Pi |
|--------|-----------|-----|
| **Philosophy** | Comprehensive orchestration platform | Minimal, observable, controllable |
| **Language** | Rust | TypeScript/Node.js |
| **UI** | Native GUI (Iced) + CLI + TUI attach | Scrollback TUI only |
| **System Prompt** | Large (Claude Code-derived) | ~1000 tokens total |
| **Tools** | Many specialized tools | 4 tools (read, write, edit, bash) |
| **Providers** | API, Headless CLI, Local | API only (multi-provider) |
| **Sessions** | Filesystem-based discovery | JSON-based with branching |
| **Observability** | RPC events, GUI dashboards | Full conversation visibility |
| **Sub-agents** | Background tasks, attach protocol | Spawn via bash (full visibility) |
| **Background Bash** | Yes (via daemon) | No (use tmux) |
| **MCP Support** | Yes (configured) | No (use CLI tools) |
| **Todos** | Built-in task systems | No (use files) |
| **Plan Mode** | Yes (via SCUD workflow) | No (use files) |

## Detailed Findings

### 1. Architectural Philosophy

#### Descartes: Comprehensive Orchestration
Descartes is a **production-grade multi-agent orchestration system** built in Rust. It provides:

- **Daemon-based architecture**: RPC server for agent lifecycle management (`descartes/daemon/src/main.rs`)
- **Native GUI**: Iced-based desktop application with multiple views (`descartes/gui/src/main.rs`)
- **Distributed execution**: ZeroMQ for remote agent communication (`descartes/core/src/zmq_*.rs`)
- **Semantic analysis**: Tree-Sitter parsing for code understanding (`descartes/agent-runner/`)
- **Visual workflows**: DAG editor for composing agent tasks (`descartes/gui/src/dag_editor.rs`)

#### Pi: Minimal and Observable
Pi is an **opinionated minimal coding agent** that prioritizes:

- **Context engineering**: Full control over what goes into the model's context
- **Observability**: Every aspect of interaction visible to the user
- **Simplicity**: If Mario doesn't need it, it doesn't get built
- **Single binary**: No daemon, no separate services

**Insight for Descartes**: Consider offering a "minimal mode" that strips away complexity for users who prefer direct control. Pi demonstrates that benchmark performance doesn't require extensive tooling.

### 2. LLM Provider Integration

#### Descartes Provider Pattern
```rust
// descartes/core/src/traits.rs:89-122
pub trait ModelBackend: Send + Sync {
    fn mode(&self) -> &ModelProviderMode;
    async fn complete(&self, request: ModelRequest) -> AgentResult<ModelResponse>;
    async fn stream(&self, request: ModelRequest) -> AgentResult<Box<dyn Stream<...>>>;
}

pub enum ModelProviderMode {
    Api { endpoint: String, api_key: String },
    Headless { command: String, args: Vec<String> },
    Local { endpoint: String, timeout_secs: u64 },
}
```

Descartes supports three distinct modes:
1. **API mode**: Direct HTTP to OpenAI, Anthropic
2. **Headless mode**: Spawn CLI tools (Claude Code, OpenCode) as child processes
3. **Local mode**: Connect to Ollama and other local services

#### Pi Provider Pattern
```typescript
// pi-ai: Unified LLM API
const claude = getModel('anthropic', 'claude-sonnet-4-5');
const gpt = getModel('openai', 'gpt-5.1-codex');
const gemini = getModel('google', 'gemini-2.5-flash');

// Context handoff between providers
const gptResponse = await complete(gpt, context); // Sees Claude's thinking as <thinking> tags
```

Pi focuses on:
- **Context handoff**: Seamlessly continue conversations across providers
- **Typesafe model registry**: Generated from OpenRouter and models.dev
- **Abort support**: Full pipeline abort with partial results
- **Split tool results**: Separate content for LLM vs UI display

**Ideas from Pi**:
1. **Cross-provider context handoff** - Descartes could serialize sessions in a provider-agnostic format for mid-session model switching
2. **Split tool results** - Return structured data for UI separate from LLM text
3. **Partial JSON parsing** - Stream tool arguments for better UX during long edits

### 3. System Prompt & Tool Design

#### Descartes: Feature-Rich Tools
Descartes inherits Claude Code-style tool definitions with extensive documentation, examples, and edge case handling. Tools include specialized operations for git commits, PR creation, and code review.

#### Pi: Minimal Toolset
Pi provides only **4 tools**:
```
read   - Read file contents (supports images)
write  - Write content to file
edit   - Replace exact text (surgical edits)
bash   - Execute bash command
```

Pi's system prompt is under **1000 tokens total** including tool definitions:
```
You are an expert coding assistant. You help users with coding tasks by
reading files, executing commands, editing code, and writing new files.

Available tools:
- read: Read file contents
- bash: Execute bash commands
- edit: Make surgical edits to files
- write: Create or overwrite files

Guidelines:
- Use bash for file operations like ls, grep, find
- Use read to examine files before editing
- Use edit for precise changes (old text must match exactly)
- Use write only for new files or complete rewrites
- Be concise in your responses
```

**Key insight**: Pi achieved **competitive benchmark results** on Terminal-Bench 2.0 with this minimal setup. The models are RL-trained to understand coding agents without extensive prompting.

**Ideas for Descartes**:
1. **Offer minimal prompt mode** - Let users opt into a lean system prompt
2. **Progressive tool disclosure** - Only inject tool definitions when needed
3. **Measure prompt overhead** - Track context consumption from tools/prompts

### 4. TUI Architecture

#### Descartes: TUI Attachment Protocol
Descartes uses a sophisticated **attach protocol** for connecting TUI clients to paused agents:

```rust
// descartes/daemon/src/claude_code_tui.rs
pub struct ClaudeCodeTuiHandler {
    stdin_tx: mpsc::Sender<Vec<u8>>,
    stdout_rx: broadcast::Receiver<Vec<u8>>,
    stderr_rx: broadcast::Receiver<Vec<u8>>,
    output_buffer: Arc<RwLock<OutputBuffer>>,
}

// Handshake with token validation, historical output replay
async fn handle_connection(&mut self, stream: UnixStream) -> DaemonResult<()> {
    let session_info = self.perform_handshake(&mut reader, &mut writer).await?;
    self.send_historical_output(&mut writer).await?;
    self.run_io_loop(&mut reader, &mut writer, session_info).await?;
}
```

This enables:
- Pause agent, attach any TUI client, resume
- Historical output replay for late attachers
- Multiple clients can connect simultaneously

#### Pi: Scrollback-Based TUI
Pi uses a **non-fullscreen TUI** that works with the terminal's native scrollback:

```typescript
// pi-tui: Differential rendering
class Component {
    render(width: number): string[];  // Returns lines with ANSI codes
    handleInput?(data: string): void;
}

// Algorithm:
// 1. First render: output all lines
// 2. Width changed: clear and re-render
// 3. Normal update: find first diff line, re-render from there
```

Benefits of this approach:
- **Native scrolling** works without simulation
- **Built-in search** via terminal's scrollback
- **Memory efficient** (just track rendered lines)
- **Reduced flicker** with synchronized output (CSI ?2026h)

**Ideas for Descartes**:
1. **Consider a lightweight TUI option** - Not everything needs the full GUI/attach infrastructure
2. **Differential rendering** - Could reduce GUI update costs
3. **Scrollback preservation** - CLI mode could use scrollback-based approach

### 5. Session & State Management

#### Descartes: Filesystem Discovery
```rust
// descartes/core/src/session_manager.rs
impl SessionManager for FileSystemSessionManager {
    async fn discover_sessions(&self) -> Result<Vec<Session>, SessionError> {
        for search_path in &self.config.search_paths {
            let workspace_paths = self.scan_directory(search_path, 0);
            for path in workspace_paths {
                if let Some(session) = self.load_session_from_path(&path) {
                    sessions.push(session);
                }
            }
        }
    }
}

fn is_workspace(&self, path: &Path) -> bool {
    path.join(".scud").exists() || path.join("config.toml").exists()
}
```

Sessions are discovered by scanning directories for `.scud/` markers.

#### Pi: Simple Session JSON
Pi stores sessions as JSON files with:
- Continue/resume semantics
- **Session branching** (fork from any point)
- HTML export capability
- Headless JSON streaming mode

**Ideas from Pi**:
1. **Session branching** - Fork sessions from any conversation point
2. **Session export** - HTML export for sharing/archiving
3. **Simpler session format** - Consider lighter-weight session storage

### 6. Sub-Agents & Parallel Work

#### Descartes: Daemon-Managed Agents
Descartes spawns and monitors agents via the daemon:
```rust
// descartes/core/src/agent_runner.rs
impl AgentRunner for LocalProcessRunner {
    async fn spawn(&self, config: AgentConfig) -> AgentResult<Box<dyn AgentHandle>>;
    async fn pause(&self, agent_id: &Uuid, force: bool) -> AgentResult<()>;
    async fn resume(&self, agent_id: &Uuid) -> AgentResult<()>;
}
```

Supports cooperative and forced pause (SIGSTOP), background health checking, and process lifecycle management.

#### Pi: No Sub-Agents (By Design)
Mario explicitly rejects built-in sub-agents:

> "When Claude Code needs to do something complex, it often spawns a sub-agent to handle part of the task. You have zero visibility into what that sub-agent does. It's a black box within a black box."

Pi's alternative:
```markdown
<!-- Custom slash command for code review -->
Spawn yourself as a sub-agent via bash to do a code review: $@
Use `pi --print` with appropriate arguments.
Pass a prompt to the sub-agent asking it to review the code.
Do not read the code yourself. Let the sub-agent do that.
Report the sub-agent's findings.
```

Benefits:
- Full visibility into sub-agent output
- User can interact with sub-agent session directly
- Session artifacts can be saved for later

**Tension to consider**: Descartes' approach enables sophisticated orchestration but sacrifices observability. Pi's approach maintains visibility but limits parallelism.

**Possible synthesis**: Descartes could offer:
1. **Verbose mode** - Full sub-agent output in main context
2. **Session serialization** - Save sub-agent sessions for inspection
3. **Manual attach** - Let users hop into any sub-agent session

### 7. Background Processes & Background Bash

#### Descartes: Daemon-Managed Background
Background processes are managed by the daemon with event streaming to GUI/CLI clients.

#### Pi: Use Tmux
Pi explicitly rejects background bash:

> "Background process management adds complexity: you need process tracking, output buffering, cleanup on exit, and ways to send input to running processes."

Instead:
```bash
# Pi debugging in tmux
pi: I'll create a tmux session and run lldb...
# User can attach to tmux session and co-debug
```

This leverages existing infrastructure (tmux) rather than reinventing it.

**Idea for Descartes**: Document tmux workflows as an alternative to daemon-managed background processes. Users get:
- `tmux list-sessions` to see all running processes
- Direct terminal interaction
- Standard tmux session management

### 8. MCP vs CLI Tools

#### Descartes: MCP Configured
```json
// .mcp.json
{
  "mcpServers": {
    "task-master-ai": {
      "command": "npx",
      "args": ["-y", "task-master-ai"],
      "env": { "ANTHROPIC_API_KEY": "..." }
    }
  }
}
```

#### Pi: No MCP (By Design)
Mario's critique:

> "Popular MCP servers like Playwright MCP (21 tools, 13.7k tokens) or Chrome DevTools MCP (26 tools, 18k tokens) dump their entire tool descriptions into your context on every session. That's 7-9% of your context window gone before you even start working."

Pi's alternative: **CLI tools with READMEs**
- Agent reads README only when needed (progressive disclosure)
- Token cost paid only when tool is used
- Composable with bash pipes

**Idea for Descartes**:
1. **Lazy tool loading** - Only inject tool definitions when the agent actually uses them
2. **MCP -> CLI bridge** - Use mcporter to wrap MCP servers as CLI tools
3. **Measure context overhead** - Show users how much context MCP servers consume

### 9. Todos & Planning

#### Descartes: Built-in Task Systems
Multiple task systems coexist:
- SCUD workflow system (`.scud/tasks/`)
- SCG format storage (`scg_task_storage.rs`)
- GUI task board (`task_board.rs`)
- DAG workflow editor

#### Pi: Files as State
> "pi does not and will not support built-in to-dos. In my experience, to-do lists generally confuse models more than they help."

Pi's approach:
```markdown
# TODO.md
- [x] Implement user authentication
- [x] Add database migrations
- [ ] Write API documentation
- [ ] Add rate limiting
```

Benefits:
- Externally stateful (survives session boundaries)
- Version-controlled with code
- Visible without special tooling
- Agent can read/update naturally

**Observation**: Both approaches have merit. SCUD provides structured workflow management valuable for teams. Pi's file-based approach is simpler for individuals.

**Possible synthesis**: Let users choose complexity level:
1. **Simple mode**: File-based todos (`TODO.md`)
2. **Structured mode**: SCUD workflows
3. **Visual mode**: DAG editor for complex dependencies

### 10. Benchmark Performance

Pi achieved **competitive results** on Terminal-Bench 2.0 with its minimal approach:

> "I performed a complete run with five trials per task, which makes the results eligible for submission to the leaderboard."

Key observation:
> "Also note the ranking of Terminus 2 on the leaderboard. Terminus 2 is the Terminal-Bench team's own minimal agent that just gives the model a tmux session... And it's holding its own against agents with far more sophisticated tooling."

**Insight**: Complexity doesn't necessarily improve performance. The models have been "RL-trained up the wazoo" and understand coding agents inherently.

## Actionable Ideas for Descartes

### High-Value, Low-Effort
1. **Measure context overhead** - Track how much context is consumed by tools/prompts vs actual work
2. **Document tmux workflows** - Provide examples of using tmux for background processes
3. **Session export** - Add HTML/markdown export for sessions
4. **Session branching** - Fork from any point in conversation

### Medium-Value, Medium-Effort
5. **Minimal prompt mode** - Optional lean system prompt (~1000 tokens)
6. **Split tool results** - Separate structured data for UI from LLM text
7. **Lazy MCP loading** - Only inject tool definitions when used
8. **Cross-provider handoff** - Serialize sessions in provider-agnostic format

### High-Value, Higher-Effort
9. **Lightweight TUI option** - Scrollback-based TUI without daemon
10. **Sub-agent visibility** - Save full sub-agent sessions for inspection
11. **Progressive tool disclosure** - Agent requests tools, not always loaded

## Code References

- **Provider trait**: `descartes/core/src/traits.rs:89-122` - ModelBackend definition
- **Agent runner**: `descartes/core/src/agent_runner.rs:37-638` - Process management
- **Session manager**: `descartes/core/src/session_manager.rs:1-693` - Filesystem discovery
- **TUI attach**: `descartes/daemon/src/claude_code_tui.rs:1-300` - Attach protocol
- **GUI main**: `descartes/gui/src/main.rs:41-830` - Iced application
- **RPC server**: `descartes/daemon/src/rpc_server.rs:28-142` - JSON-RPC methods
- **MCP config**: `.mcp.json:1-19` - MCP server configuration

## Architecture Documentation

### Descartes
- Multi-crate Rust workspace (core, cli, gui, daemon, agent-runner)
- Daemon-based architecture for agent lifecycle
- Three provider modes: API, Headless CLI, Local
- Visual DAG workflows and time-travel debugging
- Sophisticated attach protocol for TUI connection

### Pi
- Multi-package TypeScript (pi-ai, pi-agent-core, pi-tui, pi-coding-agent)
- Single-process CLI application
- API mode only (multi-provider with context handoff)
- Scrollback-based TUI with differential rendering
- Minimal tools and prompts, maximum observability

## Related Research

- Pi blog post: https://marioslab.io/posts/pi/building-a-coding-agent/
- Pi repository: github.com/badlogic/pi-mono
- Agent tools collection: github.com/badlogic/agent-tools
- mcporter MCP->CLI: github.com/peterstellengerger/mcporter
- Terminal-Bench 2.0: (benchmark referenced in blog)

## Open Questions

1. **Observability vs Orchestration**: Can Descartes provide Pi-level observability while maintaining orchestration capabilities?
2. **Context efficiency**: How much context is Descartes consuming on typical sessions vs Pi?
3. **Benchmark comparison**: How would Descartes perform on Terminal-Bench 2.0?
4. **User segmentation**: Which users need orchestration vs prefer minimalism?
