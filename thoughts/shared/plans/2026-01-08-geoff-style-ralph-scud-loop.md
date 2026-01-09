---
date: 2026-01-08
author: Claude Code (Opus 4.5)
status: draft
priority: high
tags: [implementation, ralph-loop, scud, flow]
---

# Implementation Plan: Geoff-Style Ralph Loop with SCUD Integration

## Overview

Implement a deterministic, externally-orchestrated loop system that executes SCUD tasks with fresh context per iteration, using SCUD task data + plans as the fixed spec allocation.

**Goals:**
- Fresh context per task (no cumulative history)
- SCUD-based completion detection (not promise tags)
- Flexible spec configuration (task + plan + custom files)
- Sub-agent spawning for task execution
- Slash command interface for Claude Code

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Slash Commands                            │
│  /ralph-wiggum:ralph-loop  /ralph-wiggum:cancel-ralph       │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                  descartes loop CLI                          │
│  descartes loop start --scud-tag X --spec-file Y            │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│              ScudIterativeLoop (Rust)                        │
│  - Loads spec from SCUD task + plan + custom files          │
│  - Spawns sub-agent per task with fresh context             │
│  - Tracks completion via scud stats                         │
│  - Commits after each wave                                  │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                   Sub-Agent (Claude)                         │
│  - Receives: spec + single task + verification command      │
│  - Executes: implement, test, report                        │
│  - Exits: success or blocked with reason                    │
└─────────────────────────────────────────────────────────────┘
```

## SCUD Task Mapping

This plan maps to SCUD tag `ralph` tasks as follows:

| SCUD Task | Plan Section | Description |
|-----------|--------------|-------------|
| 1 | Phase 1.1 | Add LoopSpecConfig struct |
| 2 | Phase 1.2 | Integrate into ScudLoopConfig |
| 3.1 | Phase 1.3a | Helper methods + token warning |
| 3.2 | Phase 1.3b | build_task_spec method |
| 4.1 | Phase 2.1 | TaskExecutionResult enum |
| 4.2 | Phase 2.2 | Sub-agent execution methods |
| 5 | Phase 2.3 | Update execute_task and loop |
| 6 | Phase 3.1 | CLI SCUD options |
| 7 | Phase 3.2 | Route to ScudIterativeLoop |
| 8 | Phase 4.1 | ralph-loop slash command |
| 9 | Phase 4.2-4.3 | cancel-ralph and help commands |
| 10.1 | Phase 5.1 | Unit and integration tests |
| 10.2 | Phase 5.2 | Documentation updates |

---

## Phase 1: Spec Configuration System (SCUD Tasks 1, 2, 3)

**Goal:** Add flexible spec loading to ScudLoopConfig

### 1.1 Add LoopSpecConfig struct (SCUD Task 1)

**File:** `descartes/core/src/scud_loop.rs`

```rust
/// Configuration for spec/context loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopSpecConfig {
    /// Include SCUD task details in spec (default: true)
    #[serde(default = "default_true")]
    pub include_task: bool,

    /// Include relevant plan section (default: true)
    #[serde(default = "default_true")]
    pub include_plan_section: bool,

    /// Additional spec files to include
    #[serde(default)]
    pub additional_specs: Vec<PathBuf>,

    /// Max tokens for combined spec (soft limit, will warn if exceeded)
    #[serde(default)]
    pub max_spec_tokens: Option<usize>,

    /// Custom template for combining specs
    /// Placeholders: {task}, {plan}, {custom}, {verification}
    #[serde(default)]
    pub spec_template: Option<String>,
}

impl Default for LoopSpecConfig {
    fn default() -> Self {
        Self {
            include_task: true,
            include_plan_section: true,
            additional_specs: Vec::new(),
            max_spec_tokens: Some(5000),
            spec_template: None,
        }
    }
}
```

### 1.2 Add spec field to ScudLoopConfig (SCUD Task 2)

```rust
pub struct ScudLoopConfig {
    // ... existing fields ...

