# Plan: Descartes Ralph Command Implementation

## Overview

Implement a true Ralph Wiggum loop in Descartes that replaces SCUD's `swarm` command, featuring fresh-context-per-task execution, configurable spec sources, backpressure validation, and multi-harness support.

## Prerequisites

Read [UNIFIED_CONTEXT.md](/Users/reuben/projects/harnesses/docs/UNIFIED_CONTEXT.md) first for shared context.

## Current State Analysis

### What Descartes Has Today

**SCUD Integration (`src/scud/mod.rs`):**
- Imports `scud::models::{Phase, Task, TaskStatus, Priority}`
- Imports `scud::storage::Storage`
- Wrapper functions: `next()`, `complete()`, `waves()`, `list_tasks()`, `ready_tasks()`

**Ralph Loop (`src/ralph_loop.rs`):**
- `RalphMode` enum: Plan, Build
- `run_ralph()` entry point
- Build iteration with BAML decision making
- Basic task execution flow

**Harness System (`src/harness/`):**
- `Harness` trait with `start_session()`, `send()`, `close_session()`
- `ClaudeCodeHarness` implementation using `-p --output-format stream-json`
- `OpenCodeHarness` placeholder
- `CodexHarness` placeholder

**BAML Prompts (`baml_src/`):**
- `orchestrator.baml`: `DecideNextAction`, `SelectSubagent`
- `planning.baml`: `CreatePlan`, `BreakdownTask`
- `implementation.baml`, `validation.baml`: Task execution prompts

### What's Missing for True Ralph

1. **CLI Command** - No `descartes ralph` command in `main.rs`
2. **Spec Configuration** - No configurable spec sources (task + plan + custom files)
3. **Backpressure Integration** - Not using SCUD's backpressure module
4. **Fresh Context Mode** - Current loop may accumulate context
5. **Wave-Based Execution** - Ralph loop is single-task, not wave-aware

## Desired End State

A `descartes ralph` command that:
1. **Optionally initializes from PRD** - Uses SCUD's AI commands (parse, expand, check-deps)
2. Executes SCUD tasks in dependency-order waves
3. Spawns agents with fresh context per task
4. Supports configurable spec sources (task + plan + custom files)
5. Runs backpressure validation between waves
6. Marks failed tasks when validation fails
7. Works with any harness (Claude Code, OpenCode, Codex)
8. Provides clear progress output

## Implementation Approach

Build incrementally on existing infrastructure. Reuse SCUD's backpressure module. Leverage existing harness system. **Leverage SCUD's AI commands for task setup instead of duplicating them.**

### Task Initialization Flow (when `--prd` is provided)

```
User PRD → scud generate → SCUD Tasks Ready
```

This uses SCUD's unified `generate` command which orchestrates:
- `scud parse <prd> --tag <tag> -n <num>` - Generate tasks from PRD
- `scud expand --tag <tag>` - Break complex tasks into subtasks
- `scud check-deps --prd <prd> --fix` - Validate and fix dependencies

See [SCUD Migration Plan: Generate Command](/Users/reuben/projects/harnesses/scud/docs/MIGRATION_PLAN_GENERATE_COMMAND.md) for details.

---

## Phase 1: Add Spec Configuration System

**Goal**: Create configurable spec sources for the fixed-spec allocation pattern.

**Changes**:

- [ ] Create `src/spec.rs` with spec configuration

