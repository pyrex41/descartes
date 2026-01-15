---
date: 2026-01-15T17:06:16-06:00
researcher: Claude
git_commit: 6450580efae24989c9b685d77bacbc6b68e750f4
branch: master
repository: pyrex41/descartes
topic: "Review Process, Guidance System, and Planning Workflows"
tags: [research, codebase, validation, guidance, planning, skills]
status: complete
last_updated: 2026-01-15
last_updated_by: Claude
---

# Research: Review Process, Guidance System, and Planning Workflows

**Date**: 2026-01-15T17:06:16-06:00
**Researcher**: Claude
**Git Commit**: 6450580efae24989c9b685d77bacbc6b68e750f4
**Branch**: master
**Repository**: pyrex41/descartes

## Research Question

1. Where does the review task sit in the process? Is it after each wave and configurable?
2. Does it have specific prompting?
3. How can prompts be augmented for different contexts?
4. What planning workflows exist (research → plan → PRD)?
5. What's missing from the GitHub Pages documentation?

## Summary

The research reveals several features that are **implemented but not documented** in the GitHub Pages:

1. **Validation runs after each ROUND, not wave** - uses automated commands, not LLM review
2. **Guidance system exists** for prompt augmentation but only uses "builder" context
3. **Skills system** can load Claude Code slash commands for research/planning workflows
4. **builder-reviewer category exists** but is NOT used in swarm execution

## Detailed Findings

### 1. Review/Validation Process

#### When Validation Runs

**Location**: `descartes/src/swarm_executor.rs:360-437`

Validation runs **after each round** (not after each wave). Waves are split into rounds based on `round_size`:

```rust
// Line 260: Waves split into rounds
let rounds: Vec<&[&Task]> = wave.chunks(self.round_size).collect();

// Line 361: Validation trigger condition
if (self.validate || validation_requested) && !bp_config.commands.is_empty()
```

**Flow**:
1. Wave contains N tasks
2. Wave splits into rounds of `round_size` (default: 5)
3. Each round executes tasks sequentially
4. After round completes, validation runs if enabled
5. If validation fails, all tasks in that round marked `Failed`
6. Breaks out of wave processing on failure

#### Backpressure Configuration

**Location**: `scud-cli/src/backpressure.rs:97-117`

```toml
# .scud/config.toml
[swarm.backpressure]
commands = ["cargo build", "cargo test", "cargo clippy -- -D warnings"]
stop_on_failure = true   # Stop at first failure
timeout_secs = 300       # Per-command timeout
```

If not configured, **auto-detection** occurs based on project type:
- **Rust**: `cargo build`, `cargo test`
- **Node.js**: Detects `package.json` scripts (build, test, lint, typecheck)
- **Python**: `pytest` if `pyproject.toml` exists
- **Go**: `go build ./...`, `go test ./...`

#### builder-reviewer Category (NOT USED)

**Location**: `descartes/src/config.rs:215-226`

The `builder-reviewer` category **exists** but is **not used** in swarm execution:

```rust
categories.insert("builder-reviewer", CategoryConfig {
    description: "Deep review and fixes",
    model: "opus",
    harness: Some("claude-code"),
    tools: vec!["read", "edit", "bash"],
    parallel: false,
    backpressure: false,  // NOT a backpressure gate
});
```

**Important**: Swarm executor does NOT spawn reviewer agents. All "review" is automated command validation.

---

### 2. Guidance/Prompt Augmentation System

#### GuidanceConfig Structure

**Location**: `descartes/src/config.rs:581-641`

```rust
pub struct GuidanceConfig {
    pub global: Option<String>,     // Included in ALL prompts
    pub builder: Option<String>,    // For builder/fast-builder
    pub review: Option<String>,     // For review/builder-reviewer
    pub validator: Option<String>,  // For validator
}
```

#### Configuration Example

```toml
# .descartes/config.toml
[guidance]
global = "Always follow existing code patterns. Prefer small, focused changes."
builder = "Run tests after making changes. Use cargo check before cargo test."
review = "Check for security issues and edge cases."
validator = "Use cargo test --all-features for full coverage."
```

#### Context Resolution Method

**Location**: `descartes/src/config.rs:614-641`

```rust
pub fn for_context(&self, context: &str) -> Option<String> {
    let specific = match context {
        "builder" | "fast-builder" => self.builder.as_deref(),
        "review" | "builder-reviewer" => self.review.as_deref(),
        "validator" => self.validator.as_deref(),
        _ => None,
    };
    // Combines global + specific with "\n\n" separator
}
```

#### Prompt Injection

**Location**: `descartes/src/spec.rs:333-336`

