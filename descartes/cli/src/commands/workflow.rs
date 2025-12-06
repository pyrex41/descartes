//! Workflow commands for Descartes CLI
//!
//! Implements `/cl:*` style workflow commands for common development tasks.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use descartes_core::{
    get_workflow, list_workflows, prepare_workflow, WorkflowContext,
};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    /// List available workflow commands
    List,

    /// Research the codebase (find files, analyze code, find patterns)
    #[command(name = "research")]
    Research {
        /// Topic to research
        #[arg(short, long)]
        topic: String,

        /// Additional context
        #[arg(short, long)]
        context: Option<String>,

        /// Working directory (defaults to current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Create an implementation plan
    #[command(name = "plan")]
    Plan {
        /// Feature or task to plan
        #[arg(short, long)]
        topic: String,

        /// Additional context or requirements
        #[arg(short, long)]
        context: Option<String>,

        /// Working directory (defaults to current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Implement a plan from thoughts/plans/
    #[command(name = "implement")]
    Implement {
        /// Plan file to implement (from ~/.descartes/thoughts/plans/)
        #[arg(short, long)]
        plan: String,

        /// Working directory (defaults to current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Show details about a specific workflow
    Info {
        /// Workflow name
        name: String,
    },
}

pub async fn execute(cmd: &WorkflowCommands) -> Result<()> {
    match cmd {
        WorkflowCommands::List => execute_list().await,
        WorkflowCommands::Research { topic, context, dir } => {
            execute_workflow("research_codebase", topic, context.as_deref(), dir.clone()).await
        }
        WorkflowCommands::Plan { topic, context, dir } => {
            execute_workflow("create_plan", topic, context.as_deref(), dir.clone()).await
        }
        WorkflowCommands::Implement { plan, dir } => {
            execute_implement(plan, dir.clone()).await
        }
        WorkflowCommands::Info { name } => execute_info(name).await,
    }
}

async fn execute_list() -> Result<()> {
    println!();
    println!(
        "{}",
        "┌─ Workflow Commands ─────────────────────────────┐".cyan()
    );
    println!(
        "{}",
        "│  High-level development workflows               │".cyan()
    );
    println!(
        "{}",
        "└─────────────────────────────────────────────────┘".cyan()
    );
    println!();

    let workflows = list_workflows();

    for (name, description) in workflows {
        println!("  {} - {}", name.green().bold(), description.dimmed());
    }

    println!();
    println!(
        "{}",
        "Use 'descartes workflow info <name>' for details".dimmed()
    );
    println!();

    Ok(())
}

async fn execute_workflow(
    workflow_name: &str,
    topic: &str,
    context: Option<&str>,
    dir: Option<PathBuf>,
) -> Result<()> {
    println!();
    println!(
        "{}",
        format!("┌─ Workflow: {} ─", workflow_name).cyan()
    );
    println!();

    let workflow = get_workflow(workflow_name)
        .ok_or_else(|| anyhow::anyhow!("Workflow '{}' not found", workflow_name))?;

    println!("  {}", workflow.description.dimmed());
    println!("  Topic: {}", topic.yellow());
    if let Some(ctx) = context {
        println!("  Context: {}", ctx.dimmed());
    }
    println!();

    let working_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mut wf_context = WorkflowContext::new(working_dir.clone(), topic)
        .map_err(|e| anyhow::anyhow!("Failed to create workflow context: {}", e))?;

    if let Some(ctx) = context {
        wf_context = wf_context.with_context(ctx);
    }

    // Prepare the workflow steps
    let prepared_steps = prepare_workflow(&workflow, &wf_context)
        .map_err(|e| anyhow::anyhow!("Failed to prepare workflow: {}", e))?;

    println!("{}", "Steps to execute:".green().bold());
    println!();

    for (i, (step, task)) in prepared_steps.iter().enumerate() {
        let parallel_marker = if step.parallel { " (parallel)" } else { "" };
        println!(
            "  {}. {} [{}]{}",
            i + 1,
            step.name.bold(),
            step.agent.cyan(),
            parallel_marker.dimmed()
        );
        println!("     Task: {}", task.dimmed());
        if let Some(output) = &step.output {
            println!("     Output: {}", output.yellow());
        }
        println!();
    }

    println!("{}", "─".repeat(50).dimmed());
    println!();
    println!(
        "{}",
        "To execute this workflow, run each step using:".dimmed()
    );
    println!();

    for (step, task) in &prepared_steps {
        println!(
            "  {}",
            format!(
                "descartes spawn --task \"{}\" --tool-level {} --agent {}",
                truncate_task(task, 50),
                get_tool_level_for_agent(&step.agent),
                step.agent
            )
            .cyan()
        );
    }

    println!();
    println!(
        "{}",
        "Note: Full workflow automation coming in a future release!".yellow()
    );
    println!();

    Ok(())
}

async fn execute_implement(plan: &str, dir: Option<PathBuf>) -> Result<()> {
    println!();
    println!(
        "{}",
        "┌─ Workflow: implement_plan ─────────────────────┐".cyan()
    );
    println!();

    println!("  Plan: {}", plan.yellow());
    println!();

    let _working_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Check if plan file exists
    let thoughts_dir = dirs::home_dir()
        .map(|h| h.join(".descartes").join("thoughts").join("plans"))
        .unwrap_or_default();

    let plan_path = thoughts_dir.join(plan);
    if !plan_path.exists() {
        println!(
            "  {} Plan file not found: {}",
            "Error:".red().bold(),
            plan_path.display()
        );
        println!();
        println!("  Available plans in {}:", thoughts_dir.display());

        if thoughts_dir.exists() {
            for entry in std::fs::read_dir(&thoughts_dir)? {
                if let Ok(entry) = entry {
                    if entry.path().extension().map_or(false, |e| e == "md") {
                        println!(
                            "    - {}",
                            entry.file_name().to_string_lossy().green()
                        );
                    }
                }
            }
        } else {
            println!("    (no plans directory found)");
        }

        println!();
        return Ok(());
    }

    println!("  {}", "Plan found! To implement, run:".green());
    println!();
    println!(
        "  {}",
        format!(
            "descartes spawn --task \"Implement the plan at {}\" --tool-level orchestrator",
            plan_path.display()
        )
        .cyan()
    );
    println!();

    Ok(())
}

async fn execute_info(name: &str) -> Result<()> {
    let workflow = get_workflow(name)
        .ok_or_else(|| anyhow::anyhow!("Workflow '{}' not found", name))?;

    println!();
    println!(
        "{}",
        format!("┌─ Workflow: {} ─", workflow.name).cyan()
    );
    println!();

    println!("  {}", workflow.description);
    println!();
    println!("{}", "Steps:".green().bold());
    println!();

    for (i, step) in workflow.steps.iter().enumerate() {
        let parallel_marker = if step.parallel {
            " (can run in parallel)".dimmed().to_string()
        } else {
            String::new()
        };

        println!(
            "  {}. {}{}",
            i + 1,
            step.name.bold(),
            parallel_marker
        );
        println!("     Agent: {}", step.agent.cyan());
        println!("     Task: {}", step.task.dimmed());
        if let Some(output) = &step.output {
            println!("     Output: {}", output.yellow());
        }
        println!();
    }

    Ok(())
}

fn truncate_task(task: &str, max_len: usize) -> String {
    if task.len() <= max_len {
        task.to_string()
    } else {
        format!("{}...", &task[..max_len - 3])
    }
}

fn get_tool_level_for_agent(agent: &str) -> &'static str {
    match agent {
        "codebase-locator" | "codebase-analyzer" | "codebase-pattern-finder" => "readonly",
        "researcher" => "researcher",
        "planner" => "planner",
        _ => "minimal",
    }
}