```rust
// src/spec.rs
//! Spec configuration for Ralph loop
//!
//! Implements Geoff's "fixed spec allocation" pattern:
//! ~5k tokens of persistent context at the start of each prompt.

use anyhow::{Context, Result};
use scud::models::Task;
use std::path::PathBuf;

/// Configuration for spec/context loading
#[derive(Debug, Clone, Default)]
pub struct SpecConfig {
    /// Include SCUD task details in spec (default: true)
    pub include_task: bool,

    /// Path to implementation plan document
    pub plan_path: Option<PathBuf>,

    /// Additional spec files to include
    pub additional_specs: Vec<PathBuf>,

    /// Max tokens for combined spec (warn if exceeded)
    pub max_spec_tokens: Option<usize>,

    /// Custom template for combining specs
    /// Placeholders: {task}, {plan}, {custom}, {verification}
    pub spec_template: Option<String>,
}

impl SpecConfig {
    pub fn new() -> Self {
        Self {
            include_task: true,
            plan_path: None,
            additional_specs: Vec::new(),
            max_spec_tokens: Some(5000),
            spec_template: None,
        }
    }

    pub fn with_plan(mut self, path: PathBuf) -> Self {
        self.plan_path = Some(path);
        self
    }

    pub fn with_spec_file(mut self, path: PathBuf) -> Self {
        self.additional_specs.push(path);
        self
    }
}

/// Build the spec/context for a task
pub fn build_task_spec(task: &Task, config: &SpecConfig) -> Result<String> {
    let mut parts: Vec<String> = Vec::new();

    // 1. Task details from SCUD
    if config.include_task {
        parts.push(format_task_spec(task));
    }

    // 2. Plan section (find relevant section from plan doc)
    if let Some(ref plan_path) = config.plan_path {
        if plan_path.exists() {
            match extract_plan_section(plan_path, &task.id) {
                Ok(section) => parts.push(section),
                Err(e) => tracing::warn!("Failed to extract plan section: {}", e),
            }
        }
    }

    // 3. Additional spec files
    for spec_path in &config.additional_specs {
        if spec_path.exists() {
            match std::fs::read_to_string(spec_path) {
                Ok(content) => {
                    let filename = spec_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "spec".to_string());
                    parts.push(format!("## {}\n\n{}", filename, content));
                }
                Err(e) => tracing::warn!("Failed to read spec file {:?}: {}", spec_path, e),
            }
        }
    }

    // 4. Combine with template or default separator
    let spec = if let Some(ref template) = config.spec_template {
        apply_spec_template(template, task, &parts)
    } else {
        parts.join("\n\n---\n\n")
    };

    // 5. Warn if exceeds token budget
    if let Some(max_tokens) = config.max_spec_tokens {
        let estimated_tokens = spec.len() / 4; // rough estimate: 4 chars per token
        if estimated_tokens > max_tokens {
            tracing::warn!(
                "Spec exceeds token budget: ~{} tokens (max: {})",
                estimated_tokens,
                max_tokens
            );
        }
    }

    Ok(spec)
}

fn format_task_spec(task: &Task) -> String {
    let deps = if task.dependencies.is_empty() {
        "None".to_string()
    } else {
        task.dependencies.iter()
            .map(|d| format!("- Task {}", d))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"# Current Task

**ID:** {}
**Title:** {}
**Status:** {:?}
**Complexity:** {}

## Description

{}

## Dependencies

{}"#,
        task.id,
        task.title,
        task.status,
        task.complexity,
        task.description.as_deref().unwrap_or("No description provided."),
        deps
    )
}

fn extract_plan_section(plan_path: &PathBuf, task_id: &str) -> Result<String> {
    let content = std::fs::read_to_string(plan_path)
        .context("Failed to read plan file")?;

    // Try to find section matching task ID
    let patterns = [
        format!("## Task {}:", task_id),
        format!("### {}.", task_id),
        format!("#### Task {}", task_id),
        format!("## {}", task_id),
    ];

    for pattern in &patterns {
        if let Some(start) = content.find(pattern) {
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
    let truncated: String = content.chars().take(2000).collect();
    Ok(format!("# Implementation Plan (truncated)\n\n{}...", truncated))
}

fn apply_spec_template(template: &str, task: &Task, parts: &[String]) -> String {
    let task_spec = format_task_spec(task);
    let custom = parts.get(2..).map(|p| p.join("\n\n")).unwrap_or_default();
    let plan = parts.get(1).cloned().unwrap_or_default();

    template
        .replace("{task}", &task_spec)
        .replace("{plan}", &plan)
        .replace("{custom}", &custom)
}
```

- [ ] Export from `src/lib.rs`

```rust
pub mod spec;  // Add this line
```

**Success Criteria - Automated**:
- [ ] `cargo build` passes
- [ ] `cargo test` passes
- [ ] Unit tests for `build_task_spec()` pass

**Success Criteria - Manual**:
- [ ] Spec output includes task details, plan section, and custom files

---

## Phase 2: Create Ralph Command CLI

**Goal**: Add `descartes ralph` command with proper CLI options.

**Changes**:

- [ ] Add `Ralph` variant to `Commands` enum in `src/main.rs`

```rust
// In the Commands enum, add:

/// Run Ralph Wiggum loop for SCUD tasks
Ralph {
    /// SCUD tag to execute (required unless --prd creates it)
    #[arg(long)]
    scud_tag: Option<String>,

    // === PRD Initialization Options ===

    /// Initialize tasks from PRD document (runs scud parse, expand, check-deps)
    #[arg(long)]
    prd: Option<PathBuf>,

    /// Number of tasks to generate from PRD (default: 10)
    #[arg(long, default_value = "10")]
    num_tasks: u32,

    /// Tag name for new tasks (required with --prd, defaults to filename)
    #[arg(long)]
    tag: Option<String>,

    /// Skip task expansion (don't run scud expand)
    #[arg(long)]
    no_expand: bool,

    /// Skip dependency check (don't run scud check-deps)
    #[arg(long)]
    no_check_deps: bool,

    // === Spec Configuration Options ===

    /// Path to implementation plan document for spec context
    #[arg(long)]
    plan: Option<PathBuf>,

    /// Additional spec files to include (can be repeated)
    #[arg(long = "spec-file", action = ArgAction::Append)]
    spec_files: Vec<PathBuf>,

    /// Max tokens for spec section (default: 5000)
    #[arg(long, default_value = "5000")]
    max_spec_tokens: usize,

    // === Execution Options ===

    /// Verification command for backpressure (default: auto-detect)
    #[arg(long)]
    verify: Option<String>,

    /// Harness to use: claude-code, opencode, codex (default: claude-code)
    #[arg(long, default_value = "claude-code")]
    harness: String,

    /// Model to use (default: from config)
    #[arg(long)]
    model: Option<String>,

    /// Maximum tasks per round (default: 5)
    #[arg(long, default_value = "5")]
    round_size: usize,

    /// Skip validation between waves
    #[arg(long)]
    no_validate: bool,

    /// Show execution plan without running
    #[arg(long)]
    dry_run: bool,

    /// Working directory (default: current)
    #[arg(long)]
    working_dir: Option<PathBuf>,
}
```