    /// Spec configuration for context loading
    #[serde(default)]
    pub spec: LoopSpecConfig,
}
```

### 1.3 Implement spec builder (SCUD Tasks 3.1, 3.2)

```rust
impl ScudIterativeLoop {
    /// Build the spec/context for a task
    fn build_task_spec(&self, task: &LoopTask) -> Result<String> {
        let mut parts: Vec<String> = Vec::new();

        // 1. Task details from SCUD
        if self.config.spec.include_task {
            parts.push(self.format_task_spec(task));
        }

        // 2. Plan section (find relevant section from plan doc)
        if self.config.spec.include_plan_section {
            if let Some(ref plan_path) = self.config.plan_path {
                if let Ok(plan_section) = self.extract_plan_section(plan_path, task.id) {
                    parts.push(plan_section);
                }
            }
        }

        // 3. Additional spec files
        for spec_path in &self.config.spec.additional_specs {
            if let Ok(content) = std::fs::read_to_string(spec_path) {
                parts.push(format!("## {}\n\n{}",
                    spec_path.file_name().unwrap_or_default().to_string_lossy(),
                    content
                ));
            }
        }

        // 4. Apply template or default formatting
        let spec = if let Some(ref template) = self.config.spec.spec_template {
            self.apply_spec_template(template, task, &parts)
        } else {
            parts.join("\n\n---\n\n")
        };

        // 5. Warn if exceeds token budget
        if let Some(max_tokens) = self.config.spec.max_spec_tokens {
            let estimated_tokens = spec.len() / 4; // rough estimate
            if estimated_tokens > max_tokens {
                warn!(
                    "Spec exceeds token budget: ~{} tokens (max: {})",
                    estimated_tokens, max_tokens
                );
            }
        }

        Ok(spec)
    }

    fn format_task_spec(&self, task: &LoopTask) -> String {
        format!(
            "# Current Task\n\n\
            **ID:** {}\n\
            **Title:** {}\n\
            **Complexity:** {}\n\n\
            ## Description\n\n{}\n\n\
            ## Test Strategy\n\n{}\n\n\
            ## Dependencies\n\n{}",
            task.id,
            task.title,
            task.complexity,
            task.description.as_deref().unwrap_or("No description"),
            task.test_strategy.as_deref().unwrap_or("No test strategy defined"),
            if task.depends_on.is_empty() {
                "None".to_string()
            } else {
                task.depends_on.iter().map(|d| format!("- Task {}", d)).collect::<Vec<_>>().join("\n")
            }
        )
    }

    fn extract_plan_section(&self, plan_path: &PathBuf, task_id: u32) -> Result<String> {
        let content = std::fs::read_to_string(plan_path)?;

        // Try to find section matching task ID
        // Look for patterns like "## Task 5:" or "### 5." or "#### Task 5"
        let patterns = [
            format!("## Task {}:", task_id),
            format!("### {}.", task_id),
            format!("#### Task {}", task_id),
            format!("## {}", task_id),
        ];

        for pattern in &patterns {
            if let Some(start) = content.find(pattern) {
                // Find next section header or end
                let section_content = &content[start..];
                let end = section_content[pattern.len()..]
                    .find("\n## ")
                    .or_else(|| section_content[pattern.len()..].find("\n### "))
                    .map(|e| e + pattern.len())
                    .unwrap_or(section_content.len());

                return Ok(format!(
                    "# Relevant Plan Section\n\n{}",
                    &section_content[..end].trim()
                ));
            }
        }

        // Fallback: return truncated plan
        Ok(format!(
            "# Implementation Plan (truncated)\n\n{}...",
            &content.chars().take(2000).collect::<String>()
        ))
    }
}
```

### 1.4 Deliverables

- [ ] `LoopSpecConfig` struct in `scud_loop.rs`
- [ ] `build_task_spec()` method
- [ ] `format_task_spec()` helper
- [ ] `extract_plan_section()` helper
- [ ] Unit tests for spec building

---

## Phase 2: Sub-Agent Task Execution (SCUD Tasks 4, 5)

**Goal:** Replace placeholder `execute_task()` with actual sub-agent spawning

### 2.1 Create task-implementer prompt template

**File:** `descartes/agents/task-implementer.md` (already exists, verify/update)

```markdown
# Task Implementer Agent

You are implementing a single SCUD task. Your context contains:
- The task specification
- Relevant plan details
- Project architecture context (if provided)

## Your Mission

Implement the task described below. Follow existing code patterns.

## Rules

1. **Focus**: Only implement what's specified in the task
2. **Verify**: Run the verification command before completing
3. **Report**: If blocked, explain why clearly
4. **Exit**: Complete the task and exit - do not continue to other work

## Task Spec

{spec}

## Verification Command

```bash
{verification_command}
```

## Instructions

