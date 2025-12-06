---
date: 2025-12-06
author: Claude
status: complete
last_updated: 2025-12-06
epic: Descartes CL Workflow Integration
---

# Implementation Plan: CL Workflow Support in Descartes

## Overview

Add lightweight support for structured development workflows (`/cl:*` style) to Descartes using Option C: Lightweight ToolLevel Extension approach.

## Design Decisions

Based on user feedback:
- **Agent definitions**: Markdown files with frontmatter (like existing `.claude/agents/cl/*.md`)
- **Thoughts storage**: Use existing `ThoughtsStorage` at `~/.descartes/thoughts/` (consolidate with `thoughts/shared/` pattern)
- **Implementation approach**: Extend `ToolLevel` enum with new specialized levels

## Architecture

```
~/.descartes/
├── thoughts/              # Existing ThoughtsStorage (research, plans, etc.)
│   ├── research/          # NEW: Research output documents
│   ├── plans/             # NEW: Implementation plans
│   └── ...
├── agents/                # NEW: Agent definition files
│   ├── codebase-locator.md
│   ├── codebase-analyzer.md
│   └── ...
└── config/                # Existing config
```

## Phase 1: Extend ThoughtsStorage for Research/Plans

### 1.1 Add subdirectory support to ThoughtsStorage

**File**: `descartes/core/src/thoughts.rs`

Add constants and methods for structured output directories:

- [x] Add `RESEARCH_DIR = "research"` and `PLANS_DIR = "plans"` constants
- [x] Add `save_research(&self, filename: &str, content: &str) -> ThoughtsResult<PathBuf>`
- [x] Add `save_plan(&self, filename: &str, content: &str) -> ThoughtsResult<PathBuf>`
- [x] Add `list_research() -> ThoughtsResult<Vec<PathBuf>>`
- [x] Add `list_plans() -> ThoughtsResult<Vec<PathBuf>>`
- [x] Ensure these directories are created in `initialize()`

### 1.2 Add markdown frontmatter parsing

**File**: `descartes/core/src/thoughts.rs` (or new `thoughts_markdown.rs`)

- [x] Add struct `MarkdownDocument { frontmatter: HashMap<String, String>, content: String }`
- [x] Add `parse_markdown_with_frontmatter(content: &str) -> ThoughtsResult<MarkdownDocument>`
- [x] Support YAML frontmatter (`---\nkey: value\n---`)

**Verification**: `cargo test -p descartes-core --lib thoughts` - 17 tests passed

---

## Phase 2: Add Agent Definition Loader

### 2.1 Create AgentDefinition struct

**File**: `descartes/core/src/agent_definitions.rs` (NEW)

```rust
/// Agent definition loaded from markdown file
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub tool_level: ToolLevel,
    pub tags: Vec<String>,
    pub system_prompt: String,
}
```

- [x] Create `AgentDefinition` struct with frontmatter fields
- [x] Add `AgentDefinitionLoader` to load from `~/.descartes/agents/`
- [x] Add `load_agent(name: &str) -> Result<AgentDefinition>`
- [x] Add `list_agents() -> Result<Vec<String>>`

### 2.2 Add new ToolLevel variants

**File**: `descartes/core/src/tools/registry.rs`

- [x] Add `ToolLevel::Researcher` - read-only but with focused research prompt
- [x] Add `ToolLevel::Planner` - read + write to thoughts only
- [x] Update `get_tools()` for new levels
- [x] Add `researcher_system_prompt()`
- [x] Add `planner_system_prompt()`

### 2.3 Wire up agent loader in lib.rs

**File**: `descartes/core/src/lib.rs`

- [x] Add `pub mod agent_definitions;`
- [x] Re-export `AgentDefinition`, `AgentDefinitionLoader`

**Verification**: `cargo test -p descartes-core --lib agent_definitions` - 8 tests passed, `cargo test -p descartes-core --lib tools::registry` - 11 tests passed

---

## Phase 3: Create Default Agent Definitions

### 3.1 Create agent markdown files

**Directory**: `descartes/agents/` (shipped with descartes, copied to ~/.descartes/agents/ on first run)