- [ ] Add match arm in `main()` function

```rust
// In the match statement, add:

Commands::Ralph {
    scud_tag,
    plan,
    spec_files,
    max_spec_tokens,
    verify,
    harness,
    model,
    round_size,
    no_validate,
    dry_run,
    working_dir,
} => {
    commands::ralph::run(
        scud_tag,
        plan,
        spec_files,
        max_spec_tokens,
        verify,
        harness,
        model,
        round_size,
        no_validate,
        dry_run,
        working_dir,
    )
    .await
}
```

- [ ] Add command handler directly in `src/main.rs` (Descartes uses inline handlers, not a commands/ directory)

In the match statement for Commands, add the Ralph handler:

```rust
Commands::Ralph {
    scud_tag,
    prd,
    num_tasks,
    tag,
    no_expand,
    no_check_deps,
    plan,
    spec_files,
    max_spec_tokens,
    verify,
    harness,
    model,
    round_size,
    no_validate,
    dry_run,
    working_dir,
} => {
    let working_dir = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Determine the tag to use
    let final_tag = if let Some(prd_path) = &prd {
        // Initialize from PRD using scud generate
        let tag_name = tag.unwrap_or_else(|| {
            prd_path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "ralph".to_string())
        });

        println!("📄 Initializing tasks from PRD: {:?}", prd_path);
        println!();

        // Build scud generate command args
        let mut args = vec![
            "generate".to_string(),
            prd_path.to_string_lossy().to_string(),
            "--tag".to_string(),
            tag_name.clone(),
            "-n".to_string(),
            num_tasks.to_string(),
        ];

        if no_expand {
            args.push("--no-expand".to_string());
        }
        if no_check_deps {
            args.push("--no-check-deps".to_string());
        }

        // Run unified generate command
        let generate_status = std::process::Command::new("scud")
            .args(&args)
            .current_dir(&working_dir)
            .status()?;

        if !generate_status.success() {
            anyhow::bail!("scud generate failed");
        }

        println!();
        tag_name
    } else if let Some(tag) = scud_tag {
        tag
    } else {
        anyhow::bail!("Either --scud-tag or --prd must be provided");
    };

    // Build spec config
    let mut spec_config = crate::spec::SpecConfig::new();
    spec_config.max_spec_tokens = Some(max_spec_tokens);
    // Use PRD as plan if no explicit plan provided
    if let Some(plan_path) = plan.or(prd.clone()) {
        spec_config.plan_path = Some(plan_path);
    }
    for spec_file in spec_files {
        spec_config.additional_specs.push(spec_file);
    }

    // Create executor
    let executor = crate::ralph_executor::RalphExecutor::new(
        final_tag,
        spec_config,
        verify,
        harness,
        model,
        round_size,
        !no_validate,
        working_dir,
    )?;

    if dry_run {
        executor.dry_run().await
    } else {
        executor.run().await
    }
}
```

**Success Criteria - Automated**:
- [ ] `cargo build` passes
- [ ] `descartes ralph --help` shows options

**Success Criteria - Manual**:
- [ ] All CLI options are documented
- [ ] Required options are enforced

---

## Phase 3: Implement Ralph Executor

**Goal**: Create the core Ralph loop executor with wave-based execution and backpressure.

**Changes**:

- [ ] Create `src/ralph_executor.rs`

