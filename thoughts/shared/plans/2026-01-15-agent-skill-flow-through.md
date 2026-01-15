# Plan: Agent and Skill Flow-Through for Subagents

## Overview

This plan addresses the gap where Descartes subagents currently receive only a prompt and category, with no access to skills, guidance, or agent-specific context. We will implement an agent definition system using Anthropic's skill format, enabling subagents to receive rich context including available skills and specialized instructions.

## Current State Analysis

### What Exists Today

1. **SubagentRequest** (`src/harness/mod.rs:108-117`):
   ```rust
   pub struct SubagentRequest {
       pub category: String,    // Agent type (searcher, builder, etc.)
       pub prompt: String,      // Task description
       pub model: Option<String>, // Optional model override
   }
   ```

2. **spawn_subagent()** (`src/agent/subagent.rs:66-84`):
   - Gets `CategoryConfig` from `category.default_config()`
   - Creates `SessionConfig` with model, tools, `is_subagent: true`
   - **No skills, guidance, or custom context passed**

3. **Skills System** (`src/interactive/skills.rs`):
   - Skills exist but are only for interactive use
   - Not passed to subagents

4. **Harness Context Injection**:
   - Claude Code: `--append-system-prompt` flag available but unused
   - OpenCode: `--agent` flag and `.opencode/agent/*.md` supported
   - Codex: `SessionConfig.system_prompt` exists but unused in subagents

### Key Files

| File | Purpose |
|------|---------|
| `src/agent/subagent.rs` | Subagent spawning logic |
| `src/agent/category.rs` | Category definitions |
| `src/harness/mod.rs` | Harness trait, SubagentRequest |
| `src/harness/claude_code.rs` | Claude Code implementation |
| `src/harness/opencode.rs` | OpenCode implementation |
| `src/harness/codex.rs` | Codex implementation |
| `src/interactive/skills.rs` | Current skills system |
| `src/config.rs` | Configuration structures |

## Desired End State

1. **Agent Definitions**: Agents defined in `.descartes/agents/<name>/AGENT.md` using Anthropic's skill format
2. **Skill Discovery**: Subagents receive skill frontmatter (name + description) so they know what's available
3. **Context Injection**: Each harness injects agent context via its native mechanism
4. **Enable/Disable Control**: Users control which agents are available via config
5. **Runtime Selection**: Orchestrator can select specific agents by name

## Implementation Approach

Follow Anthropic's progressive disclosure model:
- **Level 1 (Metadata)**: Agent name + description always available for selection
- **Level 2 (Instructions)**: Full AGENT.md content loaded when agent is spawned
- **Level 3 (Resources)**: Additional files loaded as needed by the agent

## Phases

### Phase 1: Agent Definition System

**Goal**: Create data model for agents and file loading from `.descartes/agents/`

**Changes**:

- [x] Create `src/agent/definition.rs` - new file for Agent struct and loading
  ```rust
  pub struct AgentDefinition {
      pub name: String,           // From YAML frontmatter
      pub description: String,    // From YAML frontmatter
      pub category: String,       // Base category for defaults
      pub model: Option<String>,  // Override model
      pub tools: Option<Vec<String>>, // Override tools
      pub skills: Vec<String>,    // Available skill names
      pub instructions: String,   // Body of AGENT.md
      pub path: PathBuf,          // Directory path for resources
  }
  ```

- [x] Create `src/agent/registry.rs` changes (`src/agent/registry.rs:166-387`):
  - Add `AgentDefinitionRegistry` struct (in definition.rs)
  - Add `load_agents(path: &Path)` method
  - Add `get_by_name(name: &str) -> Option<&AgentDefinition>`
  - Add `list_enabled() -> Vec<&AgentDefinition>`

- [x] Add to `src/config.rs` (~line 40, in Config struct):
  ```rust
  pub struct AgentsConfig {
      /// Directory containing agent definitions
      pub directory: PathBuf,  // default: .descartes/agents
      /// Explicitly enabled agents (if set, only these are available)
      /// Use either 'enabled' OR 'disabled', not both
      pub enabled: Option<Vec<String>>,
      /// Explicitly disabled agents (if set, all others are available)
      pub disabled: Option<Vec<String>>,
  }
  ```

- [x] Create example agent file `.descartes/agents/code-analyzer/AGENT.md`:
  ```markdown
  ---
  name: code-analyzer
  description: Expert code analysis agent. Use for deep architectural review, security analysis, and understanding complex codebases.
  category: analyzer
  model: opus
  skills:
    - research
    - review
  ---

  # Code Analyzer

  You are an expert code analyst. Your role is to provide deep insights into codebases.

  ## Focus Areas
  - Architecture patterns and anti-patterns
  - Security vulnerabilities
  - Performance implications
  - Code quality and maintainability

  ## Available Skills
  You have access to the following skills. Invoke them when appropriate:
  {{skills_frontmatter}}

  ## Approach
  1. Start by understanding the high-level structure
  2. Identify patterns and conventions
  3. Note areas of concern
  4. Provide actionable recommendations
  ```