1. Read the task carefully
2. Explore relevant code if needed
3. Implement the solution
4. Run verification: `{verification_command}`
5. If tests pass, you're done
6. If tests fail after 3 attempts, report as blocked with reason
```

### 2.2 Implement sub-agent spawning (SCUD Tasks 4.1, 4.2)

**File:** `descartes/core/src/scud_loop.rs`

```rust
impl ScudIterativeLoop {
    /// Execute a single task by spawning a sub-agent
    async fn execute_task(&self, task: &LoopTask) -> Result<TaskExecutionResult> {
        info!("Spawning sub-agent for task {}: {}", task.id, task.title);

        // 1. Build spec with fresh context
        let spec = self.build_task_spec(task)?;

        // 2. Build the prompt from template
        let prompt = self.build_task_prompt(&spec, task)?;

        // 3. Spawn Claude with the prompt
        let output = self.spawn_claude_agent(&prompt).await?;

        // 4. Parse result
        let result = self.parse_task_result(&output, task)?;

        Ok(result)
    }

    fn build_task_prompt(&self, spec: &str, task: &LoopTask) -> Result<String> {
        let verification = self.config.verification_command
            .as_deref()
            .unwrap_or("echo 'No verification command configured'");

        Ok(format!(
            "You are implementing SCUD task {} for tag '{}'.\n\n\
            ## Spec\n\n{}\n\n\
            ## Verification\n\n\
            After implementation, run:\n```bash\n{}\n```\n\n\
            ## Instructions\n\n\
            1. Implement the task\n\
            2. Run verification\n\
            3. If verification passes, output: TASK_COMPLETE\n\
            4. If blocked after 3 attempts, output: TASK_BLOCKED: <reason>\n\n\
            Begin implementation.",
            task.id,
            self.config.tag,
            spec,
            verification
        ))
    }

    async fn spawn_claude_agent(&self, prompt: &str) -> Result<String> {
        use tokio::process::Command;

        let mut cmd = Command::new("claude");
        cmd.args(["-p", "--output-format", "text"])
            .arg(prompt)
            .current_dir(&self.config.working_directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = cmd.output().await
            .context("Failed to spawn Claude agent")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            debug!("Agent stderr: {}", stderr);
        }

        Ok(stdout)
    }

