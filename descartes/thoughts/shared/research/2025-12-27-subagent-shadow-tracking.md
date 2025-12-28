# Sub-Agent Shadow Tracking Research

**Date**: 2025-12-27
**Status**: Implemented (Claude Code), In Progress (OpenCode)
**Related Files**:
- `core/src/intercepting_backend.rs` (new)
- `core/src/cli_backend.rs` (StreamChunk::SubAgentSpawned)

## Problem Statement

Descartes needs visibility into sub-agents spawned by Claude Code's Task tool to:
1. Build a DAG visualization of agent hierarchy
2. Track which agents spawned which sub-agents
3. Enable future "attach" functionality to any agent in the tree

## Approaches Considered

### 1. MCP Server (Rejected by User)
Provide a `spawn_agent` tool via MCP that Descartes controls. Claude would call our tool instead of Task.

**Pros**: Clean interception, full control
**Cons**: User preference ("I hate MCP")

### 2. Tool Interception via Pseudo-Tool (Experimental)
Disable all tools and describe a pseudo-tool in system prompt. Parse Claude's text output for tool patterns.

**Pros**: No external dependencies
**Cons**: Unreliable parsing, can't disable MCP tools

### 3. Shadow-Tracking (Implemented)
Let Claude Code spawn sub-agents normally via Task tool, but parse the stream-json output to detect when sub-agents are created.

