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

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use futures::StreamExt;
use scud::backpressure::{run_validation, BackpressureConfig};
use scud::models::{Phase, Task, TaskStatus};
use scud::storage::Storage;
use tracing::{debug, error, info, warn};

use crate::agent::{AgentRegistry, RegistryStatus};
use crate::context_handoff::{summarize_agent_progress, ContextMonitor, HandoffContext};
use crate::harness::{create_harness_by_name, ResponseChunk, SessionConfig};
use crate::ralph_tui::{RalphTui, TuiAction};
use crate::spec::{build_prompt, build_task_spec, SpecConfig};
use crate::{Config, Error, Result};

/// Result of executing a task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResult {
    /// Task completed successfully
    Completed {
        /// Full response text from the agent
        response: String,
    },
    /// Task was blocked and cannot proceed
    Blocked {
        /// Reason provided by the agent
        reason: String,
    },
}

/// Ralph executor for running SCUD tasks with fresh context per task
pub struct RalphExecutor {
    /// SCUD tag to operate on
    pub scud_tag: String,
    /// Spec configuration for building prompts
    pub spec_config: SpecConfig,
    /// Optional verification command (overrides backpressure config)
    pub verify_command: Option<String>,
    /// Harness name (claude-code, opencode, codex)
    pub harness_name: String,
    /// Optional model override
    pub model: Option<String>,
    /// Maximum tasks per round
    pub round_size: usize,
    /// Whether to run validation between waves
    pub validate: bool,
    /// Working directory
    pub working_dir: PathBuf,
    /// Context window size in tokens (default: 200K for Claude Opus 4.5)
    pub context_window: usize,
    /// Threshold percentage for handoff (default: 0.6 = 60%)
    pub handoff_threshold: f64,
    /// Whether context handoff is enabled
    pub enable_handoff: bool,
}