```rust
let guidance_section = guidance
    .map(|g| format!("## User Guidance\n\n{}\n\n", g))
    .unwrap_or_default();
```

#### LIMITATION: Hard-coded to "builder" context

**Location**: `descartes/src/swarm_executor.rs:499`

```rust
let guidance = config.guidance.for_context("builder");  // ALWAYS "builder"
```

The swarm executor always uses "builder" context, even though config supports "validator" and "review".

---

### 3. Agent Categories System

#### Built-in Categories

**Location**: `descartes/src/agent/category.rs:10-29`

| Category | Model Tier | Parallel | Backpressure | Harness |
|----------|------------|----------|--------------|---------|
| `searcher` | Fast (sonnet) | Yes | No | default |
| `analyzer` | Fast | Yes | No | default |
| `builder` | Strong (opus) | No | No | claude-code |
| `fast-builder` | Fast | No | No | default |
| `builder-reviewer` | Strong | No | No | claude-code |
| `validator` | Fast | No | **Yes** | default |
| `planner` | Strong | No | No | claude-code |
| `orchestrator` | Strong | No | No | claude-code |

#### Category Prompt Templates (NOT USED)

**Location**: `descartes/src/config.rs:449`

Each `CategoryConfig` has `prompt_template: Option<PathBuf>` but it's **never read or used**.

---

### 4. Skills System

#### Built-in Skills

**Location**: `descartes/src/interactive/skills.rs:200-315`

| Skill | Aliases | Category | Description |
|-------|---------|----------|-------------|
| `create_plan` | `plan`, `cp` | planner | Create implementation plan from research |
| `implement_plan` | `implement`, `ip` | builder | Implement tasks from plan |
| `research` | `r` | searcher | Research topic or codebase area |
| `commit` | `c` | - | Create git commit |
| `review` | `rv` | builder-reviewer | Review code changes |
| `fix` | `f` | builder | Fix an issue or bug |
| `test` | `t` | validator | Run tests and fix failures |

#### Cross-Tool Compatibility

**Location**: `descartes/src/interactive/skills.rs:158-183`

Skills can be loaded from multiple paths:
- `.descartes/skills/` - Descartes native
- `.claude/commands/` - Claude Code commands
- `.opencode/skill/` - OpenCode skills
- `.codex/skills/` - Codex skills

#### Skill Execution Flow

**Location**: `descartes/src/interactive/session.rs:396-431`

1. Lookup skill in registry
2. Load prompt with variable substitution (`{{variable}}`, `$1`, `$*`)
3. Merge pending context
4. Determine agent category
5. Start agent with `start_agent()`

---

### 5. Claude Code Slash Commands (Available in .claude/commands/)

#### Research Commands
- `/cl:research_codebase` - Comprehensive codebase research with sub-agents
- `/cl:research_codebase_nt` - Research without thoughts directory

#### Planning Commands
- `/cl:create_plan` - Interactive implementation plan creation
- `/cl:create_plan_nt` - Planning without thoughts prompts
- `/cl:iterate_plan` - Update existing plans

#### Implementation Commands
- `/cl:implement_plan` - Execute approved plans phase-by-phase

#### Flow Commands (Handoff-based)
- `/flow:research` - Research with handoff document
- `/flow:plan` - Planning with SCUD task generation
- `/flow:implement` - Wave-based implementation
- `/flow:resume` - Resume from handoff

---

### 6. What's Missing from GitHub Pages Docs

#### Not Documented At All

1. **Guidance System** (`[guidance]` section in config)
   - How to inject custom context into prompts
   - global, builder, review, validator fields

2. **Skills System**
   - Built-in skills (research, create_plan, implement_plan, etc.)
   - How to create custom skills
   - Cross-tool compatibility

3. **Backpressure Configuration**
   - `[swarm.backpressure]` section
   - Auto-detection behavior
   - `stop_on_failure`, `timeout_secs` options

4. **Category Configuration Details**
   - Full category customization options
   - Tool assignments per category
   - Model tier assignments

5. **Interactive Mode Commands**
   - `/pause`, `/resume`, `/cancel`
   - `/context`, `/scud`, `/waves`, `/diff`
   - `/skill` command

#### Partially Documented

1. **Validation** - Mentioned but not detailed
   - Not clear that it runs per-round, not per-wave
   - Auto-detection not explained

2. **Agent Categories** - Listed but not explained
   - Model tier not explained
   - Parallel/backpressure flags not documented

---

## Architecture Documentation

### Validation Flow

