# Session Management

*Control, observe, and manage your agent sessions*

---

Every Descartes agent runs within a **session**—a tracked execution context with its own transcript, state, and lifecycle. Understanding sessions is key to effective agent management.

## What is a Session?

A session represents a single agent execution, including:

- **Unique ID** — UUID for identification
- **Task** — The prompt/goal given to the agent
- **Transcript** — Complete conversation history
- **Status** — Current lifecycle state
- **Metadata** — Timestamps, provider info, parent session

```json
{
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "name": "auth-implementation",
  "status": "running",
  "task": "Implement JWT authentication",
  "provider": "anthropic",
  "model": "claude-3-5-sonnet",
  "created_at": "2025-01-15T10:30:00Z",
  "parent_session_id": null
}
```

---

## Session Lifecycle

Sessions track the daemon/workspace state, while agents within sessions have their own status.

### SessionStatus Values

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Inactive │────▶│ Starting │────▶│  Active  │
└──────────┘     └──────────┘     └────┬─────┘
                                       │
                      ┌────────────────┼────────────────┐
                      ▼                ▼                ▼
                ┌──────────┐    ┌──────────┐     ┌──────────┐
                │ Stopping │    │ Archived │     │  Error   │
                └──────────┘    └──────────┘     └──────────┘
```

| Status | Description |
|--------|-------------|
| **Inactive** | Session exists but daemon not running |
| **Starting** | Daemon is starting up |
| **Active** | Daemon is running and connected |
| **Stopping** | Daemon is stopping |
| **Archived** | Session has been archived |
| **Error** | Session has errors |

### AgentStatus Values

When monitoring individual agents within a session, agents have their own status:

| Status | Description |
|--------|-------------|
| **Idle** | Agent created but not started |
| **Initializing** | Agent loading context and environment |
| **Running** | Agent actively executing tasks |
| **Thinking** | Agent processing/generating response |
| **Paused** | Agent suspended, can be resumed |
| **Completed** | Agent finished successfully |
| **Failed** | Agent encountered unrecoverable error |
| **Terminated** | Agent was manually killed |

> **Note:** SessionStatus tracks the daemon/workspace state, while AgentStatus tracks individual agent lifecycle within that session.

---

## Creating Sessions

### Via CLI

```bash
# Simple spawn
descartes spawn --task "Fix the login bug"

# With custom name
descartes spawn --task "Add OAuth" --name oauth-feature

# Stream output
descartes spawn --task "Refactor tests" --stream
```

### Programmatic (Rust)

```rust
use descartes_core::{Session, SessionManager};

let session = Session::new()
    .name("my-session")
    .task("Implement feature X")
    .provider("anthropic")
    .spawn()
    .await?;
```

---

## Monitoring Sessions

### List All Sessions

```bash
descartes ps
```

Output:
```
ID       STATUS    TASK                              STARTED      PROVIDER
a1b2c3   running   Implement JWT authentication      2 min ago    anthropic
d4e5f6   paused    Review security practices         15 min ago   openai
g7h8i9   thinking  Add payment integration           30 sec ago   anthropic
j0k1l2   completed Fix login bug                     1 hour ago   anthropic
```

### Include Historical Sessions

```bash
descartes ps --all
```

### JSON Output

```bash
descartes ps --format json | jq '.[] | {id, status, task}'
```

---

## Session Transcripts

Every session creates a detailed JSON transcript.

### Location

Session transcripts are stored in project-local directories. Descartes uses a **shared directory model** with SCUD for convenience:

```
.scud/sessions/           # Primary location (when .scud/ exists)
└── 2025-01-15-10-30-00-a1b2c3.json

