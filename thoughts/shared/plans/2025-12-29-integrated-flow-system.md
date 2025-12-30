# Integrated Flow System Implementation Plan

## Overview

Merge the existing Ralph-style iterative loop from `local-master-backup` and extend it with SCUD integration, handoff documents, and a `/flow:*` command system to create an autonomous development workflow.

## Current State Analysis

### Existing Infrastructure (from `local-master-backup`)
- `descartes/core/src/iterative_loop.rs` - 1312 lines, complete loop executor
- `descartes/cli/src/commands/loop_cmd.rs` - 255 lines, CLI commands
- `descartes/gui/src/loop_state.rs` - 103 lines, GUI state
- `descartes/gui/src/loop_view.rs` - 268 lines, GUI view

### Existing Infrastructure (current branch)
- `.claude/commands/cl/*` - Research/Plan/Implement commands
- `.claude/commands/scud/*` - SCUD agent commands
- `.claude/agents/cl/*` - Sub-agent definitions
- `descartes/core/src/flow_executor.rs` - Flow orchestration
- SCUD CLI fully integrated

### Key Discoveries
- Existing loop uses `<promise>` tags for completion
- SCUD provides objective completion via task states
- Flow executor has retry/state patterns we can reuse
- Sub-agent infrastructure already supports spawning specialized agents

## Desired End State

After implementation:

1. **`descartes loop`** - Low-level generic loop (from backup)
2. **`/flow:research`** - Research with handoff generation
3. **`/flow:plan`** - Planning with SCUD task creation
4. **`/flow:implement`** - SCUD-aware loop with wave execution
5. **`/flow:resume`** - Resume from any handoff
6. **Handoff documents** - Structured context transfer

### Verification
- All existing loop tests pass
- End-to-end flow test: research → plan → implement on sample task
- Handoff resume works across sessions
- GUI shows wave-based progress

## What We're NOT Doing

- Modifying the core `IterativeLoop` struct (we wrap it)
- Changing existing `cl:*` commands (we create new `flow:*` versions)
- Breaking existing SCUD CLI commands
- Adding external dependencies

---

## Handoff Document Format Specification

### Location
`thoughts/shared/handoffs/{phase}/{YYYY-MM-DD}_{HH-MM}_{tag-or-topic}.md`

Examples:
- `thoughts/shared/handoffs/research/2025-12-29_14-30_auth-system.md`
- `thoughts/shared/handoffs/plan/2025-12-29_15-45_auth-system.md`
- `thoughts/shared/handoffs/implement/2025-12-29_18-00_auth-system.md`

### Research Handoff Format

```markdown
---
type: handoff
phase: research
timestamp: 2025-12-29T14:30:00Z
topic: "{research topic}"
research_doc: "thoughts/shared/research/{path}.md"
git_commit: "{commit hash}"
branch: "{branch name}"
next_phase: plan
next_command: "/flow:plan {handoff-path}"
---

# Research Handoff: {Topic}

## Status
Research complete. Ready for planning phase.

## Research Document
`{path to full research document}`

## Key Findings Summary

### Finding 1: {Title}
{2-3 sentence summary}
- **Reference**: `{file:line}`
- **Implication**: {what this means for implementation}

### Finding 2: {Title}
{2-3 sentence summary}
- **Reference**: `{file:line}`
- **Implication**: {what this means for implementation}

### Finding 3: {Title}
{2-3 sentence summary}
- **Reference**: `{file:line}`
- **Implication**: {what this means for implementation}

## Critical Files
Files the planner MUST read:
1. `{path}` - {why important}
2. `{path}` - {why important}
3. `{path}` - {why important}

## Existing Patterns to Follow
- **Pattern**: {name} in `{file:line}`
- **Pattern**: {name} in `{file:line}`

## Recommended Planning Approach
{2-3 sentences on how to approach the plan based on research}

## Open Questions for Planning
- {Question that research couldn't answer}
- {Design decision that needs human input}

---

## Next Steps

To continue in a new session:
```bash
# Start new Claude session, then run:
/flow:plan {this-handoff-path}
```
```

### Planning Handoff Format