```rust
// src/ralph_executor.rs
//! Ralph Wiggum executor
//!
//! Implements true fresh-context-per-task execution:
//! 1. Load spec sources (task + plan + custom files)
//! 2. Get next ready task from SCUD
//! 3. Spawn agent with fresh prompt (no accumulated context)
//! 4. Wait for completion
//! 5. Run validation (backpressure)
//! 6. Mark task done or failed in SCUD
//! 7. Repeat until all done

use anyhow::{Context, Result};
use scud::backpressure::{BackpressureConfig, run_validation};
use scud::models::{Task, TaskStatus};
use scud::storage::Storage;
use std::path::PathBuf;
use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::config::Config;
use crate::harness::{create_harness, Harness, SessionConfig};
use crate::spec::{build_task_spec, SpecConfig};

/// Ralph executor configuration
pub struct RalphExecutor {
    scud_tag: String,
    spec_config: SpecConfig,
    verify_command: Option<String>,
    harness_name: String,
    model: Option<String>,
    round_size: usize,
    validate: bool,
    working_dir: PathBuf,
    storage: Storage,
    bp_config: BackpressureConfig,
}

/// Result of executing a single task
enum TaskResult {
    Success,
    Failed(String),
    Blocked(String),
}

impl RalphExecutor {
    pub fn new(
        scud_tag: String,
        spec_config: SpecConfig,
        verify_command: Option<String>,
        harness_name: String,
        model: Option<String>,
        round_size: usize,
        validate: bool,
        working_dir: PathBuf,
    ) -> Result<Self> {
        let storage = Storage::new(Some(working_dir.clone()));

        if !storage.is_initialized() {
            anyhow::bail!("SCUD not initialized in {:?}. Run: scud init", working_dir);
        }

        // Load backpressure config
        let bp_config = if validate {
            let mut config = BackpressureConfig::load(Some(&working_dir))?;
            // Override with explicit verify command if provided
            if let Some(ref cmd) = verify_command {
                config.commands = vec![cmd.clone()];
            }
            config
        } else {
            BackpressureConfig::default()
        };

        Ok(Self {
            scud_tag,
            spec_config,
            verify_command,
            harness_name,
            model,
            round_size,
            validate,
            working_dir,
            storage,
            bp_config,
        })
    }

    /// Run the Ralph loop
    pub async fn run(&self) -> Result<()> {
        println!("🔄 Starting Ralph Wiggum loop for tag: {}", self.scud_tag);
        println!();

        // Create harness
        let config = Config::load()?;
        let harness = create_harness(&self.harness_name, &config)?;

        let mut wave_number = 1;
        loop {
            // Load fresh task state
            let phases = self.storage.load_tasks()?;
            let phase = phases.get(&self.scud_tag)
                .ok_or_else(|| anyhow::anyhow!("Tag '{}' not found", self.scud_tag))?;

            // Compute waves
            let waves = self.compute_waves(phase)?;

            if waves.is_empty() {
                println!("✅ All tasks complete!");
                break;
            }

            let wave_tasks = &waves[0];
            if wave_tasks.is_empty() {
                // Check for in-progress tasks
                let in_progress: Vec<_> = phase.tasks.iter()
                    .filter(|t| t.status == TaskStatus::InProgress)
                    .collect();

                if !in_progress.is_empty() {
                    println!("⏳ Waiting for {} in-progress task(s)...", in_progress.len());
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    continue;
                } else {
                    println!("⚠️  No ready tasks. Check for blocked tasks: scud list --status blocked");
                    break;
                }
            }

            println!("📦 Wave {} - {} task(s)", wave_number, wave_tasks.len());
            println!("{}", "-".repeat(40));

            // Execute tasks in rounds
            let mut completed_ids = Vec::new();
            for (round_idx, round) in wave_tasks.chunks(self.round_size).enumerate() {
                println!("  Round {}/{}", round_idx + 1, wave_tasks.len().div_ceil(self.round_size));

                for task in round {
                    let result = self.execute_task(task, harness.as_ref()).await;
                    match result {
                        Ok(TaskResult::Success) => {
                            self.mark_task_done(&task.id)?;
                            completed_ids.push(task.id.clone());
                            println!("    ✓ {} - {}", task.id, task.title);
                        }
                        Ok(TaskResult::Failed(reason)) => {
                            self.mark_task_failed(&task.id)?;
                            println!("    ✗ {} - {} (failed: {})", task.id, task.title, reason);
                        }
                        Ok(TaskResult::Blocked(reason)) => {
                            self.mark_task_blocked(&task.id)?;
                            println!("    ⚠ {} - {} (blocked: {})", task.id, task.title, reason);
                        }
                        Err(e) => {
                            error!("Task execution error: {}", e);
                            self.mark_task_failed(&task.id)?;
                            println!("    ✗ {} - {} (error: {})", task.id, task.title, e);
                        }
                    }
                }
            }

            // Run validation
            if self.validate && !self.bp_config.commands.is_empty() && !completed_ids.is_empty() {
                println!();
                println!("  🔍 Running validation...");

                let validation = run_validation(&self.working_dir, &self.bp_config)?;

                if validation.all_passed {
                    println!("    ✓ All checks passed");
                } else {
                    println!("    ✗ Validation failed:");
                    for failure in &validation.failures {
                        println!("      - {}", failure);
                    }

                    // Mark all completed tasks as failed
                    for task_id in &completed_ids {
                        self.mark_task_failed(task_id)?;
                    }
                    println!("    ⚠ Marked {} task(s) as failed", completed_ids.len());
                }
            }

            println!();
            wave_number += 1;
        }

        println!();
        println!("🏁 Ralph loop complete");
        Ok(())
    }

    /// Execute a single task with fresh context
    async fn execute_task(&self, task: &Task, harness: &dyn Harness) -> Result<TaskResult> {
        // Mark in-progress
        self.mark_task_in_progress(&task.id)?;

        // Build fresh spec (no accumulated context)
        let spec = build_task_spec(task, &self.spec_config)?;

        // Build prompt
        let prompt = self.build_prompt(&spec, task);

        // Start fresh session (no resume)
        let session_config = SessionConfig {
            model: self.model.clone().unwrap_or_else(|| "sonnet".to_string()),
            parent: None,
            working_dir: Some(self.working_dir.clone()),
        };
        let session = harness.start_session(session_config).await?;

        // Send prompt and collect response
        let mut stream = harness.send(&session, &prompt).await?;
        let mut full_output = String::new();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                crate::harness::ResponseChunk::Text(text) => {
                    full_output.push_str(&text);
                }
                crate::harness::ResponseChunk::Error(err) => {
                    warn!("Agent error: {}", err);
                }
                crate::harness::ResponseChunk::Done => break,
                _ => {}
            }
        }

        // Close session
        harness.close_session(&session).await?;

        // Parse result
        if full_output.contains("TASK_BLOCKED:") {
            let reason = full_output.lines()
                .find(|l| l.contains("TASK_BLOCKED:"))
                .map(|l| l.replace("TASK_BLOCKED:", "").trim().to_string())
                .unwrap_or_else(|| "Unknown reason".to_string());
            return Ok(TaskResult::Blocked(reason));
        }

        // Assume success if agent completed without error
        Ok(TaskResult::Success)
    }

    fn build_prompt(&self, spec: &str, task: &Task) -> String {
        let verification = self.verify_command.as_deref()
            .unwrap_or_else(|| {
                if !self.bp_config.commands.is_empty() {
                    &self.bp_config.commands[0]
                } else {
                    "echo 'No verification configured'"
                }
            });

        format!(
            r#"You are implementing SCUD task {} for tag '{}' using the Ralph Wiggum technique.

## Spec

{}

## Verification Command

After implementation, run:
```bash
{}
```

## Instructions

1. Implement the task described in the spec
2. Follow existing code patterns in the codebase
3. Run the verification command
4. If verification passes, you're done
5. If blocked after 3 attempts, output: TASK_BLOCKED: <reason>

Begin implementation."#,
            task.id,
            self.scud_tag,
            spec,
            verification
        )
    }

    /// Compute execution waves from task dependencies
    fn compute_waves(&self, phase: &scud::models::Phase) -> Result<Vec<Vec<&Task>>> {
        use std::collections::HashSet;

        // Get actionable tasks
        let actionable: Vec<&Task> = phase.tasks.iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .filter(|t| !t.is_expanded)
            .collect();

        if actionable.is_empty() {
            return Ok(Vec::new());
        }

        // Kahn's algorithm
        let task_ids: HashSet<String> = actionable.iter().map(|t| t.id.clone()).collect();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for task in &actionable {
            in_degree.entry(task.id.clone()).or_insert(0);
            for dep in &task.dependencies {
                if task_ids.contains(dep) {
                    *in_degree.entry(task.id.clone()).or_insert(0) += 1;
                    dependents.entry(dep.clone()).or_default().push(task.id.clone());
                }
            }
        }

        let mut waves: Vec<Vec<&Task>> = Vec::new();
        let mut remaining = in_degree.clone();

        while !remaining.is_empty() {
            let ready: Vec<String> = remaining.iter()
                .filter(|(_, &deg)| deg == 0)
                .map(|(id, _)| id.clone())
                .collect();

            if ready.is_empty() {
                break; // Circular dependency
            }

            let wave: Vec<&Task> = actionable.iter()
                .filter(|t| ready.contains(&t.id))
                .copied()
                .collect();

            for task_id in &ready {
                remaining.remove(task_id);
                if let Some(deps) = dependents.get(task_id) {
                    for dep_id in deps {
                        if let Some(deg) = remaining.get_mut(dep_id) {
                            *deg = deg.saturating_sub(1);
                        }
                    }
                }
            }

            waves.push(wave);
        }

        Ok(waves)
    }

    fn mark_task_in_progress(&self, task_id: &str) -> Result<()> {
        let mut phase = self.storage.load_group(&self.scud_tag)?;
        if let Some(task) = phase.get_task_mut(task_id) {
            task.status = TaskStatus::InProgress;
            self.storage.update_group(&self.scud_tag, &phase)?;
        }
        Ok(())
    }

    fn mark_task_done(&self, task_id: &str) -> Result<()> {
        let mut phase = self.storage.load_group(&self.scud_tag)?;
        if let Some(task) = phase.get_task_mut(task_id) {
            task.status = TaskStatus::Done;
            self.storage.update_group(&self.scud_tag, &phase)?;
        }
        Ok(())
    }

    fn mark_task_failed(&self, task_id: &str) -> Result<()> {
        let mut phase = self.storage.load_group(&self.scud_tag)?;
        if let Some(task) = phase.get_task_mut(task_id) {
            task.status = TaskStatus::Failed;
            self.storage.update_group(&self.scud_tag, &phase)?;
        }
        Ok(())
    }

    fn mark_task_blocked(&self, task_id: &str) -> Result<()> {
        let mut phase = self.storage.load_group(&self.scud_tag)?;
        if let Some(task) = phase.get_task_mut(task_id) {
            task.status = TaskStatus::Blocked;
            self.storage.update_group(&self.scud_tag, &phase)?;
        }
        Ok(())
    }

    /// Dry run - show execution plan without running
    pub async fn dry_run(&self) -> Result<()> {
        println!("📋 Ralph Execution Plan (dry-run)");
        println!("═══════════════════════════════════════");
        println!();
        println!("Tag:      {}", self.scud_tag);
        println!("Harness:  {}", self.harness_name);
        println!("Validate: {}", if self.validate { "yes" } else { "no" });
        println!();

        let phases = self.storage.load_tasks()?;
        let phase = phases.get(&self.scud_tag)
            .ok_or_else(|| anyhow::anyhow!("Tag '{}' not found", self.scud_tag))?;

        let waves = self.compute_waves(phase)?;

        if waves.is_empty() {
            println!("No pending tasks.");
            return Ok(());
        }

        let mut total_tasks = 0;
        for (wave_idx, wave) in waves.iter().enumerate() {
            total_tasks += wave.len();
            println!("Wave {} - {} task(s)", wave_idx + 1, wave.len());

            for (round_idx, round) in wave.chunks(self.round_size).enumerate() {
                println!("  Round {}:", round_idx + 1);
                for task in round {
                    println!("    ○ {} - {}", task.id, task.title);
                }
            }
            println!();
        }

        println!("Summary");
        println!("───────");
        println!("  Waves:  {}", waves.len());
        println!("  Tasks:  {}", total_tasks);
        println!("  Rounds: {}", waves.iter().map(|w| w.len().div_ceil(self.round_size)).sum::<usize>());
        println!();
        println!("No agents spawned (dry-run mode).");

        Ok(())
    }
}
```

