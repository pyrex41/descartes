//! Ralph Wiggum loop implementation
//!
//! The Ralph loop is the core execution pattern:
//! 1. Fresh context each iteration (prevents drift)
//! 2. Two modes: Plan (analyze gaps) and Build (implement)
//! 3. Subagents for parallel search, single builder, validator backpressure
//! 4. Commit only when tests pass
//!
//! Uses BAML for structured LLM interactions:
//! - DecideNextAction: Determines loop flow
//! - SelectSubagent: Routes tasks to appropriate agents
//! - GenerateCommitMessage: Creates conventional commits

use futures::StreamExt;
use std::process::Command;
use tracing::{debug, info, warn};

use crate::agent::{spawn_subagent, AgentCategory};
use crate::baml::{
    BamlClient, CreatePlanRequest, DecideNextActionRequest, GenerateCommitMessageRequest,
    NextAction, SelectSubagentRequest, SubagentCategory,
};
use crate::harness::{create_harness, Harness, ResponseChunk, SessionConfig};
use crate::scud;
use crate::transcript::Transcript;
use crate::{Config, Error, Result};

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
    /// BAML server URL (default: http://localhost:2024)
    pub baml_url: Option<String>,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            mode: LoopMode::Build,
            max_iterations: None,
            auto_commit: true,
            auto_push: false,
            baml_url: None,
        }
    }
}