```markdown
---
type: handoff
phase: plan
timestamp: 2025-12-29T15:45:00Z
topic: "{feature name}"
plan_doc: "thoughts/shared/plans/{path}.md"
research_handoff: "thoughts/shared/handoffs/research/{path}.md"
scud_tag: "{tag-name}"
total_tasks: {N}
total_waves: {M}
total_complexity: {points}
git_commit: "{commit hash}"
branch: "{branch name}"
next_phase: implement
next_command: "/flow:implement {scud-tag}"
---

# Planning Handoff: {Feature Name}

## Status
Planning complete. SCUD tasks created. Ready for implementation.

## Documents
- **Plan**: `{path to plan document}`
- **Research**: `{path to research document}`

## SCUD Overview

**Tag**: `{tag-name}`
**Tasks**: {N} total
**Waves**: {M} waves
**Complexity**: {points} points

### Wave Breakdown

| Wave | Tasks | Points | Focus |
|------|-------|--------|-------|
| 1 | {n} | {p} | {description} |
| 2 | {n} | {p} | {description} |
| ... | ... | ... | ... |

### Task Summary

#### Wave 1 (Foundation)
- [ ] Task {id}: {title} [{complexity}]
- [ ] Task {id}: {title} [{complexity}]

#### Wave 2 (Core)
- [ ] Task {id}: {title} [{complexity}] ← {dependency}
- [ ] Task {id}: {title} [{complexity}] ← {dependency}

## Critical Context for Implementation

### Architecture Decisions
- {Decision 1}: {rationale}
- {Decision 2}: {rationale}

### Patterns to Follow
- **For {component}**: Follow pattern in `{file:line}`
- **For {component}**: Follow pattern in `{file:line}`

### Testing Strategy
- Unit tests: {approach}
- Integration tests: {approach}
- Manual verification: {what to test}

## Files to Modify
1. `{path}` - {what changes}
2. `{path}` - {what changes}
3. `{path}` - {what changes}

## Success Criteria
- [ ] All SCUD tasks marked DONE
- [ ] `make check test` passes
- [ ] {Manual verification item}
- [ ] {Manual verification item}

---

## Next Steps

To continue in a new session:
```bash
# Start new Claude session, then run:
/flow:implement {scud-tag}

# Or to resume from this handoff:
/flow:resume {this-handoff-path}
```
```

### Implementation Handoff Format

```markdown
---
type: handoff
phase: implement
timestamp: 2025-12-29T18:00:00Z
topic: "{feature name}"
scud_tag: "{tag-name}"
plan_doc: "thoughts/shared/plans/{path}.md"
plan_handoff: "thoughts/shared/handoffs/plan/{path}.md"
tasks_completed: {N}
tasks_total: {M}
waves_completed: {W}
waves_total: {T}
status: "{complete|partial|blocked}"
git_commits: ["{hash1}", "{hash2}"]
branch: "{branch name}"
---

# Implementation Handoff: {Feature Name}

## Status
{Complete description of current state}

## Progress

**Tag**: `{tag-name}`
**Tasks**: {completed}/{total} ({percentage}%)
**Waves**: {completed}/{total}

### Completed Waves
- **Wave 1**: {summary} (commit: `{hash}`)
- **Wave 2**: {summary} (commit: `{hash}`)

### Current Wave (if partial)
- Wave {N}: {in-progress-tasks}

### Blocked Tasks (if any)
- Task {id}: {title} - **Blocked**: {reason}

## Commits Made
1. `{hash}` - {message}
2. `{hash}` - {message}

## Files Changed
{git diff --stat summary}

## Key Learnings
- {Learning 1}: {description}
- {Learning 2}: {description}

## What Worked Well
- {Success 1}
- {Success 2}

## Challenges Encountered
- {Challenge 1}: {how resolved or still open}
- {Challenge 2}: {how resolved or still open}

## Manual Testing Required
- [ ] {Test case 1}
- [ ] {Test case 2}

## Follow-up Items
- [ ] {Item 1}
- [ ] {Item 2}

---

## Next Steps

If implementation is complete:
```bash
# Create PR
/cl:describe_pr