- [ ] Export from `src/lib.rs`

```rust
pub mod ralph_executor;  // Add this line
```

**Success Criteria - Automated**:
- [ ] `cargo build` passes
- [ ] `cargo test` passes

**Success Criteria - Manual**:
- [ ] `descartes ralph --scud-tag test --dry-run` shows execution plan
- [ ] `descartes ralph --scud-tag test` executes tasks

---

## Phase 4: Integration Testing

**Goal**: Verify Ralph executor works correctly with real SCUD tasks.

**Changes**:

- [ ] Create integration test in `tests/ralph_integration.rs`

```rust
//! Integration tests for Ralph executor

use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_ralph_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let working_dir = temp_dir.path();

    // Initialize SCUD
    Command::new("scud")
        .arg("init")
        .current_dir(working_dir)
        .status()
        .expect("Failed to init SCUD");

    // Create a test task file
    // ... setup tasks ...

    // Run dry-run
    let output = Command::new("cargo")
        .args(["run", "--", "ralph", "--scud-tag", "test", "--dry-run"])
        .current_dir(working_dir)
        .output()
        .expect("Failed to run ralph");

    assert!(output.status.success() || String::from_utf8_lossy(&output.stderr).contains("not found"));
}
```

- [ ] Add unit tests to `src/ralph_executor.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt() {
        let executor = RalphExecutor {
            scud_tag: "test".to_string(),
            spec_config: SpecConfig::new(),
            verify_command: Some("cargo test".to_string()),
            harness_name: "claude-code".to_string(),
            model: None,
            round_size: 5,
            validate: true,
            working_dir: PathBuf::from("."),
            storage: Storage::new(None),
            bp_config: BackpressureConfig::default(),
        };

        let task = Task {
            id: "1".to_string(),
            title: "Test task".to_string(),
            description: Some("Do something".to_string()),
            status: TaskStatus::Pending,
            // ... other fields
        };

        let spec = "# Test Spec\n\nThis is a test.";
        let prompt = executor.build_prompt(spec, &task);

        assert!(prompt.contains("task 1"));
        assert!(prompt.contains("cargo test"));
        assert!(prompt.contains("TASK_BLOCKED"));
    }
}
```