    fn parse_task_result(&self, output: &str, task: &LoopTask) -> Result<TaskExecutionResult> {
        if output.contains("TASK_COMPLETE") {
            Ok(TaskExecutionResult::Success)
        } else if output.contains("TASK_BLOCKED:") {
            let reason = output
                .lines()
                .find(|l| l.contains("TASK_BLOCKED:"))
                .map(|l| l.replace("TASK_BLOCKED:", "").trim().to_string())
                .unwrap_or_else(|| "Unknown reason".to_string());
            Ok(TaskExecutionResult::Blocked(reason))
        } else {
            // No explicit signal - check if verification would pass
            Ok(TaskExecutionResult::Unknown)
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskExecutionResult {
    Success,
    Blocked(String),
    Unknown,
}
```

### 2.3 Update execute() loop to use new execution (SCUD Task 5)

```rust
// In execute() method, replace the placeholder call:

let result = self.execute_task(&task).await?;

match result {
    TaskExecutionResult::Success => {
        // Run verification
        let verified = self.run_verification()?;
        if verified {
            self.update_task_status(task.id, "done")?;
            self.state.tasks_completed += 1;
            wave_task_ids.push(task.id);
            info!("Task {} completed successfully", task.id);
        } else {
            // Verification failed despite agent claiming success
            self.handle_verification_failure(&task, 1).await?;
        }
    }
    TaskExecutionResult::Blocked(reason) => {
        self.update_task_status(task.id, "blocked")?;
        self.state.blocked_tasks.push(BlockedTask {
            task_id: task.id,
            title: task.title.clone(),
            reason,
            attempts: 1,
            blocked_at: Utc::now(),
        });
        warn!("Task {} blocked", task.id);
    }
    TaskExecutionResult::Unknown => {
        // Agent didn't signal clearly - run verification to decide
        let verified = self.run_verification()?;
        if verified {
            self.update_task_status(task.id, "done")?;
            self.state.tasks_completed += 1;
            wave_task_ids.push(task.id);
        } else {
            self.handle_verification_failure(&task, 1).await?;
        }
    }
}
```

### 2.4 Deliverables

- [ ] `TaskExecutionResult` enum
- [ ] `build_task_prompt()` method
- [ ] `spawn_claude_agent()` method
- [ ] `parse_task_result()` method
- [ ] Update `execute_task()` to use sub-agent
- [ ] Update main `execute()` loop
- [ ] Integration tests with mock agent

---

## Phase 3: CLI Integration (SCUD Tasks 6, 7)

**Goal:** Expose ScudIterativeLoop via `descartes loop` CLI

### 3.1 Add SCUD options to loop command (SCUD Task 6)

**File:** `descartes/cli/src/commands/loop_cmd.rs`

```rust
#[derive(Parser)]
pub struct LoopStartArgs {
    // ... existing args ...

    /// Use SCUD-based completion (provide tag name)
    #[arg(long)]
    pub scud_tag: Option<String>,

    /// Path to implementation plan document
    #[arg(long)]
    pub plan: Option<PathBuf>,

    /// Additional spec files to include in context
    #[arg(long, action = ArgAction::Append)]
    pub spec_file: Vec<PathBuf>,

    /// Max tokens for spec section
    #[arg(long, default_value = "5000")]
    pub max_spec_tokens: usize,

    /// Verification command (default: cargo check && cargo test)
    #[arg(long)]
    pub verify: Option<String>,
}
```

### 3.2 Route to ScudIterativeLoop when --scud-tag provided (SCUD Task 7)

```rust
pub async fn execute_start(args: LoopStartArgs) -> Result<()> {
    if let Some(tag) = args.scud_tag {
        // Use SCUD-aware loop
        let config = ScudLoopConfig {
            tag,
            plan_path: args.plan,
            working_directory: args.working_dir.unwrap_or_else(|| PathBuf::from(".")),
            verification_command: args.verify.or(Some("cargo check && cargo test".to_string())),
            spec: LoopSpecConfig {
                additional_specs: args.spec_file,
                max_spec_tokens: Some(args.max_spec_tokens),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut loop_exec = ScudIterativeLoop::new(config)?;
        let result = loop_exec.execute().await?;

        println!("Loop completed: {:?}", result.exit_reason);
    } else {
        // Use generic iterative loop (existing behavior)
        // ...
    }
    Ok(())
}
```

### 3.3 Example CLI usage

```bash
# Start SCUD loop for a tag
descartes loop start \
    --scud-tag my-feature \
    --plan ./thoughts/shared/plans/my-feature.md \
    --spec-file ./ARCHITECTURE.md \
    --verify "cargo check && cargo test"

# Resume interrupted loop
descartes loop resume

# Check status
descartes loop status

# Cancel
descartes loop cancel
```

### 3.4 Deliverables

- [ ] Add `--scud-tag` option to `loop start`
- [ ] Add `--plan` option
- [ ] Add `--spec-file` option (repeatable)
- [ ] Add `--max-spec-tokens` option
- [ ] Route to `ScudIterativeLoop` when SCUD tag provided
- [ ] Update help text and examples

---

## Phase 4: Slash Command Wrappers (SCUD Tasks 8, 9)

**Goal:** Create Claude Code slash commands for easy invocation

### 4.1 Create ralph-wiggum directory

```bash
mkdir -p .claude/commands/ralph-wiggum
```

### 4.2 ralph-loop.md (SCUD Task 8)

**File:** `.claude/commands/ralph-wiggum/ralph-loop.md`

```markdown
---
description: Start Ralph Wiggum loop for SCUD tag
---

# Ralph Loop

Start a Geoff-style iterative loop for implementing SCUD tasks.

## Arguments

$ARGUMENTS should be a SCUD tag name, optionally followed by flags:
- `--plan <path>` - Path to implementation plan
- `--spec <path>` - Additional spec file (can repeat)
- `--max-iterations <n>` - Safety limit (default: 100)

## Execution

1. Parse arguments to extract tag and options
2. Verify SCUD tag exists: `scud stats --tag {tag}`
3. Start the loop via Descartes CLI:

```bash
descartes loop start \
    --scud-tag {tag} \
    --plan {plan_path} \
    --spec-file {spec_files...} \
    --verify "cargo check && cargo test"
```

4. Monitor progress and report status

## Example Usage

```
/ralph-wiggum:ralph-loop my-feature --plan thoughts/shared/plans/my-feature.md
```

## Output Format

```
Starting Ralph loop for tag: {tag}

📊 Initial Status:
- Tasks: {pending}/{total}
- Waves: {total_waves}

🔄 Loop running...
- Use /ralph-wiggum:cancel-ralph to stop
- Progress saved to .scud/loop-state.json

Wave 1: Implementing {n} tasks...
  ✓ Task 1: {title}
  ✓ Task 2: {title}
  ✗ Task 3: {title} (blocked: {reason})

Wave 1 complete. Committed: {hash}

...continues until all tasks done or all blocked...
```
```

### 4.3 cancel-ralph.md (SCUD Task 9)

**File:** `.claude/commands/ralph-wiggum/cancel-ralph.md`

```markdown
---
description: Cancel active Ralph Wiggum loop
---

# Cancel Ralph Loop

Stop an active Ralph loop and preserve state for later resume.

## Execution

1. Check for active loop: `descartes loop status`
2. If active, cancel: `descartes loop cancel`
3. Report final state

## Output

```
Cancelling Ralph loop...

📊 Final Status:
- Tag: {tag}
- Tasks completed: {done}/{total}
- Waves completed: {waves}
- State saved to: .scud/loop-state.json

To resume later: /ralph-wiggum:ralph-loop {tag} --resume
```
```

### 4.4 help.md (SCUD Task 9)

**File:** `.claude/commands/ralph-wiggum/help.md`

```markdown
---
description: Explain Ralph Wiggum technique and commands
---

# Ralph Wiggum Help

## What is Ralph?

The Ralph Wiggum technique (created by Geoffrey Huntley) is an iterative AI development loop:

```bash
while :; do cat PROMPT.md | claude-code; done
```

**Key principles:**
- Same spec fed each iteration (fresh context)
- Agent sees previous work in files/git
- External orchestration (not model-managed)
- Deterministic failures enable systematic improvement

## SCUD Integration

This implementation uses SCUD tasks as the "fixed spec":
- Task description = objective
- Plan section = detailed spec
- Test strategy = success criteria
- Completion via SCUD stats (not promise tags)

## Available Commands

### /ralph-wiggum:ralph-loop <tag> [options]

Start loop for SCUD tag:
```
/ralph-wiggum:ralph-loop my-feature --plan ./plan.md
```

Options:
- `--plan <path>` - Implementation plan document
- `--spec <path>` - Additional spec files
- `--max-iterations <n>` - Safety limit

### /ralph-wiggum:cancel-ralph

Stop active loop, preserve state for resume.

### /ralph-wiggum:help

Show this help.

## Learn More

- Original technique: https://ghuntley.com/ralph/
- Research doc: thoughts/shared/research/2026-01-08-ralph-loop-scud-integration.md
```

### 4.5 Deliverables

- [ ] Create `.claude/commands/ralph-wiggum/` directory
- [ ] `ralph-loop.md` command
- [ ] `cancel-ralph.md` command
- [ ] `help.md` command
- [ ] Test commands work in Claude Code

---

## Phase 5: Testing & Documentation (SCUD Task 10)

### 5.1 Unit Tests (SCUD Task 10.1)

**File:** `descartes/core/src/scud_loop.rs` (add to existing tests module)

```rust
#[cfg(test)]
mod spec_tests {
    use super::*;

    #[test]
    fn test_spec_config_defaults() {
        let config = LoopSpecConfig::default();
        assert!(config.include_task);
        assert!(config.include_plan_section);
        assert_eq!(config.max_spec_tokens, Some(5000));
    }

    #[test]
    fn test_format_task_spec() {
        let task = LoopTask {
            id: 1,
            title: "Test task".to_string(),
            description: Some("Do the thing".to_string()),
            status: "pending".to_string(),
            complexity: 3,
            depends_on: vec![],
            test_strategy: Some("Unit tests".to_string()),
        };

        let loop_exec = create_test_loop();
        let spec = loop_exec.format_task_spec(&task);

        assert!(spec.contains("Test task"));
        assert!(spec.contains("Do the thing"));
        assert!(spec.contains("Unit tests"));
    }

    #[test]
    fn test_build_task_prompt() {
        let task = LoopTask { /* ... */ };
        let loop_exec = create_test_loop();
        let spec = "Test spec content";

        let prompt = loop_exec.build_task_prompt(spec, &task).unwrap();

        assert!(prompt.contains("Test spec content"));
        assert!(prompt.contains("TASK_COMPLETE"));
        assert!(prompt.contains("TASK_BLOCKED"));
    }
}
```

### 5.2 Integration Test (SCUD Task 10.1 cont'd)

**File:** `descartes/core/tests/scud_loop_integration.rs`

```rust
#[tokio::test]
async fn test_scud_loop_with_mock_tasks() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create mock SCUD tasks file
    let tasks_dir = temp_dir.path().join(".scud/tasks");
    std::fs::create_dir_all(&tasks_dir).unwrap();

    let tasks_json = json!({
        "tasks": [
            {"id": 1, "title": "Task 1", "status": "pending", "complexity": 2},
            {"id": 2, "title": "Task 2", "status": "pending", "complexity": 3, "depends_on": [1]},
        ],
        "waves": [
            {"number": 1, "task_ids": [1]},
            {"number": 2, "task_ids": [2]},
        ]
    });

    std::fs::write(
        tasks_dir.join("test-tag.json"),
        serde_json::to_string_pretty(&tasks_json).unwrap()
    ).unwrap();

    let config = ScudLoopConfig {
        tag: "test-tag".to_string(),
        working_directory: temp_dir.path().to_path_buf(),
        max_total_iterations: 5,
        use_sub_agents: false, // Use mock execution
        ..Default::default()
    };

    let mut loop_exec = ScudIterativeLoop::new(config).unwrap();
    let result = loop_exec.execute().await.unwrap();

    assert!(result.completion_promise_found);
}
```

### 5.3 Documentation Updates (SCUD Task 10.2)

- [ ] Update `descartes/docs/blog/12-iterative-loops.md` with SCUD integration
- [ ] Add example to `descartes/README.md`
- [ ] Update CLI help text

### 5.4 Deliverables

- [ ] Unit tests for spec building
- [ ] Unit tests for task execution parsing
- [ ] Integration test with mock SCUD tasks
- [ ] Documentation updates

---

## Implementation Order (Aligned with SCUD Dependencies)

The SCUD graph shows CLI options (Task 6) can start in parallel, but **CLI routing (Task 7) must wait for Task 2** since it uses `LoopSpecConfig` and the `spec` field:

```
SCUD Task Dependencies:

    1: LoopSpecConfig struct
           │
           ▼
    2: Integrate into ScudLoopConfig ◄───────┐
           │                                 │
           ▼                                 │
    3: build_task_spec          6: CLI options (parallel)
     ├─3.1 helpers                           │
     └─3.2 main                              │
           │                                 │
           ▼                                 ▼
    4: Sub-agent methods        7: Route CLI (needs 2 + 6)
     ├─4.1 enum                              │
     └─4.2 methods                           ▼
           │                    8: ralph-loop slash cmd
           ▼                                 │
    5: Update execute loop                   ▼
           │                    9: cancel-ralph, help
           │                                 │
           └──────────────┬──────────────────┘
                          ▼
               10: Tests & Documentation
                  ├─10.1 Tests
                  └─10.2 Docs
```

**Execution Strategy:**

| Phase | Tasks | Notes |
|-------|-------|-------|
| Wave 1 | 1, 6 | Can run in parallel |
| Wave 2 | 2 | Depends on 1 |
| Wave 3 | 3, 7 | 3 depends on 2; 7 depends on 2+6 |
| Wave 4 | 4, 8 | 4 depends on 3; 8 depends on 7 |
| Wave 5 | 5, 9 | 5 depends on 4; 9 depends on 8 |
| Wave 6 | 10 | Final integration & docs |

**Key dependency:** `7 -> 2` ensures CLI routing code can compile (uses `LoopSpecConfig` and `spec` field)

## Success Criteria

1. **Spec Loading:** Can build task spec from SCUD + plan + custom files
2. **Sub-Agent Execution:** Claude spawned per task with fresh context
3. **Completion Detection:** Loop exits when all SCUD tasks done (no promise reliance)
4. **CLI Works:** `descartes loop start --scud-tag X` functions correctly
5. **Slash Commands:** `/ralph-wiggum:ralph-loop X` works in Claude Code
6. **Tests Pass:** All unit and integration tests green

## Open Design Decisions

1. **Retry behavior:** How many times to retry a task before marking blocked?
   - Current: 3 attempts (configurable via `max_iterations_per_task`)

2. **Wave parallelism:** Execute tasks in a wave sequentially or in parallel?
   - Current: Sequential (simpler, avoids conflicts)
   - Future: Could add `--parallel` flag

3. **Spec template defaults:** What should the default template look like?
   - Current: Simple concatenation with `---` separators
   - Could add structured formats (XML, markdown sections)

4. **State file location:** `.scud/loop-state.json` vs `.descartes/loop-state.json`?
   - Recommendation: `.scud/loop-state.json` since it's SCUD-related
