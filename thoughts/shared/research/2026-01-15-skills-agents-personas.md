---
date: 2026-01-15T00:00:00-08:00
topic: "Skills, Agents, and Personas in Descartes Orchestrator"
tags: [research, codebase, descartes, skills, agents, personas, extensibility]
status: complete
---

# Research: Skills, Agents, and Personas in Descartes

## Research Question

How can the Descartes orchestrator and subagents have access to skills/agents/commands? How can spawned agents have "personas" with their own context? How can end users add their own and select which are available?

## Summary

Descartes currently has **three separate but related systems** for providing context and capabilities to agents:

1. **Agent Categories** (`src/agent/category.rs`) - Define agent roles with model tiers, tool access, and execution behavior
2. **Skills System** (`src/interactive/skills.rs`) - Loadable prompt templates with variable substitution
3. **Guidance System** (`src/config.rs`) - User-injected context strings per agent category

The current implementation does **NOT** dynamically route skills to spawned subagents. Instead:
- Categories define what tools/models an agent uses
- Skills are interactive commands for human users
- Guidance is static context injected into prompts

To provide "personas" (agents with custom context/capabilities), users would need to:
1. Define custom categories in `.descartes/config.toml` with specific tools/models
2. Create skill prompt templates that reference those categories
3. Add guidance strings for category-specific context

## Detailed Findings

### 1. Agent Categories System

**Location**: `descartes/descartes/src/agent/category.rs:10-161`

Agent categories define the **role and capabilities** of each agent type:

```rust
pub enum AgentCategory {
    Searcher,        // Fast parallel code search (Sonnet, read-only)
    Analyzer,        // Deep code analysis (Sonnet, read-only)
    Builder,         // Code implementation (Opus, full tools)
    FastBuilder,     // Fast implementation (Grok, full tools)
    BuilderReviewer, // Deep review and fixes (Opus, review tools)
    Validator,       // Test runner with backpressure (Sonnet, bash only)
    Planner,         // Task planning and breakdown (Opus, read + bash)
    Custom(String),  // User-defined custom category
}
```

Each category has a `CategoryConfig` with:
- `model` - Which LLM to use (opus, sonnet, grok-code-fast-1)
- `harness` - Which harness to use (claude-code, opencode, codex)
- `tools` - Available tools (read, write, edit, bash)
- `parallel` - Can run concurrently
- `backpressure` - Acts as a validation gate
- `prompt_template` - Optional custom prompt file

**Custom Categories**: Any unrecognized string becomes `AgentCategory::Custom(name)`:

```rust
impl FromStr for AgentCategory {
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "searcher" | "search" => Ok(AgentCategory::Searcher),
            // ... built-in matches ...
            other => Ok(AgentCategory::Custom(other.to_string())),
        }
    }
}
```

**User Configuration** (`.descartes/config.toml`):

```toml
[categories.code-analyzer]
description = "Deep code analysis agent"
model = "opus"
harness = "claude-code"
tools = ["read", "bash"]
parallel = true
backpressure = false
prompt_template = "prompts/code-analyzer.md"

[categories.security-reviewer]
description = "Security-focused code review"
model = "opus"
tools = ["read"]
prompt_template = "prompts/security-review.md"
```

### 2. Skills System

**Location**: `descartes/descartes/src/interactive/skills.rs:1-620`

Skills are **loadable prompt templates** that can:
- Define variables for substitution
- Specify an agent category to use
- Include auto-context (git diff, SCUD tasks, etc.)
- Auto-start an agent after loading

**Skill Definition**:

```rust
pub struct Skill {
    pub name: String,                    // Command name (without /)
    pub description: String,             // Short description
    pub prompt_file: PathBuf,            // Path to prompt markdown
    pub category: Option<String>,        // Agent category to use
    pub auto_start: bool,                // Auto-start agent after loading
    pub variables: Vec<SkillVariable>,   // Variables for substitution
    pub auto_context: Vec<String>,       // Auto-context to include
    pub aliases: Vec<String>,            // Alternate names
}
```

**Built-in Skills** (7 total):
- `create_plan` - Planning with analyzer category
- `implement_plan` - Implementation with builder category
- `research` - Research with searcher category
- `commit` - Git commits with builder category
- `review` - Code review with validator category
- `fix` - Bug fixing with builder category
- `test` - Test running with validator category

**Search Paths** for custom skills:
1. `.descartes/skills/`
2. `.claude/skills/`
3. `~/.config/descartes/skills/`

**User-Defined Skills** (`.descartes/skills/skills.toml`):

```toml
[[skills]]
name = "security_audit"
description = "Run a security audit on the codebase"
prompt_file = "security_audit.md"
category = "security-reviewer"  # References custom category
auto_start = true
aliases = ["audit", "sa"]

[[skills.variables]]
name = "target"
description = "File or directory to audit"
required = false
default = "."
```

**Variable Substitution** in prompt templates:
- `{{variable_name}}` - Named variable
- `$1`, `$2`, etc. - Positional arguments
- `$*` - All arguments

### 3. User Guidance System

**Location**: `descartes/descartes/src/config.rs:543-603`

Guidance provides **user-injected context** per agent category:

```rust
pub struct GuidanceConfig {
    pub global: Option<String>,          // Included in all prompts
    pub builder: Option<String>,         // Builder-specific guidance
    pub review: Option<String>,          // Reviewer-specific guidance
    pub validator: Option<String>,       // Validator-specific guidance
}
```