# Or run retrospective
/scud:retrospective
```

If resuming partial implementation:
```bash
/flow:resume {this-handoff-path}
# or
/flow:implement {scud-tag}
```
```

---

## Implementation Phases

### Phase 1: Merge Existing Loop Infrastructure

#### Overview
Cherry-pick the loop implementation from `local-master-backup` and integrate with current codebase.

#### Changes Required

##### 1. Core Library
**Files to add from `local-master-backup`**:
- `descartes/core/src/iterative_loop.rs` (full file)

**Files to modify**:
- `descartes/core/src/lib.rs` - Add module export

```rust
// Add to lib.rs
mod iterative_loop;
pub use iterative_loop::*;
```

##### 2. CLI Commands
**Files to add from `local-master-backup`**:
- `descartes/cli/src/commands/loop_cmd.rs` (full file)

**Files to modify**:
- `descartes/cli/src/commands/mod.rs` - Add module
- `descartes/cli/src/main.rs` - Add command routing

##### 3. GUI Components
**Files to add from `local-master-backup`**:
- `descartes/gui/src/loop_state.rs` (full file)
- `descartes/gui/src/loop_view.rs` (full file)

**Files to modify**:
- `descartes/gui/src/lib.rs` - Add exports
- `descartes/gui/src/main.rs` - Integrate view

#### Success Criteria

##### Automated Verification
- [ ] `cargo check -p descartes-core` passes
- [ ] `cargo check -p descartes-cli` passes
- [ ] `cargo check -p descartes-gui` passes
- [ ] `cargo test -p descartes-core` passes (loop tests)
- [ ] `descartes loop --help` shows commands

##### Manual Verification
- [ ] `descartes loop start --command echo --prompt "test" --max-iterations 2` works
- [ ] GUI shows loop view (if accessible)

---

### Phase 2: SCUD-Aware Loop Wrapper

#### Overview
Create `ScudIterativeLoop` that wraps `IterativeLoop` with SCUD task completion detection and wave-based execution.

#### Changes Required

##### 1. New Module: `scud_loop.rs`
**File**: `descartes/core/src/scud_loop.rs`

