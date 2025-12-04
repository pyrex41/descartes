# CLI-by-Default with Pi-Style Spawning Implementation Plan

## Overview

Transform Descartes to be CLI-by-default with minimal tooling, following Pi's philosophy of observability and simplicity. Spawned sub-sessions cannot spawn their own sub-agents (recursive prevention), and MCP tools are wrapped as "skills" for progressive disclosure.

## Current State Analysis

### CLI Spawn Command (`cli/src/commands/spawn.rs`)
- Currently standalone: makes single LLM call, prints output, exits
- No daemon integration, no agent persistence
- No tool definitions passed to model

### Agent Runner (`core/src/agent_runner.rs`)
- Full lifecycle management exists for headless CLI backends
- Spawns claude/opencode as child processes with stdio piping
- Supports pause/resume/attach via signals and cooperative protocol

### Attach System (`daemon/src/attach_session.rs`, `claude_code_tui.rs`)
- Complete token-based authentication
- Unix socket communication with framed messages
- Historical output buffering for late-joining clients

### Tool Definitions (`core/src/traits.rs`)
- Schema exists: `Tool`, `ToolCall`, `ToolParameters`
- Not actively used in CLI spawns currently

## Desired End State

After implementation:

1. **Minimal Default Toolset**: Agents get 4 core tools: `read`, `write`, `edit`, `bash`
2. **Spawn Session Tool**: Orchestrator agents can spawn sub-sessions that stream output
3. **Recursive Prevention**: Sub-sessions cannot spawn their own sub-agents
4. **Transcript Saving**: All sessions save full transcripts to `.scud/sessions/`
5. **Skill Wrappers**: Pattern for wrapping MCP tools as CLI-invokable skills

### Verification
- `descartes spawn --task "hello"` works with minimal tools
- Spawned sub-sessions stream output to parent
- Sub-sessions cannot use `spawn_session` tool
- Transcripts saved to disk

## What We're NOT Doing

- **Not removing existing provider backends** - API mode still works
- **Not adding attach to CLI-spawned sessions** - if you want attach, use daemon-spawned agents
- **Not implementing MCP server** - steering away from MCP, using skills instead
- **Not modifying GUI** - CLI-focused changes only
- **Not changing agent_runner.rs core** - reusing existing LocalProcessRunner

---

## Phase 1: Minimal Tool Definitions

### Overview
Define the core minimal toolset (read, write, edit, bash) as reusable Tool schemas that can be passed to any model backend.

### Changes Required:

#### 1.1 Create Tools Module

**File**: `descartes/core/src/tools/mod.rs` (new)
**Changes**: Create module with minimal tool definitions

```rust
//! Minimal tool definitions for Descartes agents.
//!
//! Following Pi's philosophy: if you don't need it, don't build it.
//! These 4 tools are sufficient for effective coding agents.

mod definitions;
mod registry;

pub use definitions::*;
pub use registry::*;
```

#### 1.2 Tool Definitions

**File**: `descartes/core/src/tools/definitions.rs` (new)
**Changes**: Define the 4 core tools with JSON Schema parameters