```
┌──────────────────────────────────────────────────────┐
│ Wave N                                                │
│  ┌────────────────────────────────────────────────┐  │
│  │ Round 1 (tasks 1-5)                            │  │
│  │  • Execute task 1                              │  │
│  │  • Execute task 2                              │  │
│  │  • ...                                         │  │
│  │  • Execute task 5                              │  │
│  └─────────────────────┬──────────────────────────┘  │
│                        ▼                              │
│  ┌────────────────────────────────────────────────┐  │
│  │ Backpressure Validation                        │  │
│  │  • cargo build                                 │  │
│  │  • cargo test                                  │  │
│  │  • (stop on first failure if configured)      │  │
│  └─────────────────────┬──────────────────────────┘  │
│                        │                              │
│           ┌────────────┴────────────┐                │
│           ▼                         ▼                │
│    [All Pass]                [Any Fail]              │
│         │                         │                  │
│         ▼                         ▼                  │
│    Continue to              Mark all round           │
│    Round 2                  tasks as Failed          │
│                             Break wave loop          │
└──────────────────────────────────────────────────────┘
```

### Prompt Building Flow

```
┌─────────────────────────────────────────────────────┐
│ build_prompt()                                       │
│                                                      │
│  ┌───────────────────────────────────────────────┐  │
│  │ 1. Guidance Section (if configured)           │  │
│  │    config.guidance.for_context("builder")     │  │
│  │    → "## User Guidance\n\n{text}\n\n"         │  │
│  └───────────────────────────────────────────────┘  │
│                        ▼                             │
│  ┌───────────────────────────────────────────────┐  │
│  │ 2. Task Introduction                          │  │
│  │    "You are implementing SCUD task {id}..."   │  │
│  └───────────────────────────────────────────────┘  │
│                        ▼                             │
│  ┌───────────────────────────────────────────────┐  │
│  │ 3. Task Spec (~5k tokens)                     │  │
│  │    • Task details from SCUD                   │  │
│  │    • Plan section (if --plan provided)        │  │
│  │    • Additional spec files                    │  │
│  └───────────────────────────────────────────────┘  │
│                        ▼                             │
│  ┌───────────────────────────────────────────────┐  │
│  │ 4. Verification Commands                      │  │
│  │    "After implementation, run: {commands}"    │  │
│  └───────────────────────────────────────────────┘  │
│                        ▼                             │
│  ┌───────────────────────────────────────────────┐  │
│  │ 5. Swarm Instructions                         │  │
│  │    Fresh context reminder, completion steps   │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## Code References

### Validation System
- `descartes/src/swarm_executor.rs:360-437` - Validation execution
- `scud-cli/src/backpressure.rs:226-284` - `run_validation()` function
- `scud-cli/src/backpressure.rs:97-117` - `BackpressureConfig` struct
- `scud-cli/src/backpressure.rs:147-194` - Auto-detection logic

### Guidance System
- `descartes/src/config.rs:581-641` - `GuidanceConfig` struct
- `descartes/src/config.rs:614-641` - `for_context()` method
- `descartes/src/spec.rs:333-336` - Guidance injection
- `descartes/src/swarm_executor.rs:499` - Hard-coded "builder" context

### Skills System
- `descartes/src/interactive/skills.rs:14-38` - `Skill` struct
- `descartes/src/interactive/skills.rs:200-315` - Built-in skills
- `descartes/src/interactive/skills.rs:158-183` - Cross-tool paths
- `descartes/src/interactive/session.rs:396-431` - Skill execution

### Agent Categories
- `descartes/src/agent/category.rs:10-29` - `AgentCategory` enum
- `descartes/src/config.rs:126-239` - Default category configs
- `descartes/src/agent/mod.rs:59-147` - Category default implementations

### CLI Commands
- `descartes/src/main.rs:30-174` - `Commands` enum
- `descartes/src/main.rs:176-193` - `SkillCommands` enum
- `descartes/src/interactive/commands.rs:163-269` - Interactive commands

---

## Recommendations for Documentation Updates

### New Docs Needed

1. **configuration.md** - Add sections for:
   - `[guidance]` configuration
   - `[swarm.backpressure]` configuration
   - Full category customization

2. **skills.md** (new page) - Document:
   - Built-in skills
   - Creating custom skills
   - Variable substitution
   - Cross-tool compatibility

3. **interactive.md** (new page) - Document:
   - Interactive mode commands
   - Session states
   - Context injection

4. **workflows.md** - Add section for:
   - Research → Plan → PRD workflow
   - Using skills for brownfield projects

### Updates to Existing Docs

1. **swarm.md** - Clarify:
   - Validation runs per-round, not per-wave
   - Auto-detection behavior
   - builder-reviewer is NOT used

2. **harnesses.md** - Add:
   - Category-to-harness mapping
   - Model tier explanation