- [x] `codebase-locator.md` - Find WHERE files live (ToolLevel::ReadOnly)
- [x] `codebase-analyzer.md` - Understand HOW code works (ToolLevel::ReadOnly)
- [x] `codebase-pattern-finder.md` - Find existing patterns (ToolLevel::ReadOnly)
- [x] `researcher.md` - General research agent (ToolLevel::Researcher)
- [x] `planner.md` - Planning agent (ToolLevel::Planner)

Each file format:
```markdown
---
name: codebase-locator
model: claude-3-sonnet
tool_level: readonly
tags: [research, codebase]
---

You are a codebase locator specialist...
[system prompt content]
```

### 3.2 Add default agent installation

**File**: `descartes/core/src/agent_definitions.rs`

- [x] Add `ensure_default_agents()` to copy bundled agents to `~/.descartes/agents/` if not present
- [x] Call from `AgentDefinitionLoader::new()`

**Verification**: Manual test - run descartes and check `~/.descartes/agents/` is populated

---

## Phase 4: Integrate with spawn_session

### 4.1 Allow spawn_session to use agent definitions

**File**: `descartes/core/src/tools/executors.rs`

- [x] Extend `execute_spawn_session` to accept `agent: Option<String>` parameter
- [x] If agent is specified, load `AgentDefinition` and use its system prompt + tool level
- [x] Pass loaded config via CLI args (--system, --tool-level)

### 4.2 Update spawn_session tool definition

**File**: `descartes/core/src/tools/definitions.rs`

- [x] Add `agent` parameter to `spawn_session_tool()`:
  ```rust
  properties.insert(
      "agent".to_string(),
      json!({
          "type": "string",
          "description": "Agent definition to use (e.g., 'codebase-locator')"
      }),
  );
  ```

**Verification**: `cargo test -p descartes-core tools` - 28 tests passed

---

## Phase 5: Add Workflow Commands (Optional Enhancement)

### 5.1 Create workflow command definitions

**File**: `descartes/core/src/workflow_commands.rs` (NEW)

- [x] Define `WorkflowCommand` struct with step sequence
- [x] Create `research_codebase` command that:
  1. Spawns `codebase-locator` agent
  2. Spawns `codebase-analyzer` agent in parallel
  3. Combines output to `~/.descartes/thoughts/research/`
- [x] Create `create_plan` command
- [x] Create `implement_plan` command

### 5.2 Expose commands via CLI/TUI

**File**: `descartes/cli/src/commands/workflow.rs` (NEW)

- [x] Add `workflow list` command handler
- [x] Add `workflow run <name>` command handler
- [x] Add `workflow info <name>` command handler

**Verification**: `cargo test -p descartes-core --lib workflow_commands` - 5 tests passed

---

## Success Criteria

1. **Phase 1**: `ThoughtsStorage` can save/load research and plans with frontmatter
2. **Phase 2**: `AgentDefinitionLoader` loads agent configs from `~/.descartes/agents/`
3. **Phase 3**: Default agents are installed on first run
4. **Phase 4**: `spawn_session` can spawn agents by name with correct prompts
5. **Phase 5** (optional): `/cl:*` style commands work in CLI/TUI

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing tool registry | Low | High | Add new variants, don't modify existing |
| Agent file format changes | Medium | Medium | Use versioned frontmatter with defaults |
| ThoughtsStorage path conflicts | Low | Medium | Use separate subdirectories |

## Open Questions

1. Should agent definitions support inheritance/composition?
2. Should workflow commands be TOML-defined or code-defined?
3. Integration with existing swarm TOML configs?

---

## Appendix: File Changes Summary

### New Files
- `descartes/core/src/agent_definitions.rs`
- `descartes/core/src/workflow_commands.rs` (Phase 5)
- `descartes/agents/*.md` (5 agent definitions)

### Modified Files
- `descartes/core/src/lib.rs` - add module exports
- `descartes/core/src/thoughts.rs` - add research/plans subdirs
- `descartes/core/src/tools/registry.rs` - add ToolLevel variants
- `descartes/core/src/tools/definitions.rs` - add agent param to spawn_session
- `descartes/core/src/tools/executor.rs` - handle agent loading
