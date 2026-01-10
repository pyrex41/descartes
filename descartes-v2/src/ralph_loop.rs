//! Ralph Wiggum loop implementation
//!
//! The Ralph loop is the core execution pattern:
//! 1. Fresh context each iteration (prevents drift)
//! 2. Two modes: Plan (analyze gaps) and Build (implement)
//! 3. Subagents for parallel search, single builder, validator backpressure
//! 4. Commit only when tests pass

use futures::StreamExt;
use std::process::Command;
use tracing::{debug, info, warn};

use crate::agent::{spawn_subagent, AgentCategory};
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
    /// Path to prompts directory
    pub prompts_dir: std::path::PathBuf,
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
            prompts_dir: std::path::PathBuf::from("prompts"),
            auto_commit: true,
            auto_push: false,
        }
    }
}

/// Run the Ralph loop
pub async fn run(loop_config: LoopConfig, config: &Config) -> Result<()> {
    let harness = create_harness(config)?;
    let mut iteration = 0;

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
                plan_iteration(&*harness, &mut transcript, &loop_config, config).await
            }
            LoopMode::Build => {
                build_iteration(&*harness, &mut transcript, &loop_config, config).await
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
    transcript: &mut Transcript,
    loop_config: &LoopConfig,
    config: &Config,
) -> Result<IterationResult> {
    info!("Running planning iteration");

    // Load planning prompt
    let prompt_path = loop_config.prompts_dir.join("plan.md");
    let prompt = if prompt_path.exists() {
        std::fs::read_to_string(&prompt_path)?
    } else {
        include_str!("../prompts/plan.md").to_string()
    };

    // Create session
    let session_config = SessionConfig {
        model: "opus".to_string(), // Planning needs strong reasoning
        tools: vec!["read".to_string(), "bash".to_string()],
        system_prompt: Some(prompt.clone()),
        parent: None,
        is_subagent: false,
    };

    let session = harness.start_session(session_config).await?;
    transcript.record_user_message(&prompt);

    // Run planning agent
    let mut response = harness.send(&session, &prompt).await?;

    while let Some(chunk) = response.next().await {
        transcript.record_chunk(&chunk);

        match chunk {
            ResponseChunk::SubagentSpawn(req) => {
                // Handle subagent spawn during planning
                info!("Planning spawning {} subagent", req.category);
                let category: AgentCategory = req.category.parse()?;
                let result = spawn_subagent(harness, category, req.prompt, Some(transcript)).await?;
                debug!("Subagent result: {}", result.summary());
            }
            ResponseChunk::Done => break,
            ResponseChunk::Error(e) => {
                return Err(Error::Harness(e));
            }
            _ => {}
        }
    }

    harness.close_session(&session).await?;

    Ok(IterationResult::Completed)
}

/// Run a building iteration
async fn build_iteration(
    harness: &dyn Harness,
    transcript: &mut Transcript,
    loop_config: &LoopConfig,
    config: &Config,
) -> Result<IterationResult> {
    // Get next task from SCUD
    let task = match scud::next(config)? {
        Some(t) => t,
        None => {
            info!("No tasks ready");
            return Ok(IterationResult::NoTasksReady);
        }
    };

    info!("Working on task {}: {}", task.id, task.title);

    // Phase 1: Parallel searchers
    info!("Phase 1: Running parallel searchers");
    let search_results = run_parallel_searches(harness, &task, transcript).await?;

    // Phase 2: Single builder
    info!("Phase 2: Running builder");
    let build_result = run_builder(harness, &task, &search_results, transcript, loop_config).await?;

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
    info!("Task {} marked complete", task.id);

    // Git commit
    if loop_config.auto_commit {
        git_commit(&task.title)?;

        if loop_config.auto_push {
            git_push()?;
        }
    }

    Ok(IterationResult::Completed)
}

/// Run parallel search subagents
async fn run_parallel_searches(
    harness: &dyn Harness,
    task: &scud::Task,
    transcript: &mut Transcript,
) -> Result<Vec<String>> {
    use futures::future::join_all;

    let searches = vec![
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
            format!(
                "Analyze the codebase structure relevant to: {}",
                task.title
            ),
        ),
    ];

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
    loop_config: &LoopConfig,
) -> Result<bool> {
    // Load build prompt
    let prompt_path = loop_config.prompts_dir.join("build.md");
    let base_prompt = if prompt_path.exists() {
        std::fs::read_to_string(&prompt_path)?
    } else {
        include_str!("../prompts/build.md").to_string()
    };

    // Construct full prompt with task and context
    let context_str = search_context.join("\n\n---\n\n");
    let prompt = format!(
        "{}\n\n## Current Task\n\n**{}**: {}\n\n## Search Results\n\n{}",
        base_prompt, task.title, task.description, context_str
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

/// Create a git commit
fn git_commit(message: &str) -> Result<()> {
    info!("Creating git commit: {}", message);

    // Stage all changes
    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .map_err(|e| Error::Io(e))?;

    if !status.success() {
        warn!("git add failed");
        return Ok(()); // Non-fatal
    }

    // Check if there are changes to commit
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map_err(|e| Error::Io(e))?;

    if diff_output.success() {
        info!("No changes to commit");
        return Ok(());
    }

    // Commit
    let commit_status = Command::new("git")
        .args(["commit", "-m", message])
        .status()
        .map_err(|e| Error::Io(e))?;

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