~/.descartes/sessions/    # Fallback (when no .scud/ directory)
└── 2025-01-15-10-30-00-d4e5f6.json
```

**Location logic:** If a `.scud/` directory exists in your project (indicating SCUD task management is in use), Descartes writes session transcripts to `.scud/sessions/`. Otherwise, it falls back to `~/.descartes/sessions/` in your home directory.

> **Note:** This means Descartes writes its own files into the `.scud/` directory when present. See [Session Storage](#session-storage) below for details on which files belong to which system.

### Transcript Structure

```json
{
  "session_id": "a1b2c3...",
  "started_at": "2025-01-15T10:30:00Z",
  "ended_at": "2025-01-15T10:45:00Z",
  "provider": "anthropic",
  "model": "claude-3-5-sonnet",
  "task": "Fix the login bug",
  "tool_level": "orchestrator",
  "parent_session_id": null,
  "entries": [
    {
      "role": "user",
      "content": "Fix the login bug",
      "timestamp": "2025-01-15T10:30:00Z"
    },
    {
      "role": "assistant",
      "content": "I'll analyze the login functionality...",
      "tool_calls": [
        {
          "id": "call_123",
          "name": "read",
          "arguments": {"path": "src/auth/login.ts"}
        }
      ],
      "timestamp": "2025-01-15T10:30:05Z"
    },
    {
      "role": "tool_result",
      "tool_call_id": "call_123",
      "content": "// login.ts contents...",
      "timestamp": "2025-01-15T10:30:06Z"
    }
  ]
}
```

### View Transcripts

```bash
# Text format (default)
descartes logs a1b2c3

# JSON format
descartes logs a1b2c3 --format json

# Follow in real-time
descartes logs a1b2c3 --follow

# Show 20 entries
descartes logs a1b2c3 --limit 20
```

---

## Pause and Resume

### Why Pause?

- **Free resources** during long-running tasks
- **Attach external TUI** (Claude Code, OpenCode)
- **Review progress** before continuing
- **Hand off** to a different session

### Pausing

```bash
# Cooperative pause (graceful)
descartes pause a1b2c3

# Forced pause (SIGSTOP)
descartes pause a1b2c3 --force
```

**What Happens:**
1. Agent receives pause signal
2. Current operation completes
3. State saved to disk
4. Process suspended
5. Status changes to `paused`

### Resuming

```bash
# Simple resume
descartes resume a1b2c3
```

**What Happens:**
1. State restored from disk
2. Process resumed (SIGCONT)
3. Agent continues from pause point
4. Status changes to `running`

---

## External TUI Attachment

Attach external terminal UIs to paused agents.

### Attach Protocol

```
┌──────────────┐     pause     ┌──────────────┐
│   Agent      │──────────────▶│   Paused     │
│  (running)   │               │   State      │
└──────────────┘               └──────┬───────┘
                                      │ attach
                                      ▼
                               ┌──────────────┐
                               │  External    │
                               │     TUI      │
                               └──────────────┘
```

### Supported TUIs

- **Claude Code** — Anthropic's official CLI
- **OpenCode** — Open-source alternative
- **Custom** — Any compatible TUI

### Attaching

```bash
# Pause first
descartes pause a1b2c3

# Attach Claude Code
descartes attach a1b2c3 --client claude-code

# Attach OpenCode and launch it
descartes attach a1b2c3 --client opencode --launch

# Get credentials as JSON for scripting
descartes attach a1b2c3 --json
```

### Credentials

Attachment uses time-limited tokens:

```json
{
  "attach_token": "tok_abc123...",
  "expires_at": "2025-01-15T10:35:00Z",
  "websocket_url": "ws://localhost:8080/attach/a1b2c3",
  "session_id": "a1b2c3..."
}
```

**Token Lifetime:** 5 minutes (configurable)

---

## Sub-Sessions

Orchestrator agents can spawn sub-sessions for focused tasks.

### Hierarchy

```
Main Session (orchestrator)
├── Sub-Session 1 (minimal) — "Write tests"
├── Sub-Session 2 (minimal) — "Update docs"
└── Sub-Session 3 (readonly) — "Review changes"
```

### Sub-Session Rules

1. **No recursive spawning** — Sub-sessions cannot spawn further
2. **Automatic downgrade** — Orchestrator → Minimal for children
3. **Parent tracking** — `parent_session_id` links to parent
4. **Independent transcripts** — Each has its own JSON file

### Viewing Hierarchy

```bash
descartes ps --tree