**Configuration** (`.descartes/config.toml`):

```toml
[guidance]
global = "Always follow existing code patterns. Prefer small, focused changes."
builder = "Run tests after making changes. Use cargo check before cargo test."
review = "Check for security issues and edge cases."
validator = "Use cargo test --all-features for full coverage."
```

**Injection Point**: `GuidanceConfig::for_context()` combines global + category-specific guidance and injects it into agent prompts.

### 4. How Subagents Currently Receive Context

**Location**: `descartes/descartes/src/agent/subagent.rs:66-229`

When spawning a subagent:

1. Category is parsed from string to `AgentCategory` enum
2. `CategoryConfig` is retrieved (from config or defaults)
3. `SessionConfig` is created with model, tools, and `is_subagent: true`
4. Prompt is passed directly to the harness

**Current Limitations**:
- Subagents receive ONLY the prompt string, not skills or guidance
- No mechanism to route skills to subagents
- No "persona" context beyond the category's default config
- 1-level depth limit prevents subagents from spawning further agents

### 5. BAML Orchestrator Prompts

**Location**: `descartes/descartes/baml_src/orchestrator.baml`

The BAML `SelectSubagent` function chooses which category handles a task:

```baml
function SelectSubagent(
    task_title: string,
    task_description: string,
    available_context: string,
    additional_context: string?
) -> SubagentSelection {
    client GPT4oMini
    prompt #"
        Select the right subagent for this task.

        ## Subagent Categories
        - searcher: Find files, grep codebase, explore
        - analyzer: Understand code, research, plan
        - builder: Write code, make changes
        - validator: Run tests, check quality
    "#
}
```

**Limitation**: BAML only knows about 4 base categories, not custom ones defined in config.

### 6. Current Directory Structure

```
.descartes/
├── config.toml              # Main configuration
│   ├── [categories.*]       # Custom agent categories
│   ├── [guidance]           # User guidance strings
│   ├── [swarm]              # Swarm orchestration settings
│   └── [scud]               # SCUD integration
├── transcripts/             # Agent execution transcripts
└── skills/                  # Custom skill prompts
    ├── skills.toml          # Skills manifest
    └── *.md                 # Skill prompt templates

prompts/                     # Default prompt templates
├── plan.md                  # Planning mode prompt
└── build.md                 # Building mode prompt
```

## Architecture Documentation

### How "Personas" Could Work Today

A "persona" is essentially a **custom category + skill + guidance** combination:

1. **Define Custom Category** (`.descartes/config.toml`):
   ```toml
   [categories.code-analyzer]
   description = "Expert code analyzer persona"
   model = "opus"
   harness = "claude-code"
   tools = ["read", "bash"]
   prompt_template = "prompts/code-analyzer-persona.md"
   ```

2. **Create Skill** (`.descartes/skills/skills.toml`):
   ```toml
   [[skills]]
   name = "analyze"
   description = "Deep code analysis with expert persona"
   prompt_file = "analyze.md"
   category = "code-analyzer"
   auto_start = true
   auto_context = ["git_status"]
   ```

3. **Add Guidance** (`.descartes/config.toml`):
   ```toml
   [guidance]
   code-analyzer = """
   You are an expert code analyst. Focus on:
   - Architecture patterns and anti-patterns
   - Performance implications
   - Security considerations
   """
   ```

4. **Create Prompt Template** (`.descartes/skills/analyze.md`):
   ```markdown
   # Deep Code Analysis

   Analyze the following code with expert insight.

   ## Target
   {{target}}

   ## Focus Areas
   - Architecture and design patterns
   - Potential bugs and edge cases
   - Performance bottlenecks
   - Security vulnerabilities
   ```

### What's Missing for Full Persona Support

1. **Subagent Persona Injection**: Currently subagents don't receive guidance or skills
2. **Dynamic Category Selection**: BAML only knows 4 categories, can't select custom ones
3. **Skill-to-Subagent Routing**: No mechanism to invoke skills when spawning subagents
4. **Persona Registry**: No unified way to define and discover available personas

## Code References

- `descartes/descartes/src/agent/category.rs:10-161` - AgentCategory enum and config
- `descartes/descartes/src/agent/subagent.rs:66-229` - Subagent spawning logic
- `descartes/descartes/src/interactive/skills.rs:14-620` - Skills system
- `descartes/descartes/src/interactive/commands.rs:10-422` - Command system
- `descartes/descartes/src/config.rs:386-603` - CategoryConfig and GuidanceConfig
- `descartes/descartes/baml_src/orchestrator.baml:76-108` - SelectSubagent function
- `descartes/descartes/src/ralph_loop.rs:373-413` - Category selection in Ralph loop

## Open Questions

1. **Should personas be first-class entities?** Currently they're assembled from categories + skills + guidance. A unified `Persona` struct might be cleaner.

2. **How should custom categories integrate with BAML?** The BAML `SelectSubagent` only knows 4 base categories. Options:
   - Dynamically generate BAML prompts with all configured categories
   - Map custom categories to base categories with override prompts
   - Skip BAML for custom category selection

3. **Should subagents have access to skills?** Currently they receive only a prompt. Adding skill invocation would enable richer agent capabilities.

4. **How to handle persona discovery?** End users need to see what personas are available. A registry or list command would help.

5. **Per-project vs global personas?** Currently config is per-project (`.descartes/`) with fallback to global (`~/.descartes/`). Should personas be shareable across projects?
