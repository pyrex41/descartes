//! Ralph Wiggum loop implementation
//!
//! The Ralph loop is the core execution pattern:
//! 1. Fresh context each iteration (prevents drift)
//! 2. Two modes: Plan (analyze gaps) and Build (implement)
//! 3. Subagents for parallel search, single builder, validator backpressure
//! 4. Commit only when tests pass
//!
//! Uses BAML for structured LLM interactions (native Rust codegen):
//! - DecideNextAction: Determines loop flow
//! - SelectSubagent: Routes tasks to appropriate agents
//! - GenerateCommitMessage: Creates conventional commits

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

use crate::agent::{spawn_subagent, AgentCategory, SubagentResult};
use crate::baml_client::async_client::B;
use crate::baml_client::types::{
    NextAction, Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator,
    Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest,
};
use crate::harness::{create_harness, Harness, ResponseChunk, SessionConfig};
use crate::scud;
use crate::transcript::Transcript;
use crate::{Config, Error, Result};

/// Task-specific overrides parsed from task body
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskOverrides {
    /// Override category (e.g. "fast-builder", "builder")
    pub category: Option<String>,
    /// Disable review even if config says to review
    pub disable_review: Option<bool>,
}

impl TaskOverrides {
    /// Parse overrides from task description/body
    /// Supports YAML frontmatter (---\ncategory: fast-builder\n---) or inline comments (// override: category=fast-builder)
    pub fn parse(body: &str) -> Self {
        // Try YAML frontmatter first
        if let Some(yaml_end) = body.find("\n---") {
            if body.starts_with("---\n") {
                let yaml_str = &body[4..yaml_end];
                if let Ok(overrides) = serde_yaml::from_str::<TaskOverrides>(yaml_str) {
                    return overrides;
                }
            }
        }

        // Fallback to inline comment parsing
        let mut category = None;
        let mut disable_review = None;

        for line in body.lines() {
            let line = line.trim();
            if line.starts_with("// override:") {
                let rest = &line[12..].trim();
                for kv in rest.split(',') {
                    let kv = kv.trim();
                    if let Some((key, value)) = kv.split_once('=') {
                        match key.trim() {
                            "category" => category = Some(value.trim().to_string()),
                            "disable_review" => disable_review = value.trim().parse().ok(),
                            _ => {}
                        }
                    }
                }
            }
        }

        Self {
            category,
            disable_review,
        }
    }
}

/// Loop mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// Planning mode: analyze gaps, update task graph
    Plan,
    /// Building mode: pick task, implement, validate, commit
    #[default]
    Build,
}

/// Loop configuration
#[derive(Debug, Clone)]
pub struct LoopConfig {
    /// Which mode to run in
    pub mode: LoopMode,
    /// Maximum iterations (None = infinite)
    pub max_iterations: Option<usize>,
    /// Whether to auto-commit on success
    pub auto_commit: bool,
    /// Whether to auto-push after commit
    pub auto_push: bool,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            mode: LoopMode::Build,
            max_iterations: None,
            auto_commit: true,
            auto_push: false,
        }
    }
}

/// Run the Ralph loop
pub async fn run(loop_config: LoopConfig, config: &Config) -> Result<()> {
    let harness = create_harness(config)?;

    // Initialize BAML runtime (lazy - happens on first call)
    crate::baml_client::init();
    info!("BAML native client initialized");

    let mut iteration = 0;
    let mut completed_tasks: Vec<String> = Vec::new();

    info!("Starting Ralph loop in {:?} mode", loop_config.mode);

    loop {
        // Check iteration limit
        if let Some(max) = loop_config.max_iterations {
            if iteration >= max {
                info!("Reached max iterations: {}", max);
                break;
            }
        }

        info!("=== Iteration {} ===", iteration + 1);

        // Create transcript for this iteration
        let mut transcript = Transcript::new()
            .with_harness(harness.name())
            .with_model(&config.harness.claude_code.model);

        // Run appropriate mode
        let result = match loop_config.mode {
            LoopMode::Plan => plan_iteration(&*harness, &mut transcript, config).await,
            LoopMode::Build => {
                build_iteration(
                    &*harness,
                    &mut transcript,
                    &loop_config,
                    config,
                    &mut completed_tasks,
                )
                .await
            }
        };

        // Finalize and save transcript
        transcript.finalize();
        let transcript_path = config
            .transcript_dir()
            .join(format!("{}.scg", transcript.id()));
        if let Err(e) = transcript.save_scg(&transcript_path) {
            warn!("Failed to save transcript: {}", e);
        }

        // Handle result
        match result {
            Ok(IterationResult::Completed) => {
                info!("Iteration {} completed successfully", iteration + 1);
            }
            Ok(IterationResult::NoTasksReady) => {
                info!("No tasks ready, exiting loop");
                break;
            }
            Ok(IterationResult::ValidationFailed) => {
                warn!("Validation failed, will retry next iteration");
            }
            Err(e) => {
                warn!("Iteration {} failed: {}", iteration + 1, e);
                // Continue to next iteration unless it's a fatal error
            }
        }

        iteration += 1;
    }

    info!("Ralph loop completed after {} iterations", iteration);
    Ok(())
}