```rust
//! SCUD-aware iterative loop
//!
//! Wraps IterativeLoop with SCUD task tracking for objective completion
//! detection and wave-based execution.

use crate::{IterativeLoop, IterativeLoopConfig, IterativeLoopResult, IterativeExitReason};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Statistics from SCUD
#[derive(Debug, Clone, Deserialize)]
pub struct ScudStats {
    pub total: u32,
    pub done: u32,
    pub pending: u32,
    pub in_progress: u32,
    pub blocked: u32,
}

/// Wave information from SCUD
#[derive(Debug, Clone)]
pub struct ScudWave {
    pub number: u32,
    pub tasks: Vec<ScudTask>,
}

/// Task information from SCUD
#[derive(Debug, Clone)]
pub struct ScudTask {
    pub id: u32,
    pub title: String,
    pub status: String,
    pub complexity: u32,
}

/// Configuration for SCUD-aware loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScudLoopConfig {
    /// SCUD tag for task tracking
    pub tag: String,

    /// Path to plan document (for context)
    pub plan_path: Option<PathBuf>,

    /// Path to handoff document (for resume)
    pub handoff_path: Option<PathBuf>,

    /// Maximum iterations per wave (safety)
    pub max_iterations_per_wave: u32,

    /// Maximum total iterations (safety)
    pub max_total_iterations: u32,

    /// Working directory
    pub working_directory: PathBuf,

    /// Whether to spawn sub-agents per task
    pub use_sub_agents: bool,

    /// Verification command to run after each task
    pub verification_command: Option<String>,
}

/// State for SCUD loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScudLoopState {
    pub config: ScudLoopConfig,
    pub current_wave: u32,
    pub total_waves: u32,
    pub tasks_completed: u32,
    pub tasks_total: u32,
    pub iteration_count: u32,
    pub wave_commits: Vec<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// SCUD-aware iterative loop executor
pub struct ScudIterativeLoop {
    config: ScudLoopConfig,
    state: ScudLoopState,
}

impl ScudIterativeLoop {
    pub fn new(config: ScudLoopConfig) -> Result<Self> {
        let stats = Self::get_scud_stats(&config.tag)?;
        let waves = Self::get_wave_count(&config.tag)?;

        let state = ScudLoopState {
            config: config.clone(),
            current_wave: 1,
            total_waves: waves,
            tasks_completed: stats.done,
            tasks_total: stats.total,
            iteration_count: 0,
            wave_commits: Vec::new(),
            started_at: chrono::Utc::now(),
            last_activity_at: None,
        };

        Ok(Self { config, state })
    }

    /// Check if loop is complete via SCUD stats
    pub fn is_complete(&self) -> Result<bool> {
        let stats = Self::get_scud_stats(&self.config.tag)?;
        Ok(stats.pending == 0 && stats.in_progress == 0 && stats.blocked == 0)
    }

    /// Get SCUD statistics for tag
    fn get_scud_stats(tag: &str) -> Result<ScudStats> {
        let output = Command::new("scud")
            .args(["stats", "--tag", tag, "--json"])
            .output()?;
        let stats: ScudStats = serde_json::from_slice(&output.stdout)?;
        Ok(stats)
    }

    /// Get wave count for tag
    fn get_wave_count(tag: &str) -> Result<u32> {
        let output = Command::new("scud")
            .args(["waves", "--tag", tag, "--json"])
            .output()?;
        // Parse and count waves
        // ...
        Ok(1) // placeholder
    }

    /// Get next task from SCUD
    fn get_next_task(&self) -> Result<Option<ScudTask>> {
        let output = Command::new("scud")
            .args(["next", "--tag", &self.config.tag, "--json"])
            .output()?;
        if output.stdout.is_empty() {
            return Ok(None);
        }
        let task: ScudTask = serde_json::from_slice(&output.stdout)?;
        Ok(Some(task))
    }

    /// Mark task as in-progress
    fn claim_task(&self, task_id: u32) -> Result<()> {
        Command::new("scud")
            .args(["set-status", &task_id.to_string(), "in-progress", "--tag", &self.config.tag])
            .output()?;
        Ok(())
    }

    /// Mark task as done
    fn complete_task(&self, task_id: u32) -> Result<()> {
        Command::new("scud")
            .args(["set-status", &task_id.to_string(), "done", "--tag", &self.config.tag])
            .output()?;
        Ok(())
    }

    /// Execute the SCUD-aware loop
    pub async fn execute(&mut self) -> Result<IterativeLoopResult> {
        loop {
            // Check completion
            if self.is_complete()? {
                return Ok(IterativeLoopResult {
                    iterations_completed: self.state.iteration_count,
                    completion_promise_found: true,
                    completion_text: Some("All SCUD tasks completed".to_string()),
                    final_output: String::new(),
                    exit_reason: IterativeExitReason::CompletionPromiseDetected,
                    total_duration: std::time::Duration::from_secs(0),
                });
            }

            // Check iteration limit
            if self.state.iteration_count >= self.config.max_total_iterations {
                return Ok(IterativeLoopResult {
                    iterations_completed: self.state.iteration_count,
                    completion_promise_found: false,
                    completion_text: None,
                    final_output: String::new(),
                    exit_reason: IterativeExitReason::MaxIterationsReached,
                    total_duration: std::time::Duration::from_secs(0),
                });
            }

            // Get next task
            let task = match self.get_next_task()? {
                Some(t) => t,
                None => {
                    // No more tasks - wave complete
                    self.commit_wave()?;
                    self.state.current_wave += 1;
                    continue;
                }
            };

            // Claim task
            self.claim_task(task.id)?;

            // Execute task (via sub-agent if configured)
            let success = self.execute_task(&task).await?;

            if success {
                self.complete_task(task.id)?;
                self.state.tasks_completed += 1;
            }

            self.state.iteration_count += 1;
            self.state.last_activity_at = Some(chrono::Utc::now());
            self.save_state()?;
        }
    }

    /// Execute a single task
    async fn execute_task(&self, task: &ScudTask) -> Result<bool> {
        // TODO: Implement with sub-agent spawning
        Ok(true)
    }

    /// Commit current wave
    fn commit_wave(&mut self) -> Result<()> {
        let message = format!("feat({}): complete wave {}", self.config.tag, self.state.current_wave);
        let output = Command::new("git")
            .args(["add", "-A"])
            .output()?;
        let output = Command::new("git")
            .args(["commit", "-m", &message])
            .output()?;
        // Get commit hash
        let hash_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?;
        let hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();
        self.state.wave_commits.push(hash);
        Ok(())
    }

    /// Save state to file
    fn save_state(&self) -> Result<()> {
        let path = self.config.working_directory.join(".scud/loop-state.json");
        let content = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
```