```rust
use crate::traits::{Tool, ToolParameters};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Create the `read` tool definition.
/// Reads file contents. Supports text files and images.
pub fn read_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert("path".to_string(), json!({
        "type": "string",
        "description": "Path to the file to read (relative or absolute)"
    }));
    properties.insert("offset".to_string(), json!({
        "type": "integer",
        "description": "Line number to start reading from (1-indexed, optional)"
    }));
    properties.insert("limit".to_string(), json!({
        "type": "integer",
        "description": "Maximum number of lines to read (optional)"
    }));

    Tool {
        name: "read".to_string(),
        description: "Read the contents of a file. Supports text files and images (jpg, png, gif, webp). For text files, defaults to first 2000 lines. Use offset/limit for large files.".to_string(),
        parameters: ToolParameters {
            required: vec!["path".to_string()],
            properties,
        },
    }
}

/// Create the `write` tool definition.
/// Writes content to a file, creating directories as needed.
pub fn write_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert("path".to_string(), json!({
        "type": "string",
        "description": "Path to the file to write (relative or absolute)"
    }));
    properties.insert("content".to_string(), json!({
        "type": "string",
        "description": "Content to write to the file"
    }));

    Tool {
        name: "write".to_string(),
        description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories.".to_string(),
        parameters: ToolParameters {
            required: vec!["path".to_string(), "content".to_string()],
            properties,
        },
    }
}

/// Create the `edit` tool definition.
/// Makes surgical edits by replacing exact text.
pub fn edit_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert("path".to_string(), json!({
        "type": "string",
        "description": "Path to the file to edit (relative or absolute)"
    }));
    properties.insert("old_text".to_string(), json!({
        "type": "string",
        "description": "Exact text to find and replace (must match exactly including whitespace)"
    }));
    properties.insert("new_text".to_string(), json!({
        "type": "string",
        "description": "New text to replace the old text with"
    }));

    Tool {
        name: "edit".to_string(),
        description: "Edit a file by replacing exact text. The old_text must match exactly (including whitespace). Use this for precise, surgical edits.".to_string(),
        parameters: ToolParameters {
            required: vec!["path".to_string(), "old_text".to_string(), "new_text".to_string()],
            properties,
        },
    }
}

/// Create the `bash` tool definition.
/// Executes bash commands in the working directory.
pub fn bash_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert("command".to_string(), json!({
        "type": "string",
        "description": "Bash command to execute"
    }));
    properties.insert("timeout".to_string(), json!({
        "type": "integer",
        "description": "Timeout in seconds (optional, no default timeout)"
    }));

    Tool {
        name: "bash".to_string(),
        description: "Execute a bash command in the current working directory. Returns stdout and stderr. Use for git, npm, make, and other CLI operations.".to_string(),
        parameters: ToolParameters {
            required: vec!["command".to_string()],
            properties,
        },
    }
}

/// Create the `spawn_session` tool definition.
/// Only available to orchestrator agents, NOT to spawned sub-sessions.
pub fn spawn_session_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert("task".to_string(), json!({
        "type": "string",
        "description": "The task/prompt to give to the spawned session"
    }));
    properties.insert("provider".to_string(), json!({
        "type": "string",
        "description": "Provider to use: 'claude', 'opencode', 'anthropic', 'openai', 'ollama'",
        "default": "claude"
    }));
    properties.insert("output_file".to_string(), json!({
        "type": "string",
        "description": "Optional path to save the session transcript"
    }));
    properties.insert("attachable".to_string(), json!({
        "type": "boolean",
        "description": "If true, creates an attach socket for TUI connection",
        "default": false
    }));

    Tool {
        name: "spawn_session".to_string(),
        description: "Spawn a sub-session to handle a specific task. The sub-session's output streams to this session. Sub-sessions cannot spawn their own sub-sessions (no recursive agents). Use for code review, research, or delegating focused tasks.".to_string(),
        parameters: ToolParameters {
            required: vec!["task".to_string()],
            properties,
        },
    }
}
```

#### 1.3 Tool Registry

**File**: `descartes/core/src/tools/registry.rs` (new)
**Changes**: Registry for getting tool sets by capability level

