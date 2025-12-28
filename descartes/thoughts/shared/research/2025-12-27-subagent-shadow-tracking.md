# Sub-Agent Shadow Tracking Research

**Date**: 2025-12-27
**Status**: Implemented
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