##### 2. Update lib.rs
Add module export for `scud_loop`.

#### Success Criteria

##### Automated Verification
- [ ] `cargo check -p descartes-core` passes
- [ ] `cargo test -p descartes-core` passes
- [ ] Unit tests for `ScudStats`, `ScudLoopConfig` serialization

##### Manual Verification
- [ ] Create test SCUD tag with 3 tasks
- [ ] Run `ScudIterativeLoop` and verify completion detection works

---

### Phase 3: Flow Commands

#### Overview
Create the `/flow:*` slash commands for Claude Code integration.

#### Changes Required

##### 1. `/flow:research` Command
**File**: `.claude/commands/flow/research.md`

````markdown
---
description: Enhanced research with handoff generation
model: opus
---

# Flow Research

You are conducting research that will generate a structured handoff document for the planning phase.

## Process

### Step 1: Conduct Research
Follow the same process as `/cl:research_codebase_nt`:
1. Read any mentioned files FULLY
2. Spawn parallel sub-agents for research
3. Synthesize findings
4. Write research document to `thoughts/shared/research/`

### Step 2: Generate Handoff
After completing research, generate a handoff document:

**Location**: `thoughts/shared/handoffs/research/{YYYY-MM-DD}_{HH-MM}_{topic}.md`

Use this template:
```markdown
---
type: handoff
phase: research
timestamp: {ISO timestamp}
topic: "{topic}"
research_doc: "{path to research doc}"
git_commit: "{current commit}"
branch: "{current branch}"
next_phase: plan
next_command: "/flow:plan {this-handoff-path}"
---

# Research Handoff: {Topic}

## Status
Research complete. Ready for planning phase.

## Research Document
`{path}`

## Key Findings Summary
[3-5 key findings with file:line references]

## Critical Files
[3-5 files the planner must read]

## Existing Patterns to Follow
[Patterns discovered during research]

## Recommended Planning Approach
[2-3 sentences]

## Open Questions for Planning
[Questions that research couldn't answer]

---
## Next Steps
To continue: `/flow:plan {handoff-path}`
```

### Step 3: Present Summary
After writing the handoff:
```
Research complete!

📄 Research document: {path}
📋 Handoff document: {handoff-path}

Key findings:
- {finding 1}
- {finding 2}
- {finding 3}

To continue in a new session:
/flow:plan {handoff-path}
```
````

##### 2. `/flow:plan` Command
**File**: `.claude/commands/flow/plan.md`

````markdown
---
description: Planning with SCUD task generation and handoff
model: opus
---

# Flow Plan

You create implementation plans and automatically generate SCUD tasks.

## Initial Response

If a handoff path is provided:
1. Read the handoff document
2. Read the referenced research document
3. Read all critical files listed

If no handoff but a topic is provided:
1. Prompt for more context or suggest running `/flow:research` first

## Process

### Step 1: Create Plan
Follow `/cl:create_plan_nt` process:
1. Gather context
2. Research with sub-agents
3. Present design options
4. Write plan to `thoughts/shared/plans/`

### Step 2: Generate SCUD Tasks
After plan is approved, create SCUD tasks:

```bash
# Create tag
scud init --tag {feature-name}

# For each phase in the plan, create tasks:
scud create --tag {tag} --title "Phase 1: {name}" --description "{from plan}"
scud create --tag {tag} --title "Phase 1.1: {subtask}" --depends 1

# Analyze complexity
scud analyze --tag {tag}

# Show waves
scud waves --tag {tag}
```

