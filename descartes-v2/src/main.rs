//! Descartes CLI
//!
//! Visible subagent orchestration with Ralph-Wiggum loops.

use std::str::FromStr;

use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use descartes::{Config, LoopConfig, LoopMode, Result};
use descartes::workflow::{
    self, default_workflow, GateType, RunOptions, StateManager, WorkflowConfig, WorkflowRunner,
};

#[derive(Parser)]
#[command(name = "descartes")]
#[command(author, version, about = "Visible subagent orchestration")]
#[command(propagate_version = true)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<std::path::PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the Ralph loop (continuous iteration)
    Loop {
        /// Run in planning mode (analyze gaps, update task graph)
        #[arg(long)]
        plan: bool,

        /// Maximum iterations (0 = infinite)
        #[arg(long, short, default_value = "0")]
        max: usize,
    },

    /// Run a single build iteration
    Run,

    /// Run a single planning iteration
    Plan,

    /// Spawn a subagent manually
    Spawn {
        /// Agent category (searcher, analyzer, builder, validator)
        category: String,

        /// Prompt for the subagent
        prompt: String,
    },

    /// List transcripts
    Transcripts {
        /// Show only today's transcripts
        #[arg(long)]
        today: bool,

        /// Show only transcripts from session
        #[arg(long)]
        session: Option<String>,
    },

    /// Show a transcript
    Show {
        /// Session ID
        session_id: String,
    },

    /// Replay a transcript with timing
    Replay {
        /// Session ID
        session_id: String,

        /// Playback speed multiplier
        #[arg(long, default_value = "1.0")]
        speed: f32,
    },

    /// Get next ready task from SCUD
    Next,

    /// Mark a task complete
    Complete {
        /// Task ID
        task_id: String,
    },

    /// Show task waves (parallel execution potential)
    Waves,

    /// Initialize .descartes directory
    Init,

    /// Show current configuration
    Config,

    /// Show active harness
    Harness,

    /// Workflow orchestration commands
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },

    /// Quick handoff generation
    Handoff {
        /// Target stage (plan, implement, validate)
        target: String,

        /// Extra context to include
        #[arg(long, short)]
        extra: Option<String>,

        /// Output to file instead of stdout
        #[arg(long, short)]
        output: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Run a workflow
    Run {
        /// Workflow name (default: from .descartes/workflow.toml)
        #[arg(long)]
        workflow: Option<String>,

        /// Step-by-step mode (all gates manual)
        #[arg(long)]
        step_by_step: bool,

        /// One-shot mode (all gates auto)
        #[arg(long)]
        one_shot: bool,

        /// Start from this stage
        #[arg(long)]
        from: Option<String>,

        /// Stop after this stage
        #[arg(long)]
        to: Option<String>,

        /// Extra context to inject
        #[arg(long, short)]
        extra: Option<String>,

        /// Resume a specific run by ID
        #[arg(long)]
        resume: Option<String>,

        /// Dry run (don't execute agents)
        #[arg(long)]
        dry_run: bool,
    },

    /// Show workflow status
    Status {
        /// Workflow name
        #[arg(long)]
        workflow: Option<String>,

        /// Specific run ID
        #[arg(long)]
        run: Option<String>,
    },

    /// List workflow runs
    List {
        /// Workflow name
        #[arg(long)]
        workflow: Option<String>,

        /// Show only last N runs
        #[arg(long, default_value = "10")]
        last: usize,
    },

    /// Initialize default workflow configuration
    Init {
        /// Force overwrite existing config
        #[arg(long)]
        force: bool,
    },

    /// Show workflow configuration
    Config {
        /// Workflow name
        #[arg(long)]
        workflow: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    // Load config
    let config = Config::load(cli.config.as_deref())?;

    match cli.command {
        Commands::Loop { plan, max } => {
            let mode = if plan {
                LoopMode::Plan
            } else {
                LoopMode::Build
            };
            let max_iterations = if max == 0 { None } else { Some(max) };

            let loop_config = LoopConfig {
                mode,
                max_iterations,
                ..Default::default()
            };

            descartes::ralph_loop::run(loop_config, &config).await?;
        }

        Commands::Run => {
            info!("Running single build iteration");
            let loop_config = LoopConfig {
                mode: LoopMode::Build,
                max_iterations: Some(1),
                ..Default::default()
            };
            descartes::ralph_loop::run(loop_config, &config).await?;
        }

        Commands::Plan => {
            info!("Running single planning iteration");
            let loop_config = LoopConfig {
                mode: LoopMode::Plan,
                max_iterations: Some(1),
                ..Default::default()
            };
            descartes::ralph_loop::run(loop_config, &config).await?;
        }

        Commands::Spawn { category, prompt } => {
            info!("Spawning {} subagent", category);
            let cat = descartes::agent::AgentCategory::from_str(&category)?;
            let harness = descartes::harness::create_harness(&config)?;
            let result =
                descartes::agent::spawn_subagent(&*harness, cat, prompt, None).await?;
            println!("{}", result.summary());
        }

        Commands::Transcripts { today, session } => {
            let transcripts = descartes::transcript::list_transcripts(&config, today, session)?;
            for t in transcripts {
                println!("{}", t);
            }
        }

        Commands::Show { session_id } => {
            let transcript = descartes::transcript::load(&config, &session_id)?;
            println!("{}", transcript.to_scg());
        }

        Commands::Replay { session_id, speed } => {
            let transcript = descartes::transcript::load(&config, &session_id)?;
            descartes::transcript::replay(&transcript, speed).await?;
        }

        Commands::Next => {
            match descartes::scud::next(&config)? {
                Some(task) => println!("Next task: {} - {}", task.id, task.title),
                None => println!("No tasks ready"),
            }
        }

        Commands::Complete { task_id } => {
            descartes::scud::complete(&config, &task_id)?;
            info!("Marked task {} complete", task_id);
        }

        Commands::Waves => {
            let waves = descartes::scud::waves(&config)?;
            for (i, wave) in waves.iter().enumerate() {
                println!("Wave {}: {:?}", i + 1, wave);
            }
        }

        Commands::Init => {
            descartes::config::init()?;
            info!("Initialized .descartes directory");
        }

        Commands::Config => {
            match toml::to_string_pretty(&config) {
                Ok(s) => println!("{}", s),
                Err(e) => eprintln!("Failed to serialize config: {}", e),
            }
        }

        Commands::Harness => {
            println!("Active harness: {}", config.harness.kind);
        }

        Commands::Workflow { action } => {
            handle_workflow_command(action, &config).await?;
        }

        Commands::Handoff { target, extra, output } => {
            // Determine current stage from context (simplified - uses previous stage)
            let from_stage = match target.as_str() {
                "plan" => "research",
                "implement" => "plan",
                "validate" => "implement",
                _ => {
                    eprintln!("Unknown target stage: {}", target);
                    return Ok(());
                }
            };

            // Load or create workflow config
            let workflow_config = load_workflow_config(None)?;

            // Generate handoff
            let handoff = workflow::quick_handoff(
                &workflow_config,
                from_stage,
                extra.as_deref(),
            ).await?;

            // Output
            if let Some(path) = output {
                std::fs::write(&path, &handoff)?;
                info!("Handoff written to {:?}", path);
            } else {
                println!("{}", handoff);
            }
        }
    }

    Ok(())
}

/// Handle workflow subcommands
async fn handle_workflow_command(action: WorkflowCommands, config: &Config) -> Result<()> {
    match action {
        WorkflowCommands::Run {
            workflow,
            step_by_step,
            one_shot,
            from,
            to,
            extra,
            resume,
            dry_run,
        } => {
            let workflow_config = load_workflow_config(workflow.as_deref())?;
            let harness = descartes::harness::create_harness(config)?;

            let options = RunOptions {
                step_by_step,
                one_shot,
                from_stage: from,
                to_stage: to,
                extra_context: extra,
                resume_id: resume,
                dry_run,
                ..Default::default()
            };

            let runner = WorkflowRunner::new(workflow_config, config.clone(), harness);
            let state = runner.run(options).await?;

            println!("\n{}", state.summary());
        }

        WorkflowCommands::Status { workflow, run } => {
            let state_manager = StateManager::default();
            let workflow_name = workflow.unwrap_or_else(|| "default".to_string());

            let state = if let Some(run_id) = run {
                state_manager.load(&workflow_name, &run_id)?
            } else {
                state_manager
                    .find_latest(&workflow_name)?
                    .ok_or_else(|| descartes::Error::Config("No workflow runs found".to_string()))?
            };

            println!("{}", state.summary());
        }

        WorkflowCommands::List { workflow, last } => {
            let state_manager = StateManager::default();
            let states = state_manager.list(workflow.as_deref())?;

            for state in states.into_iter().take(last) {
                println!(
                    "{} | {} | {:?} | {}",
                    state.id,
                    state.workflow,
                    state.status,
                    state.started_at.format("%Y-%m-%d %H:%M")
                );
            }
        }

        WorkflowCommands::Init { force } => {
            let path = std::path::PathBuf::from(".descartes/workflow.toml");

            if path.exists() && !force {
                eprintln!("Workflow config already exists. Use --force to overwrite.");
                return Ok(());
            }

            // Create directory if needed
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Write default workflow config
            let default = default_workflow();
            let content = toml::to_string_pretty(&default)
                .map_err(|e| descartes::Error::Config(e.to_string()))?;
            std::fs::write(&path, content)?;

            info!("Created default workflow config at {:?}", path);
        }

        WorkflowCommands::Config { workflow } => {
            let workflow_config = load_workflow_config(workflow.as_deref())?;
            let content = toml::to_string_pretty(&workflow_config)
                .map_err(|e| descartes::Error::Config(e.to_string()))?;
            println!("{}", content);
        }
    }

    Ok(())
}

/// Load workflow configuration
fn load_workflow_config(name: Option<&str>) -> Result<WorkflowConfig> {
    let path = if let Some(name) = name {
        std::path::PathBuf::from(format!(".descartes/workflows/{}.toml", name))
    } else {
        std::path::PathBuf::from(".descartes/workflow.toml")
    };

    if path.exists() {
        WorkflowConfig::load(&path)
    } else {
        // Return default workflow
        Ok(default_workflow())
    }
}