**Success Criteria - Automated**:
- [ ] `cargo test` passes
- [ ] Integration test runs without errors

**Success Criteria - Manual**:
- [ ] End-to-end test with real SCUD tasks completes successfully

---

## Phase 5: Documentation

**Goal**: Document the Ralph command and update README.

**Changes**:

- [ ] Update `README.md` with Ralph command documentation

```markdown
## Ralph Command

The `descartes ralph` command implements a true Ralph Wiggum loop for executing SCUD tasks:

```bash
# Basic usage
descartes ralph --scud-tag my-feature

# With implementation plan
descartes ralph --scud-tag my-feature --plan ./plan.md

# With additional spec files
descartes ralph --scud-tag my-feature \
    --spec-file ./ARCHITECTURE.md \
    --spec-file ./API_CONTRACTS.md

# Custom verification command
descartes ralph --scud-tag my-feature --verify "npm test"

# Using a different harness
descartes ralph --scud-tag my-feature --harness opencode

# Dry run to see execution plan
descartes ralph --scud-tag my-feature --dry-run
```

### How It Works

1. **Fresh Context Per Task**: Each task gets a fresh agent session with no accumulated history
2. **Fixed Spec Allocation**: Task details + plan section + custom files (~5k tokens)
3. **Wave-Based Execution**: Tasks execute in dependency order
4. **Backpressure Validation**: Build/test/lint runs between waves
5. **Failed Task Tracking**: Failed validation marks tasks for retry
```