```rust
use crate::traits::Tool;
use super::definitions::*;

/// Tool capability levels for agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLevel {
    /// Minimal tools: read, write, edit, bash
    /// Used for sub-sessions that cannot spawn further agents
    Minimal,
    /// Orchestrator tools: minimal + spawn_session
    /// Used for top-level agents that can delegate work
    Orchestrator,
    /// Read-only tools: read, bash (with restrictions)
    /// Used for exploration/planning without modifications
    ReadOnly,
}

/// Get the tools for a given capability level.
pub fn get_tools(level: ToolLevel) -> Vec<Tool> {
    match level {
        ToolLevel::Minimal => vec![
            read_tool(),
            write_tool(),
            edit_tool(),
            bash_tool(),
        ],
        ToolLevel::Orchestrator => vec![
            read_tool(),
            write_tool(),
            edit_tool(),
            bash_tool(),
            spawn_session_tool(),
        ],
        ToolLevel::ReadOnly => vec![
            read_tool(),
            bash_tool(), // For ls, grep, find, git status, etc.
        ],
    }
}

/// Get minimal system prompt for coding agents.
/// Pi-style: ~200 tokens, not 10,000.
pub fn minimal_system_prompt() -> &'static str {
    r#"You are an expert coding assistant. You help users with coding tasks by reading files, executing commands, editing code, and writing new files.

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
- Show file paths clearly when working with files"#
}

/// Get orchestrator system prompt (includes spawn_session).
pub fn orchestrator_system_prompt() -> &'static str {
    r#"You are an expert coding assistant with the ability to delegate tasks to sub-sessions.

Available tools:
- read: Read file contents
- bash: Execute bash commands
- edit: Make surgical edits to files
- write: Create or overwrite files
- spawn_session: Spawn a sub-session for focused tasks

Guidelines:
- Use bash for file operations like ls, grep, find
- Use read to examine files before editing
- Use edit for precise changes (old text must match exactly)
- Use write only for new files or complete rewrites
- Use spawn_session for code review, research, or focused sub-tasks
- Sub-sessions stream their output to you and save transcripts
- Be concise in your responses"#
}
```

#### 1.4 Export from Core Lib

**File**: `descartes/core/src/lib.rs`
**Changes**: Add tools module export

Add after other module declarations:
```rust
pub mod tools;
```

### Success Criteria:

#### Automated Verification:
- [x] Code compiles: `cargo build -p descartes-core`
- [x] Tests pass: `cargo test -p descartes-core --lib tools` (12 tests)
- [x] No clippy warnings: `cargo clippy -p descartes-core`

#### Manual Verification:
- [x] Review tool definitions match Pi's minimal approach
- [x] Confirm spawn_session description explains recursive prevention

---

## Phase 2: CLI Spawn with Tools and Transcript Saving

### Overview
Modify the CLI spawn command to use the minimal toolset and save session transcripts.

### Changes Required:

#### 2.1 Add Transcript Directory to Config

**File**: `descartes/core/src/config.rs`
**Changes**: Add sessions directory path to storage config

Find the storage config struct and add:
```rust
/// Directory for session transcripts
pub sessions_dir: PathBuf,
```

With default: `.scud/sessions/`

#### 2.2 Create Session Transcript Writer

**File**: `descartes/core/src/session_transcript.rs` (new)
**Changes**: Utility for writing session transcripts

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use uuid::Uuid;

/// A session transcript entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String, // "user", "assistant", "tool_call", "tool_result"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
}

/// Session transcript metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMetadata {
    pub session_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub task: String,
    pub parent_session_id: Option<Uuid>,
    pub is_sub_session: bool,
}

/// Writer for session transcripts.
pub struct TranscriptWriter {
    path: PathBuf,
    metadata: TranscriptMetadata,
    entries: Vec<TranscriptEntry>,
}

impl TranscriptWriter {
    /// Create a new transcript writer.
    pub fn new(
        sessions_dir: &PathBuf,
        provider: &str,
        model: &str,
        task: &str,
        parent_session_id: Option<Uuid>,
    ) -> std::io::Result<Self> {
        let session_id = Uuid::new_v4();
        let started_at = Utc::now();
        let is_sub_session = parent_session_id.is_some();

        // Create sessions directory if needed
        fs::create_dir_all(sessions_dir)?;

        // Generate filename: YYYY-MM-DD-HH-MM-SS-{short_id}.json
        let filename = format!(
            "{}-{}.json",
            started_at.format("%Y-%m-%d-%H-%M-%S"),
            &session_id.to_string()[..8]
        );
        let path = sessions_dir.join(filename);

        let metadata = TranscriptMetadata {
            session_id,
            started_at,
            provider: provider.to_string(),
            model: model.to_string(),
            task: task.to_string(),
            parent_session_id,
            is_sub_session,
        };

        Ok(Self {
            path,
            metadata,
            entries: Vec::new(),
        })
    }

    /// Add an entry to the transcript.
    pub fn add_entry(&mut self, role: &str, content: &str, tool_name: Option<&str>, tool_id: Option<&str>) {
        self.entries.push(TranscriptEntry {
            timestamp: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            tool_name: tool_name.map(|s| s.to_string()),
            tool_id: tool_id.map(|s| s.to_string()),
        });
    }