**Success Criteria - Automated**:
- [x] `cargo check` passes in descartes/descartes
- [x] `cargo test agent::definition` passes
- [x] New unit tests for YAML frontmatter parsing pass

**Success Criteria - Manual**:
- [x] Agent files in `.descartes/agents/` are discovered and loaded
- [x] `enabled`/`disabled` filtering works correctly

---

### Phase 2: Skill Frontmatter Generation

**Goal**: Generate lightweight skill metadata for injection into agent context

**Changes**:

- [x] Add to `src/interactive/skills.rs` (~line 370):
  ```rust
  impl SkillRegistry {
      /// Generate frontmatter summary for a list of skill names
      pub fn generate_frontmatter(&self, skill_names: &[String]) -> String {
          let mut lines = vec!["## Available Skills".to_string()];
          for name in skill_names {
              if let Some(skill) = self.get(name) {
                  let aliases = if skill.aliases.is_empty() {
                      String::new()
                  } else {
                      format!(" (aliases: {})", skill.aliases.join(", "))
                  };
                  lines.push(format!("- /{}: {}{}", skill.name, skill.description, aliases));
              }
          }
          lines.join("\n")
      }
  }
  ```

- [x] Update `AgentDefinition` to include resolved skill frontmatter:
  ```rust
  impl AgentDefinition {
      /// Build full context with skills resolved
      pub fn build_context(&self, skill_registry: &SkillRegistry) -> String {
          let skills_frontmatter = skill_registry.generate_frontmatter(&self.skills);
          self.instructions.replace("{{skills_frontmatter}}", &skills_frontmatter)
      }
  }
  ```

**Success Criteria - Automated**:
- [x] `cargo test skills::generate_frontmatter` passes
- [x] `{{skills_frontmatter}}` placeholder is replaced correctly

**Success Criteria - Manual**:
- [x] Generated frontmatter includes all specified skills
- [x] Missing skills are silently skipped (not errors)

---

### Phase 3: Harness Context Injection

**Goal**: Update each harness to inject agent context via native mechanism

**Changes**:

- [x] Update `SessionConfig` (`src/harness/mod.rs:38-67`):
  ```rust
  pub struct SessionConfig {
      pub model: String,
      pub tools: Vec<String>,
      pub system_prompt: Option<String>,  // Already exists
      pub append_system_prompt: Option<String>, // NEW: for Claude Code
      pub parent: Option<SessionHandle>,
      pub is_subagent: bool,
  }
  ```

- [x] Update Claude Code harness (`src/harness/claude_code.rs:70-98`):
  ```rust
  fn build_args(&self, session: &SessionHandle, message: &str, resume: bool,
                append_prompt: Option<&str>) -> Vec<String> {
      let mut args = vec![];
      // ... existing args ...

      // Append system prompt for agent context
      if let Some(prompt) = append_prompt {
          args.push("--append-system-prompt".to_string());
          args.push(prompt.to_string());
      }

      args
  }
  ```

- [x] Update Claude Code `send()` (`src/harness/claude_code.rs:340-401`):
  - Pass `session_config.append_system_prompt` to `build_args()`

- [x] Update OpenCode harness (`src/harness/opencode.rs:66-88`):
  - Option A: Use `--append-system-prompt` if available
  - Option B: Prepend context to message as markdown block
  ```rust
  fn build_args(&self, session: &SessionHandle, message: &str,
                session_state: &OpenCodeSession, context: Option<&str>) -> Vec<String> {
      // If context provided, prepend to message
      let full_message = match context {
          Some(ctx) => format!("<agent-context>\n{}\n</agent-context>\n\n{}", ctx, message),
          None => message.to_string(),
      };
      // ... rest of args
  }
  ```

- [x] Codex harness (`src/harness/codex.rs:358-397`) - already supports `system_prompt`:
  - Ensure `SessionConfig.system_prompt` is used in `start_session()`

**Success Criteria - Automated**:
- [x] `cargo check` passes
- [x] Existing harness tests still pass

**Success Criteria - Manual**:
- [x] Claude Code subagent receives `--append-system-prompt` with agent context
- [x] OpenCode subagent receives context (via `<agent-context>` message prefix)
- [x] Codex subagent receives system_prompt

---

### Phase 4: Subagent Request Enhancement

**Goal**: Extend SubagentRequest and spawn_subagent() to carry agent context

**Changes**:

- [x] Update `SubagentRequest` (`src/harness/mod.rs:108-117`):
  ```rust
  pub struct SubagentRequest {
      pub category: String,
      pub prompt: String,
      pub model: Option<String>,
      pub agent_name: Option<String>,  // NEW: specific agent to use
  }
  ```

