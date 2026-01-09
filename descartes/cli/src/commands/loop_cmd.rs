//! CLI command for iterative agent loops (ralph-style)

use anyhow::Result;
use clap::{ArgAction, Args, Subcommand};
use descartes_core::{
    IterativeExitReason, IterativeLoop, IterativeLoopConfig, IterativeLoopState,
    LoopBackendConfig, LoopGitConfig, ScudIterativeLoop, ScudLoopConfig, LoopSpecConfig,
};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum LoopCommand {
    /// Start a new iterative loop
    Start(LoopStartArgs),
    /// Resume an existing loop
    Resume(LoopResumeArgs),
    /// Show status of current loop
    Status(LoopStatusArgs),
    /// Cancel a running loop
    Cancel(LoopCancelArgs),
}

#[derive(Debug, Args)]
pub struct LoopStartArgs {
    /// The command to run (e.g., "claude", "opencode", "python script.py")
    #[arg(short, long)]
    pub command: String,

    /// The task prompt
    #[arg(short, long)]
    pub prompt: String,

    /// Completion promise text (loop exits when this appears in output)
    #[arg(long, default_value = "COMPLETE")]
    pub completion_promise: String,

    /// Maximum iterations (safety limit)
    #[arg(short, long, default_value = "20")]
    pub max_iterations: u32,

    /// Working directory
    #[arg(short, long)]
    pub working_dir: Option<PathBuf>,

    /// Backend type: claude, opencode, or generic
    #[arg(long, default_value = "generic")]
    pub backend: String,

    /// Auto-commit after each iteration
    #[arg(long)]
    pub auto_commit: bool,

