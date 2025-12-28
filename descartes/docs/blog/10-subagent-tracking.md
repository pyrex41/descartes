# Sub-Agent Shadow Tracking

*Non-invasive monitoring of agent hierarchies*

---

When orchestrator agents delegate to sub-agents, visibility becomes critical. Descartes implements **shadow tracking**—a read-only detection system that monitors sub-agent spawning without intercepting or modifying behavior.

## The Challenge

Modern AI coding agents like Claude Code can spawn sub-agents for focused tasks:

```
Main Agent (claude)
├── Task Agent (explore codebase)
├── Task Agent (write tests)
└── Task Agent (update docs)
```

But how do you track this hierarchy when you don't control the underlying CLI?

## Shadow Tracking Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                     Descartes Session                          │
├───────────────────────────────────────────────────────────────┤
│                                                                │
│   ┌──────────────┐      Stream-JSON       ┌───────────────┐   │
│   │  Claude Code │ ────────────────────── │    Parser     │   │
│   │     CLI      │                        └───────┬───────┘   │
│   └──────────────┘                                │           │
│                                                   ▼           │
│                                          ┌───────────────┐    │
│                                          │ Sub-Agent     │    │
│                                          │ Detector      │    │
│                                          └───────┬───────┘    │
│                                                  │            │
│                         ┌────────────────────────┼───────┐    │
│                         ▼                        ▼       ▼    │
│                   ┌──────────┐           ┌──────────┐   ...   │
│                   │ Agent A  │           │ Agent B  │         │
│                   │ Tracking │           │ Tracking │         │
│                   └──────────┘           └──────────┘         │
│                                                                │
└───────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **Non-invasive** — Read-only observation
2. **No interception** — Cannot modify sub-agent behavior
3. **Stream parsing** — Analyze JSON output in real-time
4. **Hierarchical tracking** — Parent-child relationships preserved

---

## How Detection Works

### Claude Code Sub-Agents

When Claude Code's Task tool spawns a sub-agent, the output includes:

```json
{
  "parentUuid": null,
  "isSidechain": true,
  "userType": "external",
  "sessionId": "parent-session-uuid",
  "agentId": "a9a57a7",
  "type": "user",
  "message": {
    "role": "user",
    "content": [{
      "type": "tool_use_result",
      "agentId": "a9a57a7",
      "input": {
        "prompt": "Search the codebase for auth patterns",
        "subagent_type": "explore"
      }
    }]
  }
}
```

### Detection Flow

```
1. Track pending Task tool calls (tool_id → input)
   │
2. Monitor stream for "type": "user" messages
   │
3. Look for tool_use_result with agentId
   │
4. Extract: agentId, prompt, subagent_type
   │
5. Emit SubAgentSpawned event
```

### Stream Chunk

```rust
pub enum StreamChunk {
    // ... other variants ...

    SubAgentSpawned {
        agent_id: String,           // Short 7-char ID
        session_id: String,         // Parent's session UUID
        prompt: String,             // Task given to sub-agent
        subagent_type: Option<String>,  // e.g., "explore", "general-purpose"
        parent_tool_id: String,     // Tool use ID that spawned this
    }
}
```

---

## OpenCode Sub-Agents

OpenCode uses a different format with explicit parent tracking:

```json
{
  "type": "tool_use",
  "sessionID": "ses_49c7c7eb8ffev6NZJAKSt5p48e",
  "part": {
    "tool": "task",
    "state": {
      "status": "completed",
      "input": {
        "prompt": "Search the codebase for patterns",
        "subagent_type": "explore"
      },
      "metadata": {
        "sessionId": "ses_49c7c5e7bffeI3pI0nEWWAO4p9"
      }
    }
  }
}
```

### Key Differences

| Feature | Claude Code | OpenCode |
|---------|-------------|----------|
| Tool name | `Task` (capital) | `task` (lowercase) |
| Agent ID | Short 7-char | Full session ID |
| Parent tracking | Inferred | Explicit `parentID` |
| Storage | JSONL files | SQLite + export |
| Resume support | Limited | Full session ID |

---

## Tracking Data Structure

### SubAgentInfo

```rust
pub struct SubAgentInfo {
    /// Unique agent identifier
    pub agent_id: String,

    /// Type of sub-agent (explore, general-purpose, etc.)
    pub agent_type: String,

    /// Task/prompt given to the agent
    pub prompt: String,

    /// Parent tool call ID
    pub parent_tool_id: String,

    /// When the sub-agent was spawned
    pub spawned_at: DateTime<Utc>,

    /// Current status (running, completed, failed)
    pub status: AgentStatus,

    /// Parent session ID
    pub parent_session_id: String,
}
```

### Agent Hierarchy

```rust
pub struct AgentHierarchy {
    /// Root agent (main session)
    pub root: AgentNode,

    /// All agents indexed by ID
    pub agents: HashMap<String, AgentNode>,
}

pub struct AgentNode {
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub info: SubAgentInfo,
}
```

---

## What Can Be Tracked

### Available Information

| Data | Source | Reliability |
|------|--------|-------------|
| Agent ID | Stream output | High |
| Prompt/Task | Tool input | High |
| Agent type | Tool input | High |
| Parent tool ID | Stream context | High |
| Spawn timestamp | Observation time | High |
| Completion status | Stream output | Medium |
| Sub-agent output | Stream output | Low* |

*Sub-agent output may be summarized by the parent.

### Not Available