    /// Save the transcript to disk.
    pub fn save(&self) -> std::io::Result<PathBuf> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);

        // Write as JSON with metadata and entries
        let output = serde_json::json!({
            "metadata": self.metadata,
            "entries": self.entries,
        });

        serde_json::to_writer_pretty(&mut writer, &output)?;
        writer.flush()?;

        Ok(self.path.clone())
    }

    /// Get the session ID.
    pub fn session_id(&self) -> Uuid {
        self.metadata.session_id
    }

    /// Get the transcript path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
```

#### 2.3 Export TranscriptWriter

**File**: `descartes/core/src/lib.rs`
**Changes**: Add module export

```rust
pub mod session_transcript;
```

#### 2.4 Update CLI Spawn to Use Tools

**File**: `descartes/cli/src/commands/spawn.rs`
**Changes**: Pass tools to model request, save transcripts

The key changes:
1. Import tools module and get appropriate tool set
2. Create TranscriptWriter at start
3. Pass tools in ModelRequest
4. Log assistant responses and tool calls to transcript
5. Save transcript on completion

### Success Criteria:

#### Automated Verification:
- [x] Code compiles: `cargo build -p descartes-cli`
- [x] Tests pass: `cargo test -p descartes-cli`
- [x] No clippy warnings: `cargo clippy -p descartes-cli`

#### Manual Verification:
- [ ] Run `descartes spawn --task "list files in current directory"` and verify transcript is saved
- [ ] Check transcript contains proper JSON structure with metadata and entries

---

## Phase 3: Spawn Session Implementation with Recursive Prevention

### Overview
Implement the `spawn_session` tool that spawns sub-sessions without the ability to spawn further sub-sessions.

### Changes Required:

#### 3.1 Add CLI Flag for Sub-Session Mode

**File**: `descartes/cli/src/main.rs`
**Changes**: Add `--no-spawn` flag to spawn command

```rust
/// Spawn a new agent
Spawn {
    /// The task for the agent
    #[arg(short, long)]
    task: String,

    /// Provider (anthropic, openai, ollama, claude, opencode)
    #[arg(short, long)]
    provider: Option<String>,

    /// Model override
    #[arg(short, long)]
    model: Option<String>,

    /// System prompt
    #[arg(short, long)]
    system: Option<String>,

    /// Enable streaming output
    #[arg(long, default_value = "true")]
    stream: bool,

    /// Disable spawn_session tool (for sub-sessions)
    #[arg(long, default_value = "false")]
    no_spawn: bool,

    /// Tool level: minimal, orchestrator, or readonly
    #[arg(long, default_value = "orchestrator")]
    tool_level: String,

    /// Custom transcript directory (default: .scud/sessions/)
    #[arg(long)]
    transcript_dir: Option<String>,
},
```

#### 3.2 Implement Tool Selection Based on Mode

**File**: `descartes/cli/src/commands/spawn.rs`
**Changes**: Select tool level based on `--no-spawn` flag

```rust
use descartes_core::tools::{get_tools, ToolLevel, minimal_system_prompt, orchestrator_system_prompt};

// In execute function:
let tool_level = if no_spawn {
    ToolLevel::Minimal
} else {
    ToolLevel::Orchestrator
};