/// Run the Ralph loop
pub async fn run(loop_config: LoopConfig, config: &Config) -> Result<()> {
    let harness = create_harness(config)?;

    // Create BAML client
    let baml = match &loop_config.baml_url {
        Some(url) => BamlClient::with_url(url),
        None => BamlClient::new(),
    };

    // Verify BAML server is running
    match baml.health_check().await {
        Ok(version) => info!("BAML server connected: {}", version),
        Err(e) => {
            warn!("BAML server not available: {}. Loop decisions will use defaults.", e);
        }
    }

    let mut iteration = 0;
    let mut completed_tasks: Vec<String> = Vec::new();

    info!(
        "Starting Ralph loop in {:?} mode",
        loop_config.mode
    );

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
            LoopMode::Plan => {
                plan_iteration(&*harness, &baml, &mut transcript, config).await
            }
            LoopMode::Build => {
                build_iteration(&*harness, &baml, &mut transcript, &loop_config, config, &mut completed_tasks).await
            }
        };

        // Finalize and save transcript
        transcript.finalize();
        let transcript_path = config.transcript_dir().join(format!(
            "{}.scg",
            transcript.id()
        ));
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
    baml: &BamlClient,
    transcript: &mut Transcript,
    config: &Config,
) -> Result<IterationResult> {
    info!("Running planning iteration");

    // Get current task state from SCUD
    let tasks = scud::list_tasks(config)?;
    let completed: Vec<String> = tasks.iter()
        .filter(|t| t.status == scud::TaskStatus::Done)
        .map(|t| t.title.clone())
        .collect();
    let remaining: Vec<String> = tasks.iter()
        .filter(|t| t.status != scud::TaskStatus::Done)
        .map(|t| t.title.clone())
        .collect();

    // Use BAML to create a plan
    let plan_req = CreatePlanRequest {
        objective: remaining.first().cloned().unwrap_or_else(|| "Analyze project gaps".to_string()),
        research_context: format!("Completed: {:?}\nRemaining: {:?}", completed, remaining),
        constraints: None,
        additional_context: None,
    };

    match baml.create_plan(plan_req).await {
        Ok(plan) => {
            info!("Generated plan: {}", plan.goal);
            info!("Approach: {}", plan.approach);
            for task in &plan.tasks {
                info!("  - {}: {} ({:?})", task.id, task.title, task.complexity);
            }
            transcript.record_assistant_message(&format!(
                "Plan: {}\n\nTasks:\n{}",
                plan.goal,
                plan.tasks.iter().map(|t| format!("- {}: {}", t.id, t.title)).collect::<Vec<_>>().join("\n")
            ));
        }
        Err(e) => {
            warn!("BAML planning failed, falling back to harness: {}", e);
            // Fallback: use harness directly
            let session_config = SessionConfig {
                model: "opus".to_string(),
                tools: vec!["read".to_string(), "bash".to_string()],
                system_prompt: Some("You are a planning agent. Analyze the project and identify gaps.".to_string()),
                parent: None,
                is_subagent: false,
            };

            let session = harness.start_session(session_config).await?;
            let prompt = format!("Create a plan. Completed: {:?}, Remaining: {:?}", completed, remaining);
            transcript.record_user_message(&prompt);

            let mut response = harness.send(&session, &prompt).await?;
            while let Some(chunk) = response.next().await {
                transcript.record_chunk(&chunk);
                match chunk {
                    ResponseChunk::SubagentSpawn(req) => {
                        info!("Planning spawning {} subagent", req.category);
                        let category: AgentCategory = req.category.parse()?;
                        let result = spawn_subagent(harness, category, req.prompt, Some(transcript)).await?;
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
    baml: &BamlClient,
    transcript: &mut Transcript,
    loop_config: &LoopConfig,
    config: &Config,
    completed_tasks: &mut Vec<String>,
) -> Result<IterationResult> {
    // Get current task state from SCUD
    let tasks = scud::list_tasks(config)?;
    let remaining: Vec<String> = tasks.iter()
        .filter(|t| t.status != scud::TaskStatus::Done)
        .map(|t| t.title.clone())
        .collect();

    // Use BAML to decide next action
    let decision_req = DecideNextActionRequest {
        completed_tasks: completed_tasks.clone(),
        current_task: None,
        remaining_tasks: remaining.clone(),
        blockers: vec![],
        recent_output: "Starting new iteration".to_string(),
        additional_context: None,
    };

    let decision = match baml.decide_next_action(decision_req).await {
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
                return plan_iteration(harness, baml, transcript, config).await;
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

    // Phase 1: Use BAML to select subagents dynamically
    info!("Phase 1: Running parallel searchers");
    let search_results = run_parallel_searches_baml(harness, baml, &task, transcript).await?;

    // Phase 2: Single builder
    info!("Phase 2: Running builder");
    let build_result = run_builder(harness, &task, &search_results, transcript).await?;

    if !build_result {
        warn!("Builder failed");
        return Ok(IterationResult::ValidationFailed);
    }

    // Phase 3: Validator (backpressure)
    info!("Phase 3: Running validator");
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
        git_commit_baml(baml, &task.title).await?;

        if loop_config.auto_push {
            git_push()?;
        }
    }

    Ok(IterationResult::Completed)
}

/// Run parallel search subagents using BAML for dynamic selection
async fn run_parallel_searches_baml(
    harness: &dyn Harness,
    baml: &BamlClient,
    task: &scud::Task,
    transcript: &mut Transcript,
) -> Result<Vec<String>> {
    use futures::future::join_all;

    // Use BAML to select subagents
    let select_req = SelectSubagentRequest {
        task_title: task.title.clone(),
        task_description: task.description.clone(),
        available_context: "Starting fresh iteration, need to search codebase".to_string(),
        additional_context: None,
    };

    // Try to get BAML suggestions, fall back to defaults
    let searches: Vec<(AgentCategory, String)> = match baml.select_subagent(select_req).await {
        Ok(selection) => {
            info!("BAML selected {:?} subagent: {}", selection.category, selection.timeout_hint);
            let category = match selection.category {
                SubagentCategory::Searcher => AgentCategory::Searcher,
                SubagentCategory::Analyzer => AgentCategory::Analyzer,
                SubagentCategory::Builder => AgentCategory::Builder,
                SubagentCategory::Validator => AgentCategory::Validator,
            };
            // Start with BAML suggestion, add standard searches
            vec![
                (category, selection.prompt),
                (AgentCategory::Searcher, format!("Find tests related to: {}", task.title)),
            ]
        }
        Err(e) => {
            debug!("BAML subagent selection unavailable: {}, using defaults", e);
            vec![
                (AgentCategory::Searcher, format!("Find existing implementations related to: {}", task.title)),
                (AgentCategory::Searcher, format!("Find tests related to: {}", task.title)),
                (AgentCategory::Analyzer, format!("Analyze the codebase structure relevant to: {}", task.title)),
            ]
        }
    };

    let futures: Vec<_> = searches
        .into_iter()
        .map(|(category, prompt)| spawn_subagent(harness, category, prompt, None))
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
    transcript: &mut Transcript,
) -> Result<bool> {
    // Construct prompt with task and context (no markdown file loading)
    let context_str = search_context.join("\n\n---\n\n");
    let prompt = format!(
        "## Task\n\n**{}**: {}\n\n## Context from Search\n\n{}\n\n## Instructions\n\nImplement this task. Make minimal, focused changes. Run tests after changes.",
        task.title, task.description, context_str
    );

    let result = spawn_subagent(harness, AgentCategory::Builder, prompt, Some(transcript)).await?;

    Ok(result.success)
}

/// Run the validator subagent (backpressure gate)
async fn run_validator(harness: &dyn Harness, transcript: &mut Transcript) -> Result<bool> {
    let prompt = "Run the test suite and report results. Use `cargo test` or the appropriate test command for this project.";

    let result =
        spawn_subagent(harness, AgentCategory::Validator, prompt.to_string(), Some(transcript))
            .await?;

    Ok(result.passed())
}

/// Create a git commit using BAML to generate the message
async fn git_commit_baml(baml: &BamlClient, fallback_message: &str) -> Result<()> {
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

    // Use BAML to generate commit message
    let commit_req = GenerateCommitMessageRequest {
        diff: diff_text,
        context: Some(fallback_message.to_string()),
        additional_context: None,
    };

    let message = match baml.generate_commit_message(commit_req).await {
        Ok(msg) => {
            let scope_part = msg.scope.map(|s| format!("({})", s)).unwrap_or_default();
            let breaking = if msg.breaking { "!" } else { "" };
            let body_part = msg.body.map(|b| format!("\n\n{}", b)).unwrap_or_default();
            format!("{}{}{}: {}{}", msg.commit_type.as_str(), scope_part, breaking, msg.subject, body_part)
        }
        Err(e) => {
            debug!("BAML commit message generation failed: {}, using fallback", e);
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

impl crate::baml::CommitType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Feat => "feat",
            Self::Fix => "fix",
            Self::Docs => "docs",
            Self::Refactor => "refactor",
            Self::Test => "test",
            Self::Chore => "chore",
        }
    }
}

/// Push to remote
fn git_push() -> Result<()> {
    info!("Pushing to remote");

    let status = Command::new("git")
        .args(["push"])
        .status()
        .map_err(|e| Error::Io(e))?;

    if !status.success() {
        // Try with -u origin
        let branch_output = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .map_err(|e| Error::Io(e))?;

        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        let _ = Command::new("git")
            .args(["push", "-u", "origin", &branch])
            .status();
    }

    Ok(())
}