| Data | Reason |
|------|--------|
| Internal thinking | Not exposed in stream |
| Tool call details | Sub-agent's session private |
| Full transcript | Stored in sub-agent's session |
| Error details | Often not propagated |

---

## Viewing Sub-Agents

### CLI

```bash
# List all agents including sub-agents
descartes ps --tree

# Output:
# a1b2c3 (running) — Main: Implement auth system
# ├── d4e5f6 (completed) — Explore: Find auth patterns
# ├── g7h8i9 (running) — Explore: Check test coverage
# └── j0k1l2 (pending) — General: Write documentation
```

### GUI

The Chat View displays sub-agents inline:

```
┌─ Sub-Agent: explore-abc ───────────────────────────────────┐
│ Type: Explore                                               │
│ Task: "Search for JWT implementation patterns"             │
│ Status: Running ●                                          │
│ Spawned: 30 seconds ago                                    │
└─────────────────────────────────────────────────────────────┘
```

### JSON Output

```bash
descartes logs a1b2c3 --format json | jq '.sub_agents'
```

```json
[
  {
    "agent_id": "d4e5f6",
    "agent_type": "explore",
    "prompt": "Find auth patterns in the codebase",
    "parent_tool_id": "call_abc123",
    "spawned_at": "2025-01-15T10:30:00Z",
    "status": "completed"
  }
]
```

---

## The One-Level Rule

Descartes enforces a strict hierarchy:

```
Orchestrator (can spawn)
└── Sub-Agent (cannot spawn further)
```

### Why This Matters

Without limits, recursive spawning could:
- **Exhaust resources** — Unbounded agent creation
- **Lose visibility** — Deep hierarchies become opaque
- **Increase costs** — Token multiplication
- **Create loops** — Circular delegation

### Enforcement

When a sub-agent tries to spawn:

```rust
if session.is_sub_session() {
    return Err("Sub-sessions cannot spawn further agents");
}
```

---

## Practical Examples

### Example 1: Feature Implementation

```
Main: "Implement user authentication"
├── Explore: "Find existing auth patterns" (completed)
├── Explore: "Check security best practices" (completed)
├── Minimal: "Implement JWT tokens" (running)
└── Minimal: "Add login endpoint" (pending)
```

### Example 2: Code Review

```
Main: "Review the payment module"
├── Explore: "Analyze code structure" (completed)
├── Explore: "Find similar patterns" (completed)
└── Researcher: "Check for vulnerabilities" (running)
```

### Example 3: Documentation

```
Main: "Document the API"
├── Explore: "List all endpoints" (completed)
├── Explore: "Find existing docs" (completed)
└── Minimal: "Write OpenAPI spec" (running)
```

---

## Integration with Workflows

### Flow Workflow

During the Implement phase, sub-agents are tracked:

```json
// .scud/flow-state.json
{
  "phases": {
    "implement": {
      "status": "in_progress",
      "wave": 2,
      "sub_agents": [
        {"id": "abc", "task": "TASK-002", "status": "running"},
        {"id": "def", "task": "TASK-003", "status": "completed"}
      ]
    }
  }
}
```

### Session Transcripts

Sub-agent events are recorded:

```json
{
  "entries": [
    {
      "role": "assistant",
      "tool_calls": [{
        "name": "spawn_session",
        "arguments": {"task": "Write tests"}
      }]
    },
    {
      "type": "sub_agent_spawned",
      "agent_id": "abc123",
      "task": "Write tests",
      "timestamp": "..."
    },
    {
      "type": "sub_agent_completed",
      "agent_id": "abc123",
      "result": "Tests written successfully",
      "timestamp": "..."
    }
  ]
}
```

---

## Monitoring Best Practices

### 1. Use Tree View

Always check the hierarchy:

```bash
descartes ps --tree
```

### 2. Follow Sub-Agent Logs

Track specific sub-agents:

```bash
descartes logs abc123 --follow
```

### 3. Set Alerts for Failures

Monitor for sub-agent errors:

```bash
descartes logs a1b2c3 --format json | \
  jq 'select(.type == "sub_agent_failed")'
```

### 4. Review After Completion

Check the full hierarchy:

```bash
descartes logs a1b2c3 --format json | \
  jq '.sub_agents | map({id, status, task})'
```

---

## Limitations

### What Shadow Tracking Cannot Do

1. **Intercept sub-agent calls** — Read-only observation
2. **Modify sub-agent behavior** — Cannot inject prompts
3. **Access sub-agent transcripts** — Stored separately
4. **Prevent sub-agent spawning** — Only at Descartes level
5. **Real-time sub-agent streaming** — Only status updates

### Future Possibilities

With cooperation from CLI tools:
- Full transcript access
- Real-time output streaming
- Bidirectional control
- Cross-session context sharing

---

## Troubleshooting

### Sub-Agent Not Detected

Check the stream format:
```bash
descartes logs a1b2c3 --format json | \
  grep -i "tool_use_result"
```

### Missing Agent IDs

Some sub-agent types don't expose IDs clearly. Check the raw stream:
```bash
descartes logs a1b2c3 --format json | \
  jq '.entries[] | select(.type == "stream_chunk")'
```

### Incorrect Hierarchy

Verify parent-child relationships:
```bash
descartes ps --tree --all
```

---

## Next Steps

- **[Advanced Features →](11-advanced-features.md)** — Time-travel and restoration
- **[Flow Workflow →](07-flow-workflow.md)** — See sub-agents in action
- **[GUI Features →](09-gui-features.md)** — Visual hierarchy display

---

*Visibility without invasion—shadow tracking gives you insight without control.*