- [ ] Update subagent tool detection in all harnesses to extract `agent_name`:
  - `src/harness/claude_code.rs:234-267` - `extract_subagent_request()`
  - `src/harness/opencode.rs:300-332` - `extract_subagent_request()`
  - `src/harness/codex.rs:317-345` - `extract_subagent_request()`

  Add parsing for `agent` or `agent_name` field in tool arguments.

- [x] Update `spawn_subagent()` (`src/agent/subagent.rs:66-229`):
  ```rust
  pub async fn spawn_subagent(
      harness: &dyn Harness,
      category: AgentCategory,
      prompt: String,
      agent_definition: Option<&AgentDefinition>, // NEW
      skill_registry: &SkillRegistry,             // NEW
      parent_transcript: Option<&mut Transcript>,
      control_rx: Option<mpsc::Receiver<AgentControl>>,
  ) -> Result<SubagentResult> {
      // Build context from agent definition if provided
      let context = agent_definition.map(|def| def.build_context(skill_registry));

      // Use agent's category/model/tools if defined, else fall back to category defaults
      let (model, tools) = match agent_definition {
          Some(def) => (
              def.model.clone().unwrap_or_else(|| category_config.model.clone()),
              def.tools.clone().unwrap_or_else(|| category_config.tools.clone()),
          ),
          None => (category_config.model.clone(), category_config.tools.clone()),
      };

      let session_config = SessionConfig {
          model,
          tools,
          system_prompt: None,
          append_system_prompt: context, // Pass agent context
          parent: None,
          is_subagent: true,
      };

      // ... rest of function
  }
  ```

- [x] Update callers of `spawn_subagent()` to pass agent registry and skill registry

**Success Criteria - Automated**:
- [x] `cargo check` passes
- [x] `cargo test agent::subagent` passes
- [x] Existing spawn tests still pass

**Success Criteria - Manual**:
- [x] Spawning with `agent_name` loads that agent's definition (integrated in proxy.rs)
- [x] Agent context appears in subagent's system prompt (via SessionConfig fields)
- [x] Skills listed in agent are visible in context (via generate_frontmatter)

---

### Phase 5: Configuration and Runtime Selection

**Goal**: Allow users to control which agents are available

**Changes**:

- [x] Update `.descartes/config.toml` template (created by `init`):
  ```toml
  # Agent configuration
  # Agents are defined in .descartes/agents/<name>/AGENT.md
  [agents]
  directory = ".descartes/agents"

  # Control which agents are available. Use EITHER 'enabled' OR 'disabled', not both.
  # If 'enabled' is set, only those agents are available.
  # If 'disabled' is set, all agents EXCEPT those listed are available.
  # If neither is set, all agents are available.

  # enabled = ["code-analyzer", "security-reviewer"]
  # disabled = ["experimental-agent"]
  ```

- [x] Add validation in `AgentDefinitionRegistry::load()`:
  - Warn if both `enabled` and `disabled` are set (use `enabled`, ignore `disabled`)
  - Log which agents are loaded vs filtered out

- [ ] Update BAML orchestrator prompts (`baml_src/orchestrator.baml`):
  - Add agent list to `SelectSubagent` context
  - Allow selection by agent name in addition to category
  - (Deferred - BAML integration is separate concern)

- [x] Add CLI command or interactive option to list available agents:
  ```
  descartes agents list
  descartes agents show code-analyzer
  ```

**Success Criteria - Automated**:
- [x] `cargo check` passes
- [x] Config validation tests pass

**Success Criteria - Manual**:
- [x] Setting `enabled = ["code-analyzer"]` makes only that agent available
- [x] Setting `disabled = ["experimental"]` hides only that agent
- [x] `descartes agents list` shows available agents

---

## Open Questions

*None remaining - all clarified with user*

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| **Breaking existing subagent spawning** | Phase 4 changes are additive; `agent_definition` is optional |
| **Context too large for some models** | Agent instructions should be concise; can add token budget warning |
| **Harness-specific behavior differences** | Document differences; test each harness explicitly |
| **BAML out of sync with available agents** | Phase 5 includes BAML updates; add runtime validation |

## File Structure After Implementation

```
.descartes/
├── config.toml              # [agents] section added
├── agents/                  # NEW: Agent definitions
│   ├── code-analyzer/
│   │   └── AGENT.md        # Anthropic skill format
│   ├── security-reviewer/
│   │   └── AGENT.md
│   └── fast-builder/
│       └── AGENT.md
├── skills/                  # Existing skills
│   ├── skills.toml
│   └── *.md
└── transcripts/
```

## Dependencies

- Phase 2 depends on Phase 1 (needs AgentDefinition to resolve skills)
- Phase 3 depends on Phase 1 (needs SessionConfig changes)
- Phase 4 depends on Phases 1, 2, 3 (integrates all pieces)
- Phase 5 depends on Phase 4 (needs working agent spawning)