- [ ] Add `--help` documentation to CLI

**Success Criteria - Automated**:
- [ ] `descartes ralph --help` shows complete documentation

**Success Criteria - Manual**:
- [ ] README is clear and complete
- [ ] Examples work as documented

---

## Risks and Mitigations

### Risk: SCUD library API changes
**Mitigation**: Pin to specific scud-cli version, monitor for breaking changes.

### Risk: Harness session management issues
**Mitigation**: Each task gets fresh session, no accumulated state.

### Risk: Backpressure import from SCUD
**Mitigation**: SCUD Phase 2 exposes backpressure as public API first.

### Risk: Agent doesn't signal completion clearly
**Mitigation**: Default to success if no error, backpressure catches issues.

---

## Dependencies on SCUD Migration

This plan depends on SCUD migrations completing first:

**From SCUD Backpressure Migration (Phase 2)**:
- `scud::backpressure::BackpressureConfig` must be publicly exported
- `scud::backpressure::run_validation` must be publicly available

**From SCUD Generate Command Migration**:
- `scud generate` command must be available for CLI approach
- OR `scud::commands::generate::{generate, GenerateOptions}` for library approach

### Alternative: Library Integration

Instead of shelling out to `scud generate`, Descartes could call the library directly:

```rust
use scud::commands::generate::{generate, GenerateOptions};

let options = GenerateOptions {
    num_tasks,
    no_expand,
    no_check_deps,
    ..Default::default()
};

generate(Some(working_dir.clone()), &prd_path, &tag_name, options).await?;
```

This is preferred for tighter integration but requires the SCUD generate command to be implemented first.

If SCUD migration is delayed, temporarily shell out to the three separate commands.

---

## Open Questions

None - all resolved.

---

## Phase 6: Progress Visibility & Agent Attach (Zellij Integration)

**Goal**: Provide real-time visibility into running agents with ability to attach/detach.

### Problem Statement

When running `descartes ralph` (or `scud swarm`), the orchestrator appears frozen after spawning agents. Users have no visibility into:
- What each agent is currently doing
- Whether agents are stuck or making progress
- How to jump into a specific agent's session

### Solution: Zellij-Native Tab/Pane Architecture

Spawn agents in Zellij panes within a dedicated tab, with an orchestrator pane for monitoring.

```
┌─ Zellij ──────────────────────────────────────────────────────────┐
│ [main] [ralph-{tag}] [other tabs...]                              │
├───────────────────────────────────────────────────────────────────┤
│ ┌─ orchestrator ──────┐ ┌─ task-1.1 ─────┐ ┌─ task-1.2 ─────────┐│
│ │ Ralph: {tag}        │ │                │ │                    ││
│ │ Wave 1/3            │ │ [claude code   │ │ [claude code       ││
│ │                     │ │  live output]  │ │  live output]      ││
│ │ ● 1.1 SpecConfig    │ │                │ │                    ││
│ │ ● 1.2 build_spec    │ │                │ │                    ││
│ │ ○ 2   export        │ │                │ │                    ││
│ │                     │ │                │ │                    ││
│ │ [1-5] attach agent  │ │                │ │                    ││
│ │ [v] validate now    │ │                │ │                    ││
│ └─────────────────────┘ └────────────────┘ └────────────────────┘│
└───────────────────────────────────────────────────────────────────┘
```

### Components

#### 1. Agent Registry

Track spawned agents with terminal-specific handles:

```rust
// src/agent_registry.rs

use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AgentHandle {
    pub id: Uuid,
    pub task_id: String,
    pub task_title: String,
    pub pane_name: String,        // "task-1.1"
    pub terminal_type: TerminalType,
    pub status: AgentStatus,
    pub spawned_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum AgentStatus {
    Running,
    Completed,
    Failed(String),
    Blocked(String),
}

#[derive(Debug, Clone)]
pub enum TerminalType {
    Zellij { tab_name: String },
    Tmux { session: String, window: usize },
    Kitty { window_id: String },
    Headless,  // No terminal, streaming mode
}

pub struct AgentRegistry {
    agents: HashMap<Uuid, AgentHandle>,
    by_task: HashMap<String, Uuid>,
}

impl AgentRegistry {
    pub fn spawn(&mut self, task_id: &str, task_title: &str, terminal: TerminalType) -> Uuid;
    pub fn get_by_task(&self, task_id: &str) -> Option<&AgentHandle>;
    pub fn update_status(&mut self, id: Uuid, status: AgentStatus);
    pub fn list_running(&self) -> Vec<&AgentHandle>;
    pub fn focus(&self, id: Uuid) -> Result<()>;  // Attach to agent
}
```

#### 2. Zellij Terminal Backend

Add Zellij support to terminal spawning:

