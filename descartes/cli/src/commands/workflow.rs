//! Workflow commands for Descartes CLI
//!
//! Implements `/cl:*` style workflow commands for common development tasks.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use descartes_core::{
    execute_workflow as run_workflow, get_workflow, list_workflows, prepare_workflow,
    DescaratesConfig, ProviderFactory, WorkflowContext, WorkflowExecutorConfig,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

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

        /// Use a headless CLI adapter (claude-code, opencode)
        #[arg(long)]
        adapter: Option<String>,
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

        /// Use a headless CLI adapter (claude-code, opencode)
        #[arg(long)]
        adapter: Option<String>,
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

        /// Use a headless CLI adapter (claude-code, opencode)
        #[arg(long)]
        adapter: Option<String>,
    },

    /// Show details about a specific workflow
    Info {
        /// Workflow name
        name: String,
    },
}

pub async fn execute(cmd: &WorkflowCommands, config: &DescaratesConfig) -> Result<()> {
    match cmd {
        WorkflowCommands::List => execute_list().await,
        WorkflowCommands::Research { topic, context, dir, adapter } => {
            execute_workflow_run("research_codebase", topic, context.as_deref(), dir.clone(), adapter.as_deref(), config).await
        }
        WorkflowCommands::Plan { topic, context, dir, adapter } => {
            execute_workflow_run("create_plan", topic, context.as_deref(), dir.clone(), adapter.as_deref(), config).await
        }
        WorkflowCommands::Implement { plan, dir, adapter } => {
            execute_implement(plan, dir.clone(), adapter.as_deref(), config).await
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

async fn execute_workflow_run(
    workflow_name: &str,
    topic: &str,
    context: Option<&str>,
    dir: Option<PathBuf>,
    adapter: Option<&str>,
    config: &DescaratesConfig,
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
    println!("{}", "Executing workflow...".green().bold());
    println!();

    // Create backend - use adapter if specified, otherwise use primary provider
    let (provider_name, model, mut backend) = if let Some(adapter_name) = adapter {
        println!("  Using adapter: {}", adapter_name.cyan());
        let backend = create_adapter_backend(adapter_name)?;
        (adapter_name.to_string(), "default".to_string(), backend)
    } else {
        let provider_name = &config.providers.primary;
        let backend = create_backend(config, provider_name)?;
        let model = get_model_for_provider(config, provider_name)?;
        (provider_name.clone(), model, backend)
    };
    backend.initialize().await?;

    info!("Using provider: {}, model: {}", provider_name, model);

    let exec_config = WorkflowExecutorConfig {
        provider: provider_name.clone(),
        model,
        max_parallel: 3,
        save_outputs: true,
    };

    // Execute workflow
    let results = run_workflow(
        prepared_steps,
        &wf_context,
        Arc::from(backend),
        &exec_config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Workflow execution failed: {}", e))?;

    // Display results
    println!("{}", "─".repeat(50).dimmed());
    println!();
    println!("{}", "Workflow Results:".green().bold());
    println!();

    let mut all_success = true;
    for result in &results {
        let status = if result.success {
            "✓".green()
        } else {
            all_success = false;
            "✗".red()
        };
        println!(
            "  {} {} ({}ms)",
            status,
            result.step_name,
            result.duration_ms
        );
        if let Some(path) = &result.saved_to {
            println!("    Output: {}", path.display().to_string().yellow());
        }
        if let Some(error) = &result.error {
            println!("    Error: {}", error.red());
        }
    }

    println!();
    if all_success {
        println!("{}", "Workflow completed successfully!".green().bold());
    } else {
        println!("{}", "Workflow completed with errors.".yellow().bold());
    }
    println!();

    Ok(())
}

async fn execute_implement(plan: &str, dir: Option<PathBuf>, _adapter: Option<&str>, _config: &DescaratesConfig) -> Result<()> {
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
            for entry in (std::fs::read_dir(&thoughts_dir)?).flatten() {
                if entry.path().extension().is_some_and(|e| e == "md") {
                    println!(
                        "    - {}",
                        entry.file_name().to_string_lossy().green()
                    );
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

/// Create a model backend for the given provider
fn create_backend(
    config: &DescaratesConfig,
    provider: &str,
) -> Result<Box<dyn descartes_core::ModelBackend + Send + Sync>> {
    let mut provider_config: HashMap<String, String> = HashMap::new();

    match provider {
        "anthropic" => {
            if let Some(api_key) = &config.providers.anthropic.api_key {
                if !api_key.is_empty() {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.anthropic.endpoint.clone(),
            );
        }
        "openai" => {
            if let Some(api_key) = &config.providers.openai.api_key {
                if !api_key.is_empty() {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.openai.endpoint.clone(),
            );
        }
        "ollama" => {
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.ollama.endpoint.clone(),
            );
        }
        "grok" => {
            if let Some(api_key) = &config.providers.grok.api_key {
                if !api_key.is_empty() {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.grok.endpoint.clone(),
            );
        }
        _ => {
            anyhow::bail!("Unknown provider: {}", provider);
        }
    }

    let backend = ProviderFactory::create(provider, provider_config)?;
    Ok(backend)
}

/// Get the model for a provider from config
fn get_model_for_provider(config: &DescaratesConfig, provider: &str) -> Result<String> {
    match provider {
        "anthropic" => Ok(config.providers.anthropic.model.clone()),
        "openai" => Ok(config.providers.openai.model.clone()),
        "ollama" => Ok(config.providers.ollama.model.clone()),
        "grok" => Ok(config.providers.grok.model.clone()),
        _ => anyhow::bail!("Unknown provider: {}", provider),
    }
}

/// Create a headless CLI adapter backend
fn create_adapter_backend(
    adapter_name: &str,
) -> Result<Box<dyn descartes_core::ModelBackend + Send + Sync>> {
    let mut provider_config: HashMap<String, String> = HashMap::new();

    match adapter_name {
        "claude-code" | "claude" => {
            provider_config.insert("command".to_string(), "claude".to_string());
        }
        "opencode" => {
            provider_config.insert("command".to_string(), "opencode".to_string());
        }
        _ => {
            // Treat as generic command
            provider_config.insert("command".to_string(), adapter_name.to_string());
        }
    }

    // Use headless-cli provider for CLI adapters
    let backend = ProviderFactory::create("headless-cli", provider_config)?;
    Ok(backend)
}