**Pros**: Non-invasive, works with existing Claude Code behavior
**Cons**: Read-only (can't intercept/modify sub-agent behavior)

## Key Discoveries

### Stream-JSON Format

Claude Code's `--output-format stream-json` emits NDJSON with these message types:

```json
// System init
{"type":"system","subtype":"init","session_id":"uuid","tools":[...]}

// Assistant message with tool use
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Task","id":"toolu_xxx","input":{...}}]}}

// Task tool completion (KEY FOR SHADOW-TRACKING)
{"type":"user","message":{"content":[{"tool_use_id":"toolu_xxx","type":"tool_result",...}]},
 "tool_use_result":{"status":"completed","prompt":"...","agentId":"a9a57a7",...}}
```

### agentId Format

- **Short ID**: 7-character hex string (e.g., `a9a57a7`)
- **Returned in**: `tool_use_result.agentId` when Task tool completes
- **Session file**: `~/.claude/projects/.../agent-{agentId}.jsonl`

### Session File Structure

```json
{"parentUuid":null,"isSidechain":true,"userType":"external",
 "sessionId":"parent-uuid","agentId":"a9a57a7",
 "type":"user","message":{"role":"user","content":"..."}}
```

Key fields:
- `isSidechain: true` - Marks as sub-agent
- `sessionId` - Parent's session UUID (shared)
- `agentId` - Unique short ID for this sub-agent

### Resume Limitation

- `--resume {agentId}` does NOT work (expects full UUID)
- `--resume {sessionId}` resumes parent, not specific sub-agent
- No direct way to resume a specific sub-agent by agentId

## Implementation

### New StreamChunk Variant

```rust
SubAgentSpawned {
    agent_id: String,      // Short ID (e.g., "a9a57a7")
    session_id: String,    // Parent's session UUID
    prompt: String,        // Task given to sub-agent
    subagent_type: Option<String>, // e.g., "general-purpose", "Explore"
    parent_tool_id: String, // Tool use ID that spawned this
}
```

### Detection Logic

1. Track pending Task tool calls in a HashMap (tool_id -> input)
2. When `type: "user"` message arrives with `tool_use_result.agentId`:
   - Extract agentId, prompt from tool_use_result
   - Look up subagent_type from tracked Task call
   - Emit `SubAgentSpawned` event

### InterceptingClaudeBackend

New backend that extends stream-json parsing:
- Bidirectional communication via stdin/stdout
- Tool result injection capability (for future interception)
- Sub-agent spawn detection

## Usage

```rust
let backend = InterceptingClaudeBackend::new();
let handle = backend.start_session(config).await?;

while let Some(chunk) = handle.stream_rx.recv().await {
    match chunk {
        StreamChunk::SubAgentSpawned { agent_id, prompt, .. } => {
            // Add to DAG, update UI, etc.
            dag.add_node(agent_id, parent_id, prompt);
        }
        // ... handle other chunks
    }
}
```

## Future Work

1. **Attach Functionality**: Read agent session file to get conversation context, start new session with that context

2. **Sub-Agent Progress**: Monitor agent session file for updates (new messages appended)

3. **Recursive Tracking**: Detect sub-agents of sub-agents by parsing nested stream-json

4. **Agent Resume**: Investigate if there's an undocumented way to resume specific sub-agents

## Test Data

Example Task tool output that triggers detection:

```json
{
  "type": "user",
  "message": {
    "role": "user",
    "content": [{
      "tool_use_id": "toolu_014bmYNjTN754JKMTVXd9ijG",
      "type": "tool_result",
      "content": ["2 + 2 = 4"]
    }]
  },
  "tool_use_result": {
    "status": "completed",
    "prompt": "What is 2+2?",
    "agentId": "a9a57a7",
    "totalDurationMs": 2710,
    "totalTokens": 40348
  }
}
```

---

## OpenCode Sub-Agent Tracking

OpenCode (v1.0.201) also supports sub-agents with a different but more structured format.

### Available Agent Types

```
build (primary)
compaction (primary)
explore (subagent)    # For codebase exploration
general (subagent)    # General purpose sub-agent
plan (primary)
summary (primary)
title (primary)
context7 (all)
```

### JSON Output Format

Run with `--format json` to get structured output:

```json
{
  "type": "tool_use",
  "timestamp": 1766901159273,
  "sessionID": "ses_49c7c7eb8ffev6NZJAKSt5p48e",
  "part": {
    "tool": "task",
    "state": {
      "status": "completed",
      "input": {
        "description": "Search CLI files",
        "prompt": "Search the codebase for...",
        "subagent_type": "explore"
      },
      "output": "...",
      "metadata": {
        "sessionId": "ses_49c7c5e7bffeI3pI0nEWWAO4p9"
      }
    }
  }
}
```

### Session Export

OpenCode provides `opencode export <sessionID>` with rich metadata:

```json
{
  "info": {
    "id": "ses_49c7c5e7bffeI3pI0nEWWAO4p9",
    "parentID": "ses_49c7c7eb8ffev6NZJAKSt5p48e",  // Direct parent tracking!
    "title": "Search CLI files (@explore subagent)",
    "directory": "/path/to/project"
  },
  "messages": [{
    "info": {
      "agent": "explore",
      "model": {"providerID": "xai", "modelID": "grok-3-mini-latest"},
      "tools": {
        "task": false,  // Sub-agents can't spawn more sub-agents
        "edit": false,
        "write": false
      }
    }
  }]
}
```

### Key Differences from Claude Code

| Feature | Claude Code | OpenCode |
|---------|-------------|----------|
| Tool name | `Task` (capital) | `task` (lowercase) |
| Agent ID format | Short 7-char (`a9a57a7`) | Full session ID (`ses_...`) |
| Parent tracking | Must infer from tool_use_id | Explicit `parentID` field |
| Session storage | `~/.claude/projects/.../agent-{id}.jsonl` | SQLite DB, exportable |
| Sub-agent nesting | Allowed | Restricted (`task: false`) |
| Resume support | Limited (short ID doesn't work) | `--session <id>` with full ID |

### OpenCode Shadow-Tracking Implementation

For OpenCode, detection is simpler:

```rust
// In stream parsing
if part.tool == "task" && state.status == "completed" {
    let subagent_id = state.metadata.sessionId;
    let subagent_type = state.input.subagent_type;
    let prompt = state.input.prompt;

    emit(StreamChunk::SubAgentSpawned {
        agent_id: subagent_id,
        session_id: parent_session_id,  // From message sessionID
        prompt,
        subagent_type: Some(subagent_type),
        parent_tool_id: part.callID,
    });
}
```

### OpenCode Advantages

1. **Explicit parent tracking** - `parentID` field eliminates guesswork
2. **Full session IDs** - No short-ID-to-UUID mapping needed
3. **Tool restrictions visible** - Can see what sub-agents are allowed to do
4. **Export command** - `opencode export <id>` gives complete session data
5. **Resume works** - `opencode run --session <id>` continues any session