# Output:
# a1b2c3 (running) — Main implementation
# ├── d4e5f6 (completed) — Write tests
# ├── g7h8i9 (running) — Update docs
# └── j0k1l2 (completed) — Review changes
```

---

## Session Storage

### The Shared Directory Model

Descartes and SCUD share the `.scud/` directory for convenience. This keeps all project artifacts in one place for easy backup and version control. Here's what each system owns:

| File/Directory | Owner | Purpose |
|----------------|-------|---------|
| `.scud/active-tag` | SCUD | Current active epic/tag |
| `.scud/config.toml` | SCUD | SCUD configuration |
| `.scud/tasks/` | SCUD | Task definitions (SCG/JSON) |
| `.scud/workflow-state.json` | SCUD | SCUD workflow state |
| `.scud/sessions/` | **Descartes** | Session transcripts |
| `.scud/flow-state.json` | **Descartes** | Flow workflow state |
| `.scud/loop-state.json` | **Descartes** | Iterative loop state |
| `.scud/tune-state.json` | **Descartes** | Task tuning state |
| `.scud/qa-log.json` | **Descartes** | QA monitoring log |

> **Why share?** When using SCUD for task management with Descartes for execution, sharing a directory means one location to manage. You can use `scud` CLI for tasks and `descartes` CLI for agents, with all state colocated.

### Directory Structure

```
~/.descartes/
├── data/
│   ├── descartes.db        # SQLite database
│   ├── state/              # Agent state snapshots
│   └── events/             # Event logs
└── sessions/               # Global session fallback

.scud/ (per-project, shared)
├── active-tag              # [SCUD] Current tag
├── config.toml             # [SCUD] Configuration
├── tasks/                  # [SCUD] Task definitions
│   ├── tasks.scg
│   └── tasks.json
├── workflow-state.json     # [SCUD] Workflow state
├── sessions/               # [DESCARTES] Transcripts
│   ├── 2025-01-15-10-30-00-a1b2c3.json
│   └── 2025-01-15-11-00-00-d4e5f6.json
├── flow-state.json         # [DESCARTES] Flow state
├── loop-state.json         # [DESCARTES] Loop state
└── tune-state.json         # [DESCARTES] Tune state
```

### Session Discovery

Descartes finds sessions by scanning for:
- `.scud/` directories (primary, shared with SCUD)
- `.descartes/` directories (legacy/standalone)
- Session metadata files

```bash
# Scan for sessions in current directory
descartes ps

# Scan specific directory
descartes ps --dir /path/to/project
```

---

## Session Cleanup

### Terminate Sessions

```bash
# Graceful shutdown
descartes kill a1b2c3

# Force kill
descartes kill a1b2c3 --force
```

### Archive Sessions

```bash
# Mark as archived (preserve transcript)
descartes archive a1b2c3

# List archived
descartes ps --archived
```

### Delete Sessions

```bash
# Remove session and transcript
descartes delete a1b2c3

# Clean up old sessions
descartes cleanup --older-than 30d
```

---

## Session Recovery

### From Crash

If Descartes crashes, sessions can be recovered:

```bash
# List orphaned sessions
descartes recover --list

# Recover specific session
descartes recover a1b2c3

# Recover all
descartes recover --all
```

### From Transcript

Reconstruct session state from transcript:

```bash
descartes restore .scud/sessions/2025-01-15-10-30-00-a1b2c3.json
```

---

## Best Practices

### 1. Use Meaningful Names

```bash
descartes spawn --task "Add OAuth" --name oauth-feature
```

### 2. Stream for Long Tasks

```bash
descartes spawn --task "Major refactor" --stream
```

### 3. Pause Before Walking Away

```bash
# Starting a long task
descartes spawn --task "Implement payment system" --attachable

# Need to step away? Pause it.
descartes pause a1b2c3

# Come back later
descartes resume a1b2c3
```

### 4. Review Transcripts

After completion, review the transcript to understand what happened:

```bash
descartes logs a1b2c3 --format json | jq '.entries[] | select(.tool_calls)'
```

### 5. Clean Up Regularly

```bash
# Weekly cleanup
descartes cleanup --older-than 7d --status completed
```

---

## Troubleshooting

### "Session Not Found"

```bash
# Check session exists
descartes ps --all | grep a1b2c3

# Check correct directory
ls .scud/sessions/
```

### "Cannot Pause Running Session"

```bash
# Force pause
descartes pause a1b2c3 --force
```

### "Attach Timeout"

```bash
# Ensure session is paused first
descartes pause a1b2c3

# Then attach
descartes attach a1b2c3 --tui claude
```

---

## Next Steps

- **[Agent Types →](06-agent-types.md)** — Understand tool levels
- **[Flow Workflow →](07-flow-workflow.md)** — Multi-phase automation
- **[Sub-Agent Tracking →](10-subagent-tracking.md)** — Monitor sub-agents

---

*With session management mastered, you have full control over your AI workforce.*