    /// Timeout per iteration in seconds
    #[arg(long)]
    pub timeout: Option<u64>,

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

#[derive(Debug, Args)]
pub struct LoopResumeArgs {
    /// Path to state file (default: .descartes/loop-state.json)
    #[arg(short, long)]
    pub state_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LoopStatusArgs {
    /// Path to state file
    #[arg(short, long)]
    pub state_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LoopCancelArgs {
    /// Path to state file
    #[arg(short, long)]
    pub state_file: Option<PathBuf>,
}

pub async fn execute(cmd: &LoopCommand) -> Result<()> {
    match cmd {
        LoopCommand::Start(args) => handle_start(args).await,
        LoopCommand::Resume(args) => handle_resume(args).await,
        LoopCommand::Status(args) => handle_status(args).await,
        LoopCommand::Cancel(args) => handle_cancel(args).await,
    }
}

async fn handle_start(args: &LoopStartArgs) -> Result<()> {
    use colored::Colorize;

    // Check if SCUD tag is provided
    if let Some(tag) = args.scud_tag.clone() {
        // Use SCUD-aware loop
        println!("{}", "Starting SCUD iterative loop...".cyan());
        println!("  SCUD tag: {}", tag.yellow());
        println!("  Max tokens: {}", args.max_spec_tokens);
        if let Some(ref plan) = args.plan {
            println!("  Plan: {:?}", plan);
        }
        if !args.spec_file.is_empty() {
            println!("  Spec files: {:?}", args.spec_file);
        }
        println!();

        let config = ScudLoopConfig {
            tag,
            plan_path: args.plan.clone(),
            working_directory: args.working_dir.clone().unwrap_or_else(|| PathBuf::from(".")),
            verification_command: args.verify.clone().or(Some("cargo check && cargo test".to_string())),
            spec: LoopSpecConfig {
                additional_specs: args.spec_file.clone(),
                max_spec_tokens: Some(args.max_spec_tokens),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut loop_exec = ScudIterativeLoop::new(config)?;
        let result = loop_exec.execute().await?;

        println!();
        println!("{}", "SCUD loop completed!".green());
        println!("  Exit reason: {:?}", result.exit_reason);
        println!("  Duration: {:?}", result.total_duration);
        if result.completion_promise_found {
            println!("  Completion: All tasks done");
        }

        Ok(())
    } else {
        // Use generic iterative loop (existing behavior)
        println!("{}", "Starting iterative loop...".cyan());
        println!("  Command: {}", args.command.yellow());
        println!(
            "  Prompt: {}...",
            args.prompt.chars().take(50).collect::<String>().dimmed()
        );
        println!("  Max iterations: {}", args.max_iterations);
        println!(
            "  Completion promise: {}",
            format!("<promise>{}</promise>", args.completion_promise).green()
        );
        println!();

        // Parse command into command + args
        let parts: Vec<&str> = args.command.split_whitespace().collect();
        let (command, cmd_args) = if parts.is_empty() {
            return Err(anyhow::anyhow!("Command cannot be empty"));
        } else {
            (
                parts[0].to_string(),
                parts[1..].iter().map(|s| s.to_string()).collect(),
            )
        };

        let config = IterativeLoopConfig {
            command,
            args: cmd_args,
            prompt: args.prompt.clone(),
            completion_promise: Some(args.completion_promise.clone()),
            max_iterations: Some(args.max_iterations),
            working_directory: args.working_dir.clone(),
            state_file: None,
            include_iteration_context: true,
            iteration_timeout_secs: args.timeout,
            backend: LoopBackendConfig {
                backend_type: args.backend.clone(),
                ..Default::default()
            },
            git: LoopGitConfig {
                auto_commit: args.auto_commit,
                ..Default::default()
            },
        };

        let mut loop_exec = IterativeLoop::new(config).await?;

        // Set up Ctrl+C handler
        let cancel_handle = loop_exec.cancellation_handle();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            println!("\n{}", "Received Ctrl+C, finishing current iteration...".yellow());
            cancel_handle.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let result = loop_exec.execute().await?;

        println!();
        println!("{}", "Loop completed!".green());
        println!("  Iterations: {}", result.iterations_completed);
        println!("  Exit reason: {:?}", result.exit_reason);
        println!("  Duration: {:?}", result.total_duration);
        if let Some(ref text) = result.completion_text {
            println!("  Completion text: {}", text);
        }

        Ok(())
    }
}

async fn handle_resume(args: &LoopResumeArgs) -> Result<()> {
    use colored::Colorize;

    let state_file = args
        .state_file
        .clone()
        .unwrap_or_else(|| PathBuf::from(".descartes/loop-state.json"));

    println!(
        "{} {:?}...",
        "Resuming loop from".cyan(),
        state_file
    );

    let mut loop_exec = IterativeLoop::resume(state_file).await?;

    println!("  Current iteration: {}", loop_exec.current_iteration());
    println!();

    // Set up Ctrl+C handler
    let cancel_handle = loop_exec.cancellation_handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n{}", "Received Ctrl+C, finishing current iteration...".yellow());
        cancel_handle.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let result = loop_exec.execute().await?;

    println!();
    println!("{}", "Loop completed!".green());
    println!("  Total iterations: {}", result.iterations_completed);
    println!("  Exit reason: {:?}", result.exit_reason);

    Ok(())
}

async fn handle_status(args: &LoopStatusArgs) -> Result<()> {
    use colored::Colorize;

    let state_file = args
        .state_file
        .clone()
        .unwrap_or_else(|| PathBuf::from(".descartes/loop-state.json"));

    let content = tokio::fs::read_to_string(&state_file).await?;
    let state: IterativeLoopState = serde_json::from_str(&content)?;

    println!("{}", "Loop Status".cyan().bold());
    println!("{}", "===========".dimmed());
    println!("  State file: {:?}", state_file);
    println!("  Iteration: {}", state.iteration);
    println!("  Started: {}", state.started_at);
    println!(
        "  Completed: {}",
        if state.completed {
            "Yes".green()
        } else {
            "No".yellow()
        }
    );
    if let Some(ref reason) = state.exit_reason {
        println!("  Exit reason: {:?}", reason);
    }
    if let Some(ref last) = state.last_iteration_at {
        println!("  Last iteration: {}", last);
    }
    println!();
    println!("{}", "Config:".dimmed());
    println!("  Command: {}", state.config.command);
    println!("  Max iterations: {:?}", state.config.max_iterations);
    println!("  Completion promise: {:?}", state.config.completion_promise);

    Ok(())
}

async fn handle_cancel(args: &LoopCancelArgs) -> Result<()> {
    use colored::Colorize;

    let state_file = args
        .state_file
        .clone()
        .unwrap_or_else(|| PathBuf::from(".descartes/loop-state.json"));

    let content = tokio::fs::read_to_string(&state_file).await?;
    let mut state: IterativeLoopState = serde_json::from_str(&content)?;

    state.completed = true;
    state.exit_reason = Some(IterativeExitReason::UserCancelled);

    let content = serde_json::to_string_pretty(&state)?;
    tokio::fs::write(&state_file, content).await?;

    println!("{}", "Loop cancelled. State updated.".green());

    Ok(())
}