let tools = get_tools(tool_level);
let default_system = if no_spawn {
    minimal_system_prompt()
} else {
    orchestrator_system_prompt()
};
```

#### 3.3 Implement spawn_session Tool Handler

**File**: `descartes/cli/src/commands/spawn.rs`
**Changes**: Handle spawn_session tool calls by spawning sub-process

When model calls `spawn_session`:
1. Extract task, provider, output_file, attachable from tool arguments
2. Spawn `descartes spawn --no-spawn --parent-session <current_id> --task <task>`
3. Stream stdout/stderr to current session's output
4. Capture full output for tool result
5. Return result to model

### Success Criteria:

#### Automated Verification:
- [x] Code compiles: `cargo build -p descartes-cli`
- [x] Tests pass: `cargo test -p descartes-cli`
- [x] `descartes spawn --help` shows new flags

#### Manual Verification:
- [ ] Run orchestrator spawn and ask it to spawn a sub-session
- [ ] Verify sub-session output streams to parent
- [ ] Verify sub-session does NOT have spawn_session tool
- [ ] Verify both transcripts are saved

---

## Phase 4: MCP-to-Skill Wrapper Pattern

### Overview
Create documentation and example patterns for wrapping MCP tools as CLI-invokable skills.

### Changes Required:

#### 5.1 Create Skills Documentation

**File**: `docs/SKILLS.md` (new)
**Changes**: Document the skills pattern

```markdown
# Descartes Skills

Skills are CLI tools that agents can invoke via bash, following Pi's philosophy
of progressive disclosure: only load tool definitions when actually needed.

## Why Skills Instead of MCP?

MCP servers inject all tool definitions into every session, consuming 7-15% of
context window before you start working. Skills are invoked via bash, so the
agent only pays the token cost when it actually uses the tool.

## Creating a Skill

### 1. Create a CLI tool

```bash
#!/bin/bash
# web-search - Search the web for information
# Usage: web-search --query "search terms" --max-results 5

curl -s "https://api.example.com/search?q=$2&limit=$4"
```

### 2. Create a README

The agent reads this when it needs to use the tool:

```markdown
# web-search

Search the web for current information.

## Usage
\`\`\`bash
web-search --query "your search" --max-results 5
\`\`\`

## Examples
- `web-search --query "rust async patterns 2024"`
- `web-search --query "latest npm version" --max-results 1`
```

### 3. Add to AGENTS.md

Tell the agent about available skills:

```markdown
## Available Skills

You have access to these CLI tools. Read their READMEs for usage:

- `web-search` - Web search (README: /path/to/web-search/README.md)
- `browser` - Browser automation (README: /path/to/browser/README.md)
```

## Migrating from MCP

If you have an MCP server you want to use:

1. Use `mcporter` to wrap it as a CLI tool
2. Create a README documenting the commands
3. Reference in AGENTS.md

Example with Playwright MCP:
```bash
# Instead of MCP tool injection
mcporter playwright navigate --url "https://example.com"
mcporter playwright screenshot --path "shot.png"
```
```

#### 5.2 Create Example Skill Wrapper

**File**: `examples/skills/web-search/` (new directory)
**Changes**: Example skill implementation

Create `examples/skills/web-search/web-search` (executable script)
Create `examples/skills/web-search/README.md` (usage docs)

### Success Criteria:

#### Automated Verification:
- [x] Documentation files exist and are valid markdown

#### Manual Verification:
- [ ] Read through SKILLS.md and verify it's clear
- [ ] Test example skill can be invoked via bash

---

## Testing Strategy

### Unit Tests:
- Tool definition serialization to JSON
- ToolLevel::Minimal vs Orchestrator tool sets
- TranscriptWriter creates valid JSON
- CLI flag parsing for --no-spawn, --tool-level

### Integration Tests:
- Spawn with tools and verify tool schemas in request
- Spawn sub-session and verify no spawn_session tool
- Transcript saving and loading

### Manual Testing Steps:
1. `descartes spawn --task "ls"` - basic spawn works
2. `descartes spawn --task "spawn a helper to review code"` - orchestrator spawns sub-session
3. Verify sub-session cannot spawn (check transcript for tools available)
4. Verify transcripts in `.scud/sessions/`

## Performance Considerations

- Tool definitions add ~500 tokens to context (minimal set)
- Transcript writes are buffered and async-friendly
- Sub-session spawning is just process fork (fast)

## Migration Notes

- Existing spawn commands continue to work (tools are additive)
- No breaking changes to daemon RPC API
- Sessions directory created on first use

## References

- Pi blog post: https://marioslab.io/posts/pi/building-a-coding-agent/
- HumanLayer 12-Factor Agents: https://github.com/humanlayer/12-factor-agents
- Research: `thoughts/shared/research/2025-12-03-descartes-vs-pi-comparison.md`