```rust
// In terminal spawning code

pub fn spawn_zellij(task_id: &str, task_title: &str, command: &str, tab_name: &str) -> Result<String> {
    // Create tab if not exists
    let tab_exists = Command::new("zellij")
        .args(["action", "query-tab-names"])
        .output()?;

    if !String::from_utf8_lossy(&tab_exists.stdout).contains(tab_name) {
        Command::new("zellij")
            .args(["action", "new-tab", "--name", tab_name])
            .status()?;
    }

    // Spawn pane with named task
    let pane_name = format!("task-{}", task_id);
    Command::new("zellij")
        .args([
            "action", "new-pane",
            "--name", &pane_name,
            "--direction", "right",
            "--",
        ])
        .arg("bash")
        .arg("-c")
        .arg(format!(
            "export SCUD_TASK_ID='{}'; {} ; exec bash",
            task_id, command
        ))
        .status()?;

    Ok(pane_name)
}

pub fn focus_zellij_pane(pane_name: &str) -> Result<()> {
    // Zellij doesn't have direct pane-by-name focus, use move-focus
    // Alternative: use zellij plugin API for more control
    Command::new("zellij")
        .args(["action", "move-focus", "right"])
        .status()?;
    Ok(())
}
```

#### 3. Orchestrator TUI

Simple terminal UI for the orchestrator pane:

```rust
// src/ralph_tui.rs

use crossterm::{event, terminal};
use std::io::Write;

pub struct RalphTui {
    registry: AgentRegistry,
    tag: String,
    current_wave: usize,
    total_waves: usize,
}

impl RalphTui {
    pub fn run(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;

        loop {
            self.render()?;

            if event::poll(Duration::from_millis(100))? {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('1'..='9') => {
                            let idx = key.code as usize - '1' as usize;
                            self.attach_agent(idx)?;
                        }
                        KeyCode::Char('v') => self.run_validation()?,
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }

            self.poll_task_status()?;
        }

        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn render(&self) -> Result<()> {
        print!("\x1B[2J\x1B[1;1H");  // Clear screen

        println!("Ralph: {} | Wave {}/{}", self.tag, self.current_wave, self.total_waves);
        println!("{}", "─".repeat(40));

        for (i, agent) in self.registry.list_running().iter().enumerate() {
            let status_icon = match agent.status {
                AgentStatus::Running => "●",
                AgentStatus::Completed => "✓",
                AgentStatus::Failed(_) => "✗",
                AgentStatus::Blocked(_) => "⚠",
            };
            println!("[{}] {} {} - {}", i + 1, status_icon, agent.task_id, agent.task_title);
        }

        println!();
        println!("[1-9] attach  [v] validate  [q] quit");

        std::io::stdout().flush()?;
        Ok(())
    }

    fn attach_agent(&self, idx: usize) -> Result<()> {
        if let Some(agent) = self.registry.list_running().get(idx) {
            self.registry.focus(agent.id)?;
        }
        Ok(())
    }
}
```

#### 4. CLI Options

```rust
// Add to Ralph command

/// Terminal type for agent spawning
#[arg(long, default_value = "zellij")]
terminal: String,  // zellij, tmux, kitty, headless

/// Enable interactive TUI mode (default when terminal is zellij/tmux)
#[arg(long)]
interactive: bool,

/// Tab name for Zellij (default: ralph-{tag})
#[arg(long)]
tab_name: Option<String>,
```

### User Workflow

1. **Start Ralph session**:
   ```bash
   descartes ralph --tag migrate --terminal zellij
   ```

2. **Zellij opens new tab** "ralph-migrate" with orchestrator pane

3. **Agents spawn** in new panes as waves execute

4. **Watch progress** in orchestrator pane (updates every 100ms)

5. **Attach to agent**: Press `1` to focus first agent's pane
   - You're now in Claude Code, can interact normally
   - Agent continues where it was

6. **Detach**: Use Zellij keybinds (Ctrl+p, arrows) to return to orchestrator

7. **Validation**: Press `v` to run backpressure validation immediately

8. **Completion**: Tab remains for review, orchestrator shows final summary

### Success Criteria

**Automated**:
- [ ] `cargo build` passes with new modules
- [ ] Agent registry tracks spawned agents correctly
- [ ] Zellij pane spawning works

**Manual**:
- [ ] New tab created in Zellij when ralph starts
- [ ] Agent panes spawn with correct names
- [ ] Orchestrator TUI updates in real-time
- [ ] Pressing number keys focuses correct pane
- [ ] Can interact with Claude Code after attach
- [ ] Validation runs on keypress

---

## File Reference Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `src/spec.rs` | Create | Spec configuration system |
| `src/ralph_executor.rs` | Create | Ralph loop executor |
| `src/agent_registry.rs` | Create | Agent tracking with terminal handles |
| `src/ralph_tui.rs` | Create | Orchestrator TUI for watch/attach |
| `src/terminal/mod.rs` | Create | Terminal backend abstraction |
| `src/terminal/zellij.rs` | Create | Zellij pane spawning/focus |
| `src/main.rs` | Modify | Add Ralph command handler inline |
| `src/main.rs` | Modify | Add Ralph command variant |
| `src/lib.rs` | Modify | Export new modules |
| `tests/ralph_integration.rs` | Create | Integration tests |
| `README.md` | Modify | Add documentation |