impl RalphExecutor {
    /// Create a new RalphExecutor
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
        Ok(Self {
            scud_tag,
            spec_config,
            verify_command,
            harness_name,
            model,
            round_size,
            validate,
            working_dir,
            context_window: 200_000,
            handoff_threshold: 0.6,
            enable_handoff: true,
        })
    }

    /// Configure context handoff settings
    pub fn with_handoff_settings(
        mut self,
        context_window: usize,
        threshold: f64,
        enabled: bool,
    ) -> Self {
        self.context_window = context_window;
        self.handoff_threshold = threshold.clamp(0.0, 1.0);
        self.enable_handoff = enabled;
        self
    }

    /// Show execution plan without running agents (dry run mode)
    pub async fn dry_run(&self) -> Result<()> {
        info!(
            "Dry run for tag '{}' in {:?}",
            self.scud_tag, self.working_dir
        );

        // Load tasks from storage
        let storage = Storage::new(Some(self.working_dir.clone()));
        let phases = storage
            .load_tasks()
            .map_err(|e| crate::Error::Config(format!("Failed to load tasks: {}", e)))?;

        let phase = phases
            .get(&self.scud_tag)
            .ok_or_else(|| crate::Error::Config(format!("Tag '{}' not found", self.scud_tag)))?;

        // Compute execution waves
        let waves = self.compute_waves(phase);

        // Header
        println!("=== Ralph Execution Plan ===");
        println!();
        println!("Tag:       {}", self.scud_tag);
        println!("Harness:   {}", self.harness_name);
        println!("Validate:  {}", if self.validate { "yes" } else { "no" });
        println!();

        if waves.is_empty() {
            println!("No pending tasks.");
            println!();
            println!("No agents spawned (dry-run mode)");
            return Ok(());
        }

        // Count totals for summary
        let total_waves = waves.len();
        let total_tasks: usize = waves.iter().map(|w| w.len()).sum();
        let mut total_rounds = 0;

        // Display each wave with rounds
        for (wave_idx, wave) in waves.iter().enumerate() {
            println!("Wave {}:", wave_idx + 1);

            // Split wave into rounds based on round_size
            let rounds: Vec<&[&Task]> = wave.chunks(self.round_size).collect();
            total_rounds += rounds.len();

            for (round_idx, round) in rounds.iter().enumerate() {
                println!("  Round {}:", round_idx + 1);
                for task in *round {
                    println!("    - [{}] {}", task.id, task.title);
                }
            }
            println!();
        }

        // Summary section
        println!("Summary:");
        println!("  Waves:  {}", total_waves);
        println!("  Tasks:  {}", total_tasks);
        println!("  Rounds: {}", total_rounds);
        println!();
        println!("No agents spawned (dry-run mode)");

        Ok(())
    }

    /// Run the Ralph loop, executing tasks with fresh context
    ///
    /// Executes tasks in waves, where each wave contains tasks that can be
    /// executed in parallel. Within each wave, tasks are processed in rounds
    /// of `round_size` tasks, with backpressure validation between rounds.
    ///
    /// If validation fails after a round, all completed tasks in that round
    /// are marked as Failed.
    pub async fn run(&self, config: &Config) -> Result<()> {
        info!(
            "Starting Ralph loop for tag '{}' in {:?}",
            self.scud_tag, self.working_dir
        );

        // Load backpressure config
        let bp_config = BackpressureConfig::load(Some(&self.working_dir))
            .map_err(|e| Error::Config(format!("Failed to load backpressure config: {}", e)))?;
        let backpressure_commands = bp_config.commands.clone();

        info!(
            "Backpressure config: {} commands, stop_on_failure={}",
            backpressure_commands.len(),
            bp_config.stop_on_failure
        );

        // Load tasks from storage
        let storage = Storage::new(Some(self.working_dir.clone()));
        let phases = storage
            .load_tasks()
            .map_err(|e| Error::Config(format!("Failed to load tasks: {}", e)))?;

        let phase = phases
            .get(&self.scud_tag)
            .ok_or_else(|| Error::Config(format!("Tag '{}' not found", self.scud_tag)))?;

        // Compute execution waves
        let waves = self.compute_waves(phase);

        if waves.is_empty() {
            println!("All tasks complete for tag '{}'!", self.scud_tag);
            return Ok(());
        }

        let total_tasks: usize = waves.iter().map(|w| w.len()).sum();
        println!(
            "Ralph loop for tag '{}' - {} wave(s), {} task(s)",
            self.scud_tag,
            waves.len(),
            total_tasks
        );
        println!();

        // Initialize TUI with waves
        let wave_ids: Vec<Vec<String>> = waves
            .iter()
            .map(|wave| wave.iter().map(|t| t.id.clone()).collect())
            .collect();

        let mut tui = RalphTui::new();
        tui.set_waves(wave_ids);

        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut blocked_count = 0;
        let mut validation_requested = false;

        // Process each wave
        for (wave_idx, wave) in waves.iter().enumerate() {
            println!(
                "=== Wave {} / {} ({} tasks) ===",
                wave_idx + 1,
                waves.len(),
                wave.len()
            );

            // Update TUI wave progress
            if let Some(progress) = tui.wave_progress_mut() {
                if wave_idx > 0 {
                    progress.advance_wave();
                }
            }

            // Split wave into rounds based on round_size
            let rounds: Vec<&[&Task]> = wave.chunks(self.round_size).collect();

            for (round_idx, round) in rounds.iter().enumerate() {
                println!(
                    "  Round {} / {} ({} tasks)",
                    round_idx + 1,
                    rounds.len(),
                    round.len()
                );

                // Track completed tasks in this round for potential failure marking
                let mut round_completed_tasks: Vec<String> = Vec::new();

                // Execute each task in the round sequentially
                for task in *round {
                    println!("    Executing: [{}] {}", task.id, task.title);

                    // Track agent in registry
                    let pane_name = format!("ralph-{}", task.id);
                    let agent_handle = tui.registry_mut().spawn(&pane_name, Some(task.id.clone()));

                    // Update agent status to running
                    let _ = tui
                        .registry_mut()
                        .update_status(&agent_handle.id, RegistryStatus::Running);

                    // Render TUI and check for keyboard input
                    match tui.tick() {
                        Ok(TuiAction::Validate) => {
                            validation_requested = true;
                            println!("    Manual validation requested via TUI");
                        }
                        Ok(TuiAction::Quit) => {
                            println!("    TUI quit requested - stopping execution");
                            return Ok(());
                        }
                        Ok(TuiAction::Attach(_)) => {
                            // Handled in TUI internally
                        }
                        Ok(TuiAction::None) => {}
                        Err(e) => {
                            debug!("TUI tick error (non-fatal): {}", e);
                        }
                    }

                    match self
                        .execute_task(task, config, &backpressure_commands)
                        .await
                    {
                        Ok(TaskResult::Completed { .. }) => {
                            // Mark task as done
                            if let Err(e) = self.set_task_status(&task.id, TaskStatus::Done) {
                                error!("Failed to mark task {} as done: {}", task.id, e);
                            }
                            round_completed_tasks.push(task.id.clone());
                            completed_count += 1;
                            println!("      [DONE] Task {} completed", task.id);

                            // Update agent status and wave progress
                            let _ = tui
                                .registry_mut()
                                .update_status(&agent_handle.id, RegistryStatus::Completed);
                            if let Some(progress) = tui.wave_progress_mut() {
                                progress.complete_task();
                            }
                        }
                        Ok(TaskResult::Blocked { reason }) => {
                            // Mark task as blocked
                            if let Err(e) = self.set_task_status(&task.id, TaskStatus::Blocked) {
                                error!("Failed to mark task {} as blocked: {}", task.id, e);
                            }
                            blocked_count += 1;
                            println!("      [BLOCKED] Task {}: {}", task.id, reason);

                            // Update agent status
                            let _ = tui
                                .registry_mut()
                                .update_status(&agent_handle.id, RegistryStatus::Failed);
                        }
                        Err(e) => {
                            // Mark task as failed on error
                            error!("Task {} execution error: {}", task.id, e);
                            if let Err(set_err) = self.set_task_status(&task.id, TaskStatus::Failed)
                            {
                                error!("Failed to mark task {} as failed: {}", task.id, set_err);
                            }
                            failed_count += 1;
                            println!("      [ERROR] Task {}: {}", task.id, e);

                            // Update agent status
                            let _ = tui
                                .registry_mut()
                                .update_status(&agent_handle.id, RegistryStatus::Failed);
                        }
                    }

                    // Render TUI after task completion
                    let _ = tui.render();
                }

                // Run backpressure validation after round if enabled or manually requested
                if (self.validate || validation_requested) && !bp_config.commands.is_empty() {
                    println!("    Running validation...");
                    validation_requested = false; // Reset flag

                    match run_validation(&self.working_dir, &bp_config) {
                        Ok(result) => {
                            if result.all_passed {
                                println!("    [PASS] All validation checks passed");
                            } else {
                                // Validation failed - mark all completed tasks in this round as Failed
                                error!(
                                    "Validation failed: {:?}. Marking {} tasks as failed.",
                                    result.failures,
                                    round_completed_tasks.len()
                                );
                                println!(
                                    "    [FAIL] Validation failed: {}",
                                    result.failures.join(", ")
                                );

                                // Print failure details
                                for cmd_result in &result.results {
                                    if !cmd_result.passed {
                                        println!(
                                            "      {} (exit {:?})",
                                            cmd_result.command, cmd_result.exit_code
                                        );
                                        if !cmd_result.stderr.is_empty() {
                                            // Print first few lines of stderr
                                            for line in cmd_result.stderr.lines().take(5) {
                                                println!("        {}", line);
                                            }
                                        }
                                    }
                                }

                                // Mark all completed tasks in this round as Failed
                                for task_id in &round_completed_tasks {
                                    if let Err(e) =
                                        self.set_task_status(task_id, TaskStatus::Failed)
                                    {
                                        error!("Failed to mark task {} as failed: {}", task_id, e);
                                    } else {
                                        // Adjust counts: task was completed but now failed
                                        completed_count -= 1;
                                        failed_count += 1;
                                        println!("      Marked task {} as failed", task_id);
                                    }
                                }

                                // Stop processing this wave on validation failure
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Validation execution error: {}", e);
                            println!("    [ERROR] Validation failed to run: {}", e);

                            // Mark all completed tasks as failed on validation error
                            for task_id in &round_completed_tasks {
                                if let Err(set_err) =
                                    self.set_task_status(task_id, TaskStatus::Failed)
                                {
                                    error!(
                                        "Failed to mark task {} as failed: {}",
                                        task_id, set_err
                                    );
                                } else {
                                    completed_count -= 1;
                                    failed_count += 1;
                                }
                            }

                            break;
                        }
                    }
                }
            }

            println!();
        }

        // Final render before summary
        let _ = tui.render();

        // Final summary
        println!("=== Summary ===");
        println!("  Completed: {}", completed_count);
        println!("  Failed:    {}", failed_count);
        println!("  Blocked:   {}", blocked_count);
        println!();

        if failed_count > 0 {
            warn!("Ralph loop completed with {} failed task(s)", failed_count);
        }

        Ok(())
    }

    /// Execute a single task with fresh harness session lifecycle
    ///
    /// This implements the core Ralph Wiggum pattern:
    /// 1. Mark task as in-progress
    /// 2. Build fresh spec via `build_task_spec()`
    /// 3. Create SessionConfig with model/working_dir
    /// 4. Start harness session
    /// 5. Send prompt and collect stream response
    /// 6. Close session (fresh context for next task)
    /// 7. Parse result for TASK_BLOCKED: pattern
    ///
    /// # Arguments
    ///
    /// * `task` - The task to execute
    /// * `config` - The application config for harness creation
    /// * `backpressure_commands` - Validation commands from backpressure config
    ///
    /// # Returns
    ///
    /// `TaskResult::Completed` with full response or `TaskResult::Blocked` with reason
    pub async fn execute_task(
        &self,
        task: &Task,
        config: &Config,
        backpressure_commands: &[String],
    ) -> Result<TaskResult> {
        info!(
            "Executing task {} [{}] for tag '{}'",
            task.id, task.title, self.scud_tag
        );

        // 1. Mark task as in-progress
        self.set_task_status(&task.id, TaskStatus::InProgress)?;

        // 2. Build fresh spec
        let spec = build_task_spec(&self.spec_config, task)?;
        debug!("Built spec for task {}: {} chars", task.id, spec.len());

        // 3. Build the prompt with verification commands
        let mut prompt = build_prompt(
            &spec,
            task,
            &self.scud_tag,
            self.verify_command.as_deref(),
            backpressure_commands,
        );
        debug!("Built prompt for task {}: {} chars", task.id, prompt.len());

        // Create context monitor
        let mut context_monitor = if self.enable_handoff {
            ContextMonitor::new(self.context_window, self.handoff_threshold)
        } else {
            ContextMonitor::disabled()
        };

        // Track handoff context for potential restarts
        let mut handoff_count = 0;
        let mut full_response = String::new();

        // Main execution loop with handoff support
        loop {
            // 4. Create harness and session
            let harness = create_harness_by_name(&self.harness_name, config)?;

            // Get default model based on harness type
            let default_model = match self.harness_name.as_str() {
                "opencode" => config.harness.opencode.model.clone(),
                "codex" => config.harness.codex.model.clone(),
                _ => config.harness.claude_code.model.clone(),
            };
            let model = self.model.clone().unwrap_or(default_model);

            let session_config = SessionConfig {
                model,
                tools: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "edit".to_string(),
                    "bash".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ],
                system_prompt: None,
                parent: None,
                is_subagent: false,
            };

            // 5. Start session
            let session = harness.start_session(session_config).await?;
            info!(
                "Started session {} with model {} (handoff #{})",
                session.id, session.model, handoff_count
            );

            // Record prompt tokens
            context_monitor.record_usage(&prompt);

            // 6. Send prompt and collect response with context monitoring
            let mut response_stream = harness.send(&session, &prompt).await?;
            let mut iteration_response = String::new();

            while let Some(chunk) = response_stream.next().await {
                match chunk {
                    ResponseChunk::Text(text) => {
                        iteration_response.push_str(&text);
                        context_monitor.record_usage(&text);

                        // Check if handoff threshold reached
                        if context_monitor.should_handoff() {
                            warn!(
                                "Context handoff triggered at {}% usage for task {}",
                                (context_monitor.usage_percentage() * 100.0) as u32,
                                task.id
                            );
                            break; // Exit stream collection to perform handoff
                        }
                    }
                    ResponseChunk::Done => {
                        debug!("Response stream complete for task {}", task.id);
                        break;
                    }
                    ResponseChunk::Error(err) => {
                        warn!("Error in response stream for task {}: {}", task.id, err);
                        // Continue collecting - error might be recoverable
                    }
                    ResponseChunk::ToolCall(tool) => {
                        debug!("Tool call in task {}: {} ({})", task.id, tool.name, tool.id);
                    }
                    ResponseChunk::ToolResult(result) => {
                        debug!(
                            "Tool result for task {}: {} (success: {})",
                            task.id, result.tool_call_id, result.success
                        );
                    }
                    ResponseChunk::SubagentSpawn(req) => {
                        debug!(
                            "Subagent spawn request in task {}: {} - {}",
                            task.id, req.category, req.prompt
                        );
                    }
                }
            }

            // Accumulate response
            full_response.push_str(&iteration_response);

            // 7. Close session
            harness.close_session(&session).await?;
            info!("Closed session {} for task {}", session.id, task.id);

            // Check if we need to perform handoff or if we're done
            if context_monitor.should_handoff() {
                // Perform handoff
                handoff_count += 1;
                info!(
                    "Performing context handoff #{} for task {}",
                    handoff_count, task.id
                );

                // Summarize progress so far
                let summary = summarize_agent_progress(&full_response);
                debug!("Generated handoff summary: {} chars", summary.len());

                // Create handoff context and new prompt
                let handoff_ctx = HandoffContext::new(summary, spec.clone(), handoff_count);
                prompt = handoff_ctx.build_handoff_prompt();

                // Reset context monitor for the new agent
                context_monitor.reset();

                // Continue loop with fresh agent
                info!(
                    "Spawning fresh agent for task {} after handoff #{}",
                    task.id, handoff_count
                );
                continue;
            } else {
                // Task completed (no handoff needed)
                break;
            }
        }

        // 8. Parse result for TASK_BLOCKED pattern
        let result = Self::parse_task_result(&full_response);

        match &result {
            TaskResult::Completed { .. } => {
                info!(
                    "Task {} completed successfully (handoffs: {})",
                    task.id, handoff_count
                );
            }
            TaskResult::Blocked { reason } => {
                info!("Task {} blocked: {}", task.id, reason);
            }
        }

        Ok(result)
    }

    /// Parse response text for TASK_BLOCKED pattern
    ///
    /// Scans the response for "TASK_BLOCKED:" pattern and extracts the reason.
    /// Returns `TaskResult::Completed` if no blocking pattern is found.
    fn parse_task_result(response: &str) -> TaskResult {
        // Look for TASK_BLOCKED: pattern (case-insensitive prefix match)
        for line in response.lines().rev() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("TASK_BLOCKED:") {
                let reason = rest.trim().to_string();
                return TaskResult::Blocked {
                    reason: if reason.is_empty() {
                        "No reason provided".to_string()
                    } else {
                        reason
                    },
                };
            }
        }

        TaskResult::Completed {
            response: response.to_string(),
        }
    }

    /// Set the status of a task in SCUD storage
    fn set_task_status(&self, task_id: &str, status: TaskStatus) -> Result<()> {
        let storage = Storage::new(Some(self.working_dir.clone()));

        // Load tasks
        let mut phases = storage
            .load_tasks()
            .map_err(|e| Error::Config(format!("Failed to load tasks: {}", e)))?;

        // Get mutable reference to the phase
        let phase = phases
            .get_mut(&self.scud_tag)
            .ok_or_else(|| Error::Config(format!("Tag '{}' not found", self.scud_tag)))?;

        // Find and update the task
        if let Some(task) = phase.get_task_mut(task_id) {
            task.set_status(status.clone());

            // Save the updated tasks
            storage
                .save_tasks(&phases)
                .map_err(|e| Error::Config(format!("Failed to save tasks: {}", e)))?;

            debug!("Set task {} status to {:?}", task_id, status);
            Ok(())
        } else {
            Err(Error::Config(format!(
                "Task '{}' not found in tag '{}'",
                task_id, self.scud_tag
            )))
        }
    }

    /// Compute execution waves from task dependencies using Kahn's algorithm
    ///
    /// Returns tasks grouped into waves where each wave contains tasks that can
    /// be executed in parallel (all their dependencies are satisfied by previous waves).
    ///
    /// # Algorithm
    /// 1. Filter to actionable tasks (Pending status, not expanded into subtasks)
    /// 2. Build dependency graph with in-degree counts
    /// 3. Repeatedly extract tasks with in-degree 0 into waves
    /// 4. Decrement in-degrees of dependent tasks after each wave
    pub fn compute_waves<'a>(&self, phase: &'a Phase) -> Vec<Vec<&'a Task>> {
        // Get actionable tasks: pending and not expanded into subtasks
        let actionable: Vec<&Task> = phase
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .filter(|t| !t.is_expanded())
            .collect();

        if actionable.is_empty() {
            return Vec::new();
        }

        // Build set of actionable task IDs for dependency filtering
        let task_ids: HashSet<String> = actionable.iter().map(|t| t.id.clone()).collect();

        // Initialize in-degree (count of unmet dependencies) for each task
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        // Map from task ID to list of tasks that depend on it
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for task in &actionable {
            in_degree.entry(task.id.clone()).or_insert(0);

            // Only count dependencies that are in the actionable set
            // (dependencies already done or not in this tag are ignored)
            for dep in &task.dependencies {
                if task_ids.contains(dep) {
                    *in_degree.entry(task.id.clone()).or_insert(0) += 1;
                    dependents
                        .entry(dep.clone())
                        .or_default()
                        .push(task.id.clone());
                }
            }
        }

        // Kahn's algorithm: repeatedly find tasks with in_degree == 0
        let mut waves: Vec<Vec<&Task>> = Vec::new();
        let mut remaining = in_degree.clone();

        while !remaining.is_empty() {
            // Find all tasks with no unmet dependencies
            let mut ready: Vec<String> = remaining
                .iter()
                .filter(|(_, &deg)| deg == 0)
                .map(|(id, _)| id.clone())
                .collect();

            if ready.is_empty() {
                // Circular dependency detected - log warning and break
                tracing::warn!(
                    "Circular dependency detected. Remaining tasks: {:?}",
                    remaining.keys().collect::<Vec<_>>()
                );
                break;
            }

            // Sort for deterministic ordering
            ready.sort();

            // Build wave from ready task IDs
            let wave: Vec<&Task> = actionable
                .iter()
                .filter(|t| ready.contains(&t.id))
                .copied()
                .collect();

            // Remove ready tasks from remaining and update dependents
            for task_id in &ready {
                remaining.remove(task_id);

                // Decrement in-degree for tasks that depend on this one
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

        waves
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_phase() -> Phase {
        let mut phase = Phase::new("test".to_string());

        // Task 1 has no deps - should be in wave 1
        let task1 = Task::new("1".to_string(), "Task 1".to_string(), "".to_string());

        // Task 2 depends on 1 - should be in wave 2
        let mut task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());
        task2.dependencies = vec!["1".to_string()];

        // Task 3 depends on 1 - should be in wave 2 (parallel with 2)
        let mut task3 = Task::new("3".to_string(), "Task 3".to_string(), "".to_string());
        task3.dependencies = vec!["1".to_string()];

        // Task 4 depends on 2 and 3 - should be in wave 3
        let mut task4 = Task::new("4".to_string(), "Task 4".to_string(), "".to_string());
        task4.dependencies = vec!["2".to_string(), "3".to_string()];

        phase.add_task(task1);
        phase.add_task(task2);
        phase.add_task(task3);
        phase.add_task(task4);

        phase
    }

    fn create_executor() -> RalphExecutor {
        RalphExecutor {
            scud_tag: "test".to_string(),
            spec_config: SpecConfig::default(),
            verify_command: None,
            harness_name: "claude-code".to_string(),
            model: None,
            round_size: 5,
            validate: false,
            working_dir: PathBuf::from("."),
            context_window: 200_000,
            handoff_threshold: 0.6,
            enable_handoff: false,
        }
    }

    #[test]
    fn test_compute_waves_basic() {
        let phase = create_test_phase();
        let executor = create_executor();

        let waves = executor.compute_waves(&phase);

        assert_eq!(waves.len(), 3, "Should have 3 waves");
        assert_eq!(waves[0].len(), 1, "Wave 1 should have 1 task");
        assert_eq!(waves[0][0].id, "1");
        assert_eq!(waves[1].len(), 2, "Wave 2 should have 2 tasks");
        assert_eq!(waves[2].len(), 1, "Wave 3 should have 1 task");
        assert_eq!(waves[2][0].id, "4");
    }

    #[test]
    fn test_compute_waves_empty() {
        let phase = Phase::new("empty".to_string());
        let executor = create_executor();

        let waves = executor.compute_waves(&phase);
        assert!(waves.is_empty(), "Empty phase should have no waves");
    }

    #[test]
    fn test_compute_waves_all_done() {
        let mut phase = create_test_phase();
        for task in &mut phase.tasks {
            task.set_status(TaskStatus::Done);
        }

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);
        assert!(waves.is_empty(), "All done tasks should produce no waves");
    }

    #[test]
    fn test_compute_waves_no_deps() {
        let mut phase = Phase::new("test".to_string());

        // Three independent tasks
        phase.add_task(Task::new("a".to_string(), "A".to_string(), "".to_string()));
        phase.add_task(Task::new("b".to_string(), "B".to_string(), "".to_string()));
        phase.add_task(Task::new("c".to_string(), "C".to_string(), "".to_string()));

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        assert_eq!(waves.len(), 1, "Independent tasks should be in one wave");
        assert_eq!(waves[0].len(), 3, "All 3 tasks should be in wave 1");
    }

    #[test]
    fn test_compute_waves_partial_done() {
        let mut phase = create_test_phase();

        // Mark task 1 as done - tasks 2 and 3 should now be in wave 1
        phase.tasks[0].set_status(TaskStatus::Done);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        assert_eq!(waves.len(), 2, "Should have 2 waves after task 1 done");
        assert_eq!(waves[0].len(), 2, "Wave 1 should have tasks 2 and 3");
        assert_eq!(waves[1].len(), 1, "Wave 2 should have task 4");
    }

    #[test]
    fn test_executor_new() {
        let spec_config = SpecConfig::default();
        let executor = RalphExecutor::new(
            "test-tag".to_string(),
            spec_config,
            Some("cargo test".to_string()),
            "claude-code".to_string(),
            Some("claude-3-opus".to_string()),
            10,
            true,
            PathBuf::from("/tmp/test"),
        )
        .unwrap();

        assert_eq!(executor.scud_tag, "test-tag");
        assert_eq!(executor.harness_name, "claude-code");
        assert_eq!(executor.model, Some("claude-3-opus".to_string()));
        assert_eq!(executor.round_size, 10);
        assert!(executor.validate);
        assert_eq!(executor.verify_command, Some("cargo test".to_string()));
    }

    #[test]
    fn test_executor_new_minimal() {
        let spec_config = SpecConfig::default();
        let executor = RalphExecutor::new(
            "minimal".to_string(),
            spec_config,
            None,
            "opencode".to_string(),
            None,
            5,
            false,
            PathBuf::from("."),
        )
        .unwrap();

        assert_eq!(executor.scud_tag, "minimal");
        assert_eq!(executor.harness_name, "opencode");
        assert!(executor.model.is_none());
        assert!(executor.verify_command.is_none());
        assert!(!executor.validate);
    }

    #[test]
    fn test_compute_waves_circular_dependency() {
        let mut phase = Phase::new("circular".to_string());

        // Create circular dependency: A -> B -> C -> A
        let mut task_a = Task::new("a".to_string(), "Task A".to_string(), "".to_string());
        task_a.dependencies = vec!["c".to_string()];

        let mut task_b = Task::new("b".to_string(), "Task B".to_string(), "".to_string());
        task_b.dependencies = vec!["a".to_string()];

        let mut task_c = Task::new("c".to_string(), "Task C".to_string(), "".to_string());
        task_c.dependencies = vec!["b".to_string()];

        phase.add_task(task_a);
        phase.add_task(task_b);
        phase.add_task(task_c);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        // Circular dependencies should result in incomplete waves
        // The algorithm should detect this and break out
        assert!(
            waves.is_empty(),
            "Circular dependencies should produce no complete waves"
        );
    }

    #[test]
    fn test_compute_waves_external_dependency() {
        // Test tasks with dependencies on tasks not in the phase
        let mut phase = Phase::new("external".to_string());

        // Task depends on non-existent task "external-1"
        let mut task1 = Task::new("1".to_string(), "Task 1".to_string(), "".to_string());
        task1.dependencies = vec!["external-1".to_string()];

        let task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());

        phase.add_task(task1);
        phase.add_task(task2);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        // External dependencies are ignored (treated as satisfied)
        // so both tasks should be in wave 1
        assert_eq!(waves.len(), 1, "Should have 1 wave");
        assert_eq!(waves[0].len(), 2, "Both tasks should be in wave 1");
    }

    #[test]
    fn test_compute_waves_skips_expanded_tasks() {
        let mut phase = Phase::new("expanded".to_string());

        // Parent task that has been expanded into subtasks
        let mut parent = Task::new("1".to_string(), "Parent".to_string(), "".to_string());

        // Mark parent as having subtasks (subtasks field contains task IDs as strings)
        parent.subtasks = vec!["1.1".to_string(), "1.2".to_string()];

        // Independent task
        let task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());

        phase.add_task(parent);
        phase.add_task(task2);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        // Only task 2 should be in waves (parent is expanded and filtered out)
        assert_eq!(waves.len(), 1, "Should have 1 wave");
        assert_eq!(waves[0].len(), 1, "Only task 2 should be in wave");
        assert_eq!(waves[0][0].id, "2");
    }

    #[test]
    fn test_compute_waves_in_progress_filtered() {
        let mut phase = Phase::new("status".to_string());

        // In-progress task should be filtered out
        let mut task1 = Task::new("1".to_string(), "Task 1".to_string(), "".to_string());
        task1.set_status(TaskStatus::InProgress);

        // Pending task depending on the in-progress one
        let mut task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());
        task2.dependencies = vec!["1".to_string()];

        // Independent pending task
        let task3 = Task::new("3".to_string(), "Task 3".to_string(), "".to_string());

        phase.add_task(task1);
        phase.add_task(task2);
        phase.add_task(task3);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        // Task 1 is in-progress (filtered out)
        // Task 2 depends on task 1 which is not actionable, so dep is ignored
        // Task 3 is independent
        // Both task 2 and task 3 should be ready in wave 1
        assert_eq!(waves.len(), 1, "Should have 1 wave");
        assert_eq!(waves[0].len(), 2, "Tasks 2 and 3 should be in wave 1");
    }

    #[test]
    fn test_compute_waves_blocked_filtered() {
        let mut phase = Phase::new("blocked".to_string());

        // Blocked task should be filtered out (only Pending is actionable)
        let mut task1 = Task::new("1".to_string(), "Task 1".to_string(), "".to_string());
        task1.set_status(TaskStatus::Blocked);

        // Pending task
        let task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());

        phase.add_task(task1);
        phase.add_task(task2);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        // Only task 2 should be actionable
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].len(), 1);
        assert_eq!(waves[0][0].id, "2");
    }

    #[test]
    fn test_compute_waves_deterministic_order() {
        let mut phase = Phase::new("order".to_string());

        // Add tasks in non-alphabetical order
        phase.add_task(Task::new("z".to_string(), "Z".to_string(), "".to_string()));
        phase.add_task(Task::new("a".to_string(), "A".to_string(), "".to_string()));
        phase.add_task(Task::new("m".to_string(), "M".to_string(), "".to_string()));

        let executor = create_executor();

        // Run twice to verify determinism
        let waves1 = executor.compute_waves(&phase);
        let waves2 = executor.compute_waves(&phase);

        assert_eq!(waves1.len(), 1);
        assert_eq!(waves1[0].len(), 3);

        // Verify that running compute_waves produces the same result
        // (determinism from sorted ID processing)
        for (i, task) in waves1[0].iter().enumerate() {
            assert_eq!(task.id, waves2[0][i].id, "Order should be deterministic");
        }
    }

    #[test]
    fn test_compute_waves_diamond_dependency() {
        // Diamond pattern: A -> B, A -> C, B -> D, C -> D
        let mut phase = Phase::new("diamond".to_string());

        let task_a = Task::new("a".to_string(), "A".to_string(), "".to_string());

        let mut task_b = Task::new("b".to_string(), "B".to_string(), "".to_string());
        task_b.dependencies = vec!["a".to_string()];

        let mut task_c = Task::new("c".to_string(), "C".to_string(), "".to_string());
        task_c.dependencies = vec!["a".to_string()];

        let mut task_d = Task::new("d".to_string(), "D".to_string(), "".to_string());
        task_d.dependencies = vec!["b".to_string(), "c".to_string()];

        phase.add_task(task_a);
        phase.add_task(task_b);
        phase.add_task(task_c);
        phase.add_task(task_d);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        assert_eq!(waves.len(), 3, "Diamond should have 3 waves");
        assert_eq!(waves[0].len(), 1, "Wave 1: A");
        assert_eq!(waves[0][0].id, "a");
        assert_eq!(waves[1].len(), 2, "Wave 2: B, C (parallel)");
        assert_eq!(waves[2].len(), 1, "Wave 3: D");
        assert_eq!(waves[2][0].id, "d");
    }

    #[test]
    fn test_compute_waves_long_chain() {
        // Linear chain: 1 -> 2 -> 3 -> 4 -> 5
        let mut phase = Phase::new("chain".to_string());

        let task1 = Task::new("1".to_string(), "1".to_string(), "".to_string());

        let mut task2 = Task::new("2".to_string(), "2".to_string(), "".to_string());
        task2.dependencies = vec!["1".to_string()];

        let mut task3 = Task::new("3".to_string(), "3".to_string(), "".to_string());
        task3.dependencies = vec!["2".to_string()];

        let mut task4 = Task::new("4".to_string(), "4".to_string(), "".to_string());
        task4.dependencies = vec!["3".to_string()];

        let mut task5 = Task::new("5".to_string(), "5".to_string(), "".to_string());
        task5.dependencies = vec!["4".to_string()];

        phase.add_task(task1);
        phase.add_task(task2);
        phase.add_task(task3);
        phase.add_task(task4);
        phase.add_task(task5);

        let executor = create_executor();
        let waves = executor.compute_waves(&phase);

        // Each task should be in its own wave (no parallelism possible)
        assert_eq!(waves.len(), 5, "Linear chain should have 5 waves");
        for (i, wave) in waves.iter().enumerate() {
            assert_eq!(wave.len(), 1);
            assert_eq!(wave[0].id, (i + 1).to_string());
        }
    }

    // ============ TaskResult parsing tests ============

    #[test]
    fn test_parse_task_result_completed() {
        let response = "I've implemented the feature successfully.\n\nAll tests pass.";
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Completed { response: resp } => {
                assert_eq!(resp, response);
            }
            TaskResult::Blocked { .. } => panic!("Expected Completed, got Blocked"),
        }
    }

    #[test]
    fn test_parse_task_result_blocked_simple() {
        let response =
            "I tried to implement the feature.\n\nTASK_BLOCKED: Missing dependency on auth module";
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Blocked { reason } => {
                assert_eq!(reason, "Missing dependency on auth module");
            }
            TaskResult::Completed { .. } => panic!("Expected Blocked, got Completed"),
        }
    }

    #[test]
    fn test_parse_task_result_blocked_with_whitespace() {
        let response = "  TASK_BLOCKED:   API endpoint unavailable  ";
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Blocked { reason } => {
                assert_eq!(reason, "API endpoint unavailable");
            }
            TaskResult::Completed { .. } => panic!("Expected Blocked, got Completed"),
        }
    }

    #[test]
    fn test_parse_task_result_blocked_no_reason() {
        let response = "TASK_BLOCKED:";
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Blocked { reason } => {
                assert_eq!(reason, "No reason provided");
            }
            TaskResult::Completed { .. } => panic!("Expected Blocked, got Completed"),
        }
    }

    #[test]
    fn test_parse_task_result_blocked_empty_reason() {
        let response = "TASK_BLOCKED:   ";
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Blocked { reason } => {
                assert_eq!(reason, "No reason provided");
            }
            TaskResult::Completed { .. } => panic!("Expected Blocked, got Completed"),
        }
    }

    #[test]
    fn test_parse_task_result_blocked_at_end() {
        // TASK_BLOCKED at the end of a long response
        let response = r#"I attempted to implement the feature.

Here's what I tried:
1. First approach - didn't work
2. Second approach - failed due to missing module

After multiple attempts, I cannot proceed.

TASK_BLOCKED: Requires auth module that doesn't exist yet"#;
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Blocked { reason } => {
                assert_eq!(reason, "Requires auth module that doesn't exist yet");
            }
            TaskResult::Completed { .. } => panic!("Expected Blocked, got Completed"),
        }
    }

    #[test]
    fn test_parse_task_result_multiple_blocked_lines() {
        // Last TASK_BLOCKED line wins (scanning from end)
        let response = r#"TASK_BLOCKED: First reason (old)
Some more work...
TASK_BLOCKED: Final reason"#;
        let result = RalphExecutor::parse_task_result(response);

        match result {
            TaskResult::Blocked { reason } => {
                assert_eq!(reason, "Final reason");
            }
            TaskResult::Completed { .. } => panic!("Expected Blocked, got Completed"),
        }
    }

    #[test]
    fn test_parse_task_result_not_at_line_start() {
        // TASK_BLOCKED mentioned mid-line should NOT trigger blocking
        let response = r#"The agent can output TASK_BLOCKED: when stuck.
But since it's not at the start of a line, this should be Completed."#;
        let result = RalphExecutor::parse_task_result(response);

        // Should be Completed since TASK_BLOCKED is not at line start (after "output ")
        match result {
            TaskResult::Completed { response: resp } => {
                assert!(resp.contains("should be Completed"));
            }
            TaskResult::Blocked { .. } => {
                panic!("Expected Completed since TASK_BLOCKED is mid-line")
            }
        }
    }

    #[test]
    fn test_parse_task_result_completed_with_similar_text() {
        // Text that mentions TASK_BLOCKED but not as a directive
        let response = "If you get stuck, you can output a TASK_BLOCKED message.\nBut I completed the task successfully!";
        let result = RalphExecutor::parse_task_result(response);

        // Should be Completed since TASK_BLOCKED is not at line start
        match result {
            TaskResult::Completed { response: resp } => {
                assert!(resp.contains("completed the task"));
            }
            TaskResult::Blocked { .. } => panic!("Expected Completed"),
        }
    }
}