/// Result of a single iteration
enum IterationResult {
    /// Iteration completed successfully (task done and committed)
    Completed,
    /// No tasks were ready to work on
    NoTasksReady,
    /// Validation (tests) failed
    ValidationFailed,
}

/// Run a planning iteration
async fn plan_iteration(
    harness: &dyn Harness,
    transcript: &mut Transcript,
    config: &Config,
) -> Result<IterationResult> {
    info!("Running planning iteration");

    // Get current task state from SCUD
    let tasks = scud::list_tasks(config)?;
    let completed: Vec<String> = tasks
        .iter()
        .filter(|t| t.status == scud::TaskStatus::Done)
        .map(|t| t.title.clone())
        .collect();
    let remaining: Vec<String> = tasks
        .iter()
        .filter(|t| t.status != scud::TaskStatus::Done)
        .map(|t| t.title.clone())
        .collect();

    let objective = remaining
        .first()
        .cloned()
        .unwrap_or_else(|| "Analyze project gaps".to_string());
    let research_context = format!("Completed: {:?}\nRemaining: {:?}", completed, remaining);

    // Use native BAML client to create a plan
    match B
        .CreatePlan
        .call(&objective, &research_context, None::<&str>, None::<&str>)
        .await
    {
        Ok(plan) => {
            info!("Generated plan: {}", plan.goal);
            info!("Approach: {}", plan.approach);
            for task in &plan.tasks {
                info!("  - {}: {} ({:?})", task.id, task.title, task.complexity);
            }
            transcript.record_assistant_message(&format!(
                "Plan: {}\n\nTasks:\n{}",
                plan.goal,
                plan.tasks
                    .iter()
                    .map(|t| format!("- {}: {}", t.id, t.title))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        Err(e) => {
            warn!("BAML planning failed, falling back to harness: {}", e);
            // Fallback: use harness directly
            let session_config = SessionConfig {
                model: "opus".to_string(),
                tools: vec!["read".to_string(), "bash".to_string()],
                system_prompt: Some(
                    "You are a planning agent. Analyze the project and identify gaps.".to_string(),
                ),
                parent: None,
                is_subagent: false,
            };

            let session = harness.start_session(session_config).await?;
            let prompt = format!(
                "Create a plan. Completed: {:?}, Remaining: {:?}",
                completed, remaining
            );
            transcript.record_user_message(&prompt);

            let mut response = harness.send(&session, &prompt).await?;
            while let Some(chunk) = response.next().await {
                transcript.record_chunk(&chunk);
                match chunk {
                    ResponseChunk::SubagentSpawn(req) => {
                        info!("Planning spawning {} subagent", req.category);
                        let category: AgentCategory = req.category.parse()?;
                        let result =
                            spawn_subagent(harness, category, req.prompt, Some(transcript), None).await?;
                        debug!("Subagent result: {}", result.summary());
                    }
                    ResponseChunk::Done => break,
                    ResponseChunk::Error(e) => return Err(Error::Harness(e)),
                    _ => {}
                }
            }
            harness.close_session(&session).await?;
        }
    }

    Ok(IterationResult::Completed)
}

/// Run a building iteration
async fn build_iteration(
    harness: &dyn Harness,
    transcript: &mut Transcript,
    loop_config: &LoopConfig,
    config: &Config,
    completed_tasks: &mut Vec<String>,
) -> Result<IterationResult> {
    // Get current task state from SCUD
    let tasks = scud::list_tasks(config)?;
    let remaining: Vec<String> = tasks
        .iter()
        .filter(|t| t.status != scud::TaskStatus::Done)
        .map(|t| t.title.clone())
        .collect();

    // Use BAML to decide next action
    let decision = match B
        .DecideNextAction
        .call(
            completed_tasks,
            None::<&str>,
            &remaining,
            &Vec::<String>::new(),
            "Starting new iteration",
            None::<&str>,
        )
        .await
    {
        Ok(d) => {
            info!("BAML decision: {:?} - {}", d.action, d.reasoning);
            Some(d)
        }
        Err(e) => {
            debug!("BAML decision unavailable: {}, using default flow", e);
            None
        }
    };

    // Handle BAML-driven decisions
    if let Some(ref d) = decision {
        match d.action {
            NextAction::Complete => {
                info!("BAML says all work is done");
                return Ok(IterationResult::NoTasksReady);
            }
            NextAction::AskHuman => {
                info!("BAML requests human input: {:?}", d.message);
                return Ok(IterationResult::NoTasksReady);
            }
            NextAction::Replan => {
                info!("BAML suggests replanning, switching to plan mode");
                return plan_iteration(harness, transcript, config).await;
            }
            _ => {} // Continue or Validate - proceed with build
        }
    }

    // Get next task from SCUD
    let task = match scud::next(config)? {
        Some(t) => t,
        None => {
            info!("No tasks ready");
            return Ok(IterationResult::NoTasksReady);
        }
    };

    info!("Working on task {}: {}", task.id, task.title);

    // Parse task overrides from body
    let overrides = TaskOverrides::parse(&task.description);
    info!("Task overrides: {:?}", overrides);

    // Phase 1: Use BAML to select subagents dynamically
    info!("Phase 1: Running parallel searchers");
    let search_results = run_parallel_searches_baml(harness, &task, transcript).await?;

    // Decide implementation category (override > BAML > config)
    let impl_category = if let Some(cat) = &overrides.category {
        cat.clone()
    } else {
        // Use BAML orchestrator to suggest category
        match B
            .SelectSubagent
            .call(
                &task.title,
                &task.description,
                "Choose implementation category",
                Some(&config.ralph_loop.heuristic),
            )
            .await
        {
            Ok(selection) => {
                info!("BAML selected category: {:?}", selection.category);
                match selection.category {
                    Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Kbuilder => {
                        "builder".to_string()
                    }
                    Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Kanalyzer => {
                        "analyzer".to_string()
                    }
                    Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Ksearcher => {
                        "searcher".to_string()
                    }
                    Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Kvalidator => {
                        "validator".to_string()
                    }
                }
            }
            Err(e) => {
                debug!("BAML category selection failed: {}, using heuristic", e);
                if config.ralph_loop.use_fast_first {
                    "fast-builder".to_string()
                } else {
                    "builder".to_string()
                }
            }
        }
    };

    info!("Using implementation category: {}", impl_category);

    // Phase 2: Implementation
    info!("Phase 2: Running {}", impl_category);
    let impl_result =
        run_builder(harness, &task, &search_results, &impl_category, transcript).await?;

    if !impl_result.success {
        warn!("{} failed", impl_category);
        return Ok(IterationResult::ValidationFailed);
    }

    // Capture implementation summary
    let impl_summary = impl_result.summary();

    // Stage changes for review
    if Command::new("git")
        .args(["add", "-A"])
        .status()
        .map_err(Error::Io)?
        .success()
    {
        // Phase 3: Conditional review
        let needs_review = config.ralph_loop.always_review
            || (impl_category == "fast-builder" && overrides.disable_review != Some(true));

        if needs_review {
            info!("Phase 3: Running reviewer");
            let review_passed = run_reviewer(
                harness,
                &task,
                &search_results,
                &impl_summary,
                &config.ralph_loop.heuristic,
                transcript,
            )
            .await?;
            if !review_passed {
                warn!("Review failed");
                return Ok(IterationResult::ValidationFailed);
            }
        } else {
            info!("Skipping review as configured");
        }
    } else {
        warn!("git add failed");
    }

    // Phase 4: Validator (backpressure)
    info!("Phase 4: Running validator");
    let validation_passed = run_validator(harness, transcript).await?;

    if !validation_passed {
        warn!("Validation failed");
        return Ok(IterationResult::ValidationFailed);
    }

    // Mark task complete
    scud::complete(config, &task.id)?;
    completed_tasks.push(task.title.clone());
    info!("Task {} marked complete", task.id);

    // Git commit using BAML for message generation
    if loop_config.auto_commit {
        git_commit_baml(&task.title).await?;

        if loop_config.auto_push {
            git_push()?;
        }
    }

    Ok(IterationResult::Completed)
}

/// Run parallel search subagents using BAML for dynamic selection
async fn run_parallel_searches_baml(
    harness: &dyn Harness,
    task: &scud::Task,
    transcript: &mut Transcript,
) -> Result<Vec<String>> {
    use futures::future::join_all;

    // Try to get BAML suggestions, fall back to defaults
    let searches: Vec<(AgentCategory, String)> = match B
        .SelectSubagent
        .call(
            &task.title,
            &task.description,
            "Starting fresh iteration, need to search codebase",
            None::<&str>,
        )
        .await
    {
        Ok(selection) => {
            info!(
                "BAML selected {:?} subagent: {}",
                selection.category, selection.timeout_hint
            );
            let category = match selection.category {
                Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Ksearcher => {
                    AgentCategory::Searcher
                }
                Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Kanalyzer => {
                    AgentCategory::Analyzer
                }
                Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Kbuilder => {
                    AgentCategory::Builder
                }
                Union4KanalyzerOrKbuilderOrKsearcherOrKvalidator::Kvalidator => {
                    AgentCategory::Validator
                }
            };
            // Start with BAML suggestion, add standard searches
            vec![
                (category, selection.prompt),
                (
                    AgentCategory::Searcher,
                    format!("Find tests related to: {}", task.title),
                ),
            ]
        }
        Err(e) => {
            debug!("BAML subagent selection unavailable: {}, using defaults", e);
            vec![
                (
                    AgentCategory::Searcher,
                    format!("Find existing implementations related to: {}", task.title),
                ),
                (
                    AgentCategory::Searcher,
                    format!("Find tests related to: {}", task.title),
                ),
                (
                    AgentCategory::Analyzer,
                    format!("Analyze the codebase structure relevant to: {}", task.title),
                ),
            ]
        }
    };

    let futures: Vec<_> = searches
        .into_iter()
        .map(|(category, prompt)| spawn_subagent(harness, category, prompt, None, None))
        .collect();

    let results = join_all(futures).await;

    // Collect successful results
    let mut outputs = Vec::new();
    for result in results {
        match result {
            Ok(r) => {
                transcript.record_subagent(&r.session_id, "searcher", &r.output);
                outputs.push(r.output);
            }
            Err(e) => {
                warn!("Search subagent failed: {}", e);
            }
        }
    }

    Ok(outputs)
}

/// Run the builder subagent
async fn run_builder(
    harness: &dyn Harness,
    task: &scud::Task,
    search_context: &[String],
    category: &str,
    transcript: &mut Transcript,
) -> Result<SubagentResult> {
    // Map category string to AgentCategory
    let agent_category = match category {
        "fast-builder" => AgentCategory::FastBuilder,
        "builder" => AgentCategory::Builder,
        _ => {
            warn!("Unknown category '{}', falling back to Builder", category);
            AgentCategory::Builder
        }
    };

    // Construct prompt with task and context (no markdown file loading)
    let context_str = search_context.join("\n\n---\n\n");
    let prompt = format!(
        "## Task\n\n**{}**: {}\n\n## Context from Search\n\n{}\n\n## Instructions\n\nImplement this task. Make minimal, focused changes. Run tests after changes.",
        task.title, task.description, context_str
    );

    let result = spawn_subagent(harness, agent_category, prompt, Some(transcript), None).await?;

    Ok(result)
}

/// Run the reviewer subagent for fast-builder changes
async fn run_reviewer(
    harness: &dyn Harness,
    task: &scud::Task,
    search_context: &[String],
    impl_summary: &str,
    heuristic: &str,
    transcript: &mut Transcript,
) -> Result<bool> {
    // Get staged changes for review
    let diff_output = Command::new("git")
        .args(["diff", "--cached"])
        .output()
        .map_err(Error::Io)?;
    let diff = String::from_utf8_lossy(&diff_output.stdout);

    // Construct review prompt
    let context_str = search_context.join("\n\n---\n\n");
    let prompt = format!(
        "## Task\n\n**{}**: {}\n\n## Context from Search\n\n{}\n\n## Implementation Summary\n\n{}\n\n## Changes Made\n\n```\n{}\n```\n\n## Instructions\n\nReview the implementation for quality, correctness, architecture, security, and edge cases. Edit minimally if needed. Heuristic: {}\n\nConfirm the changes are appropriate and complete the task properly.",
        task.title, task.description, context_str, impl_summary, diff, heuristic
    );

    let result = spawn_subagent(
        harness,
        AgentCategory::BuilderReviewer,
        prompt,
        Some(transcript),
        None,
    )
    .await?;

    Ok(result.success)
}

/// Run the validator subagent (backpressure gate)
async fn run_validator(harness: &dyn Harness, transcript: &mut Transcript) -> Result<bool> {
    let prompt = "Run the test suite and report results. Use `cargo test` or the appropriate test command for this project.";

    let result = spawn_subagent(
        harness,
        AgentCategory::Validator,
        prompt.to_string(),
        Some(transcript),
        None,
    )
    .await?;

    Ok(result.passed())
}

/// Create a git commit using BAML to generate the message
async fn git_commit_baml(fallback_message: &str) -> Result<()> {
    // Stage all changes
    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .map_err(Error::Io)?;

    if !status.success() {
        warn!("git add failed");
        return Ok(()); // Non-fatal
    }

    // Check if there are changes to commit
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map_err(Error::Io)?;

    if diff_output.success() {
        info!("No changes to commit");
        return Ok(());
    }

    // Get diff for commit message generation
    let diff = Command::new("git")
        .args(["diff", "--cached", "--stat"])
        .output()
        .map_err(Error::Io)?;
    let diff_text = String::from_utf8_lossy(&diff.stdout).to_string();

    // Use native BAML client to generate commit message
    let message = match B
        .GenerateCommitMessage
        .call(&diff_text, Some(fallback_message), None::<&str>)
        .await
    {
        Ok(msg) => {
            let scope_part = msg.scope.map(|s| format!("({})", s)).unwrap_or_default();
            let breaking = if msg.breaking { "!" } else { "" };
            let body_part = msg.body.map(|b| format!("\n\n{}", b)).unwrap_or_default();
            let type_str = match msg.r#type {
                Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest::Kfeat => "feat",
                Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest::Kfix => "fix",
                Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest::Kdocs => "docs",
                Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest::Krefactor => "refactor",
                Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest::Ktest => "test",
                Union6KchoreOrKdocsOrKfeatOrKfixOrKrefactorOrKtest::Kchore => "chore",
            };
            format!(
                "{}{}{}: {}{}",
                type_str, scope_part, breaking, msg.subject, body_part
            )
        }
        Err(e) => {
            debug!(
                "BAML commit message generation failed: {}, using fallback",
                e
            );
            fallback_message.to_string()
        }
    };

    info!("Creating git commit: {}", message);

    // Commit
    let commit_status = Command::new("git")
        .args(["commit", "-m", &message])
        .status()
        .map_err(Error::Io)?;

    if !commit_status.success() {
        warn!("git commit failed");
    }

    Ok(())
}

/// Push to remote
fn git_push() -> Result<()> {
    info!("Pushing to remote");

    let status = Command::new("git")
        .args(["push"])
        .status()
        .map_err(Error::Io)?;

    if !status.success() {
        // Try with -u origin
        let branch_output = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .map_err(Error::Io)?;

        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        let _ = Command::new("git")
            .args(["push", "-u", "origin", &branch])
            .status();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_overrides_yaml_frontmatter() {
        let body = "---\ncategory: fast-builder\ndisable_review: true\n---\nFix the login bug.";
        let overrides = TaskOverrides::parse(body);
        assert_eq!(overrides.category, Some("fast-builder".to_string()));
        assert_eq!(overrides.disable_review, Some(true));
    }

    #[test]
    fn test_task_overrides_inline_comment() {
        let body = "// override: category=builder,disable_review=false\nImplement feature X.";
        let overrides = TaskOverrides::parse(body);
        assert_eq!(overrides.category, Some("builder".to_string()));
        assert_eq!(overrides.disable_review, Some(false));
    }

    #[test]
    fn test_task_overrides_no_overrides() {
        let body = "Just a regular task description without any overrides.";
        let overrides = TaskOverrides::parse(body);
        assert_eq!(overrides.category, None);
        assert_eq!(overrides.disable_review, None);
    }

    #[test]
    fn test_task_overrides_partial() {
        let body = "---\ncategory: fast-builder\n---\nOnly category specified.";
        let overrides = TaskOverrides::parse(body);
        assert_eq!(overrides.category, Some("fast-builder".to_string()));
        assert_eq!(overrides.disable_review, None);
    }
}