### Step 3: Generate Handoff
**Location**: `thoughts/shared/handoffs/plan/{YYYY-MM-DD}_{HH-MM}_{tag}.md`

Include:
- Plan document path
- SCUD tag and task summary
- Wave breakdown
- Architecture decisions
- Testing strategy

### Step 4: Present Summary
```
Planning complete!

📄 Plan: {path}
🏷️ SCUD Tag: {tag}
📊 Tasks: {N} tasks in {M} waves
📋 Handoff: {handoff-path}

Wave breakdown:
- Wave 1: {n} tasks - {description}
- Wave 2: {n} tasks - {description}

To continue in a new session:
/flow:implement {tag}
```
````

##### 3. `/flow:implement` Command
**File**: `.claude/commands/flow/implement.md`

````markdown
---
description: SCUD-aware implementation loop with wave execution
model: opus
---

# Flow Implement

You implement SCUD tasks using a wave-based loop until all tasks are complete.

## Initial Response

If a SCUD tag is provided:
1. Check `scud stats --tag {tag}` for current state
2. Load any existing handoff from `thoughts/shared/handoffs/plan/`
3. Read the plan document

If resuming (handoff path provided):
1. Read the implementation handoff
2. Verify current SCUD state matches handoff
3. Continue from last wave

## Implementation Loop

```
LOOP START
│
├─► Check Completion
│   scud stats --tag {tag}
│   └─► All done? → Exit with success
│
├─► Get Current Wave
│   scud waves --tag {tag}
│   scud next --tag {tag}
│   └─► No tasks? → Commit wave, next wave
│
├─► For Each Task in Wave:
│   │
│   ├─► 1. Claim Task
│   │   scud set-status {id} in-progress --tag {tag}
│   │
│   ├─► 2. Read Context
│   │   - Task details from scud show {id}
│   │   - Relevant plan phase
│   │   - Pattern examples from codebase
│   │
│   ├─► 3. Implement (spawn sub-agent if complex)
│   │   Use codebase-pattern-finder for consistency
│   │   Write code and tests
│   │
│   ├─► 4. Verify
│   │   Run: make check test (or configured command)
│   │   └─► Fail? → Fix (up to 3 attempts)
│   │   └─► Still fail? → Mark blocked with reason
│   │
│   └─► 5. Complete
│       scud set-status {id} done --tag {tag}
│
├─► Wave Complete
│   git add -A
│   git commit -m "feat({tag}): complete wave {N}"
│   Update progress
│
└─► Continue Loop
```

## Progress Reporting

After each wave:
```
Wave {N} Complete ✓

Tasks completed: {n}/{total}
Commits: {hash}

Remaining:
- Wave {N+1}: {m} tasks

Continuing...
```

## Completion

When all tasks done:
```
Implementation Complete! ✓

📊 Summary:
- Tasks: {N} completed
- Waves: {M} completed
- Commits: {list}

📋 Generating implementation handoff...

[Generate handoff to thoughts/shared/handoffs/implement/]

Next steps:
- Run /cl:describe_pr to create PR description
- Run /scud:retrospective to capture learnings
```

## Error Handling

If a task is blocked:
```
Task {id} blocked after 3 attempts.

Reason: {error details}

Options:
1. Mark blocked and continue with other tasks
2. Investigate with sub-agent
3. Pause for human intervention

Choice (or provide fix):
```
````

##### 4. `/flow:resume` Command
**File**: `.claude/commands/flow/resume.md`

````markdown
---
description: Resume from any handoff document
model: opus
---

# Flow Resume

Resume work from a handoff document.

## Process

1. **Read the handoff document**
2. **Determine phase**: research, plan, or implement
3. **Load context**:
   - For research handoff → prepare for planning
   - For plan handoff → prepare for implementation
   - For implement handoff → resume implementation
4. **Verify state**:
   - Git branch matches
   - SCUD state matches (if applicable)
   - Files mentioned still exist
5. **Present status and options**
6. **Continue with appropriate phase**

## Example Flow

```
Reading handoff: thoughts/shared/handoffs/plan/2025-12-29_15-30_auth-system.md

Handoff Type: Planning
Topic: Auth System
SCUD Tag: auth-system
Tasks: 8 total, 0 completed
Plan: thoughts/shared/plans/2025-12-29-auth-system.md

Verifying state...
✓ Branch matches: feature/auth-system
✓ SCUD tag exists with 8 pending tasks
✓ Plan document exists

Ready to begin implementation.

Continue with /flow:implement auth-system? (y/n)
```
````

##### 5. `/flow:status` Command
**File**: `.claude/commands/flow/status.md`

````markdown
---
description: Show current flow status across all phases
model: haiku
---

# Flow Status

Show the current state of the flow workflow.

## Check

1. Look for recent handoffs in `thoughts/shared/handoffs/`
2. Check for active SCUD tags
3. Check git status

## Output Format

```
Flow Workflow Status
═══════════════════════════════════════════════════

Active Flows:

🔬 Research
   └─ No active research handoffs

📋 Planning
   └─ auth-system (handoff: 2025-12-29_15-30)
      Plan: thoughts/shared/plans/2025-12-29-auth-system.md
      Ready for: /flow:implement auth-system

🔨 Implementation
   └─ No active implementations

Recent Handoffs:
- research/2025-12-28_10-00_api-design.md (complete → plan)
- plan/2025-12-28_14-30_api-design.md (complete → implement)
- implement/2025-12-28_18-00_api-design.md (complete)

Commands:
- /flow:research [topic] - Start new research
- /flow:plan [handoff]   - Create implementation plan
- /flow:implement [tag]  - Execute implementation
- /flow:resume [handoff] - Resume from handoff
```
````

#### Success Criteria

##### Automated Verification
- [ ] All command files are valid markdown
- [ ] Commands appear in Claude Code's command list

##### Manual Verification
- [ ] `/flow:research` generates research + handoff
- [ ] `/flow:plan` creates plan + SCUD tasks + handoff
- [ ] `/flow:implement` executes loop with wave commits
- [ ] `/flow:resume` successfully resumes from handoff
- [ ] `/flow:status` shows accurate state

---

### Phase 4: Sub-Agent Integration

#### Overview
Create specialized sub-agents for task implementation within the flow loop.

#### Changes Required

##### 1. Implementation Sub-Agent
**File**: `.claude/agents/flow/task-implementer.md`

```markdown
---
name: task-implementer
description: Implements a single SCUD task with verification
model: sonnet
---

# Task Implementer

You implement a single SCUD task.

## Input
- Task ID and description
- Plan context (relevant phase)
- Patterns to follow (from codebase-pattern-finder)

## Process
1. Understand the task requirements
2. Find similar implementations in codebase
3. Write the code
4. Write/update tests
5. Run verification
6. Report success or failure with details

## Output
Return structured result:
- success: boolean
- files_modified: list
- tests_added: list
- verification_output: string
- error_message: string (if failed)
```

##### 2. Task Context Gatherer
**File**: `.claude/agents/flow/task-context.md`

```markdown
---
name: task-context
description: Gathers context for implementing a SCUD task
model: haiku
---

# Task Context Gatherer

Gather all context needed for implementing a task.

## Input
- Task ID
- SCUD tag
- Plan document path

## Output
- Task description and details
- Relevant plan phase content
- Similar implementations found
- Files that will be modified
- Test files to update
```

#### Success Criteria

##### Automated Verification
- [ ] Agent definitions are valid

##### Manual Verification
- [ ] Task implementer successfully implements a simple task
- [ ] Context gatherer returns useful information

---

### Phase 5: GUI Integration

#### Overview
Update the GUI to show SCUD-aware loop progress with wave visualization.

#### Changes Required

##### 1. Update Loop State
**File**: `descartes/gui/src/loop_state.rs`

Add SCUD-specific fields:
```rust
pub struct LoopViewState {
    // Existing fields...

    // SCUD integration
    pub scud_tag: Option<String>,
    pub current_wave: u32,
    pub total_waves: u32,
    pub tasks_in_wave: Vec<TaskProgress>,
    pub wave_commits: Vec<String>,
}

pub struct TaskProgress {
    pub id: u32,
    pub title: String,
    pub status: TaskStatus,
    pub complexity: u32,
}
```

##### 2. Update Loop View
**File**: `descartes/gui/src/loop_view.rs`

Add wave progress visualization:
- Wave indicator (Wave 2/4)
- Task list for current wave with status
- Commit history per wave

#### Success Criteria

##### Automated Verification
- [ ] `cargo check -p descartes-gui` passes

##### Manual Verification
- [ ] GUI shows wave progress during SCUD loop
- [ ] Task statuses update in real-time

---

## SCUD Task Breakdown

Below is the task breakdown for implementation. Run these commands to set up:

```bash
# Initialize SCUD tag
scud init --tag flow-system

# Phase 1: Merge Loop Infrastructure
scud create --tag flow-system --title "Cherry-pick loop commit from local-master-backup" --complexity 3
scud create --tag flow-system --title "Resolve merge conflicts with blog docs" --depends 1 --complexity 2
scud create --tag flow-system --title "Verify cargo check passes for all crates" --depends 2 --complexity 1
scud create --tag flow-system --title "Verify loop tests pass" --depends 3 --complexity 1
scud create --tag flow-system --title "Test descartes loop CLI command" --depends 4 --complexity 2

# Phase 2: SCUD Loop Wrapper
scud create --tag flow-system --title "Create scud_loop.rs with ScudLoopConfig" --depends 5 --complexity 5
scud create --tag flow-system --title "Implement ScudStats and SCUD CLI integration" --depends 6 --complexity 3
scud create --tag flow-system --title "Implement wave-based execution logic" --depends 7 --complexity 5
scud create --tag flow-system --title "Add wave commit functionality" --depends 8 --complexity 2
scud create --tag flow-system --title "Write tests for ScudIterativeLoop" --depends 9 --complexity 3

# Phase 3: Flow Commands
scud create --tag flow-system --title "Create /flow:research command" --depends 5 --complexity 5
scud create --tag flow-system --title "Create /flow:plan command with SCUD task generation" --depends 11 --complexity 8
scud create --tag flow-system --title "Create /flow:implement command with loop integration" --depends 10,12 --complexity 8
scud create --tag flow-system --title "Create /flow:resume command" --depends 13 --complexity 5
scud create --tag flow-system --title "Create /flow:status command" --depends 14 --complexity 3

# Phase 4: Sub-Agents
scud create --tag flow-system --title "Create task-implementer sub-agent" --depends 13 --complexity 5
scud create --tag flow-system --title "Create task-context sub-agent" --depends 16 --complexity 3
scud create --tag flow-system --title "Integrate sub-agents with flow:implement" --depends 17 --complexity 5

# Phase 5: GUI Integration
scud create --tag flow-system --title "Update loop_state.rs with SCUD fields" --depends 10 --complexity 3
scud create --tag flow-system --title "Update loop_view.rs with wave visualization" --depends 19 --complexity 5

# Analyze and show waves
scud analyze --tag flow-system
scud waves --tag flow-system
```

---

## Testing Strategy

### Unit Tests
- `ScudLoopConfig` serialization
- `ScudStats` parsing from CLI output
- Wave completion detection
- Handoff document parsing

### Integration Tests
- End-to-end flow with echo command
- SCUD task progression
- Handoff generation and resume

### Manual Testing
1. Run `/flow:research` on a real topic
2. Continue with `/flow:plan` from handoff
3. Execute `/flow:implement` with SCUD tag
4. Verify wave commits
5. Test resume from partial implementation

---

## References

- Research: `thoughts/shared/research/2025-12-29-integrated-flow-prompt-patterns.md`
- Comparison: `thoughts/shared/research/2025-12-29-loop-implementation-comparison.md`
- Existing loop: `origin/local-master-backup:descartes/core/src/iterative_loop.rs`
- Flow executor: `descartes/core/src/flow_executor.rs`
