//! Descartes CLI
//!
//! Visible subagent orchestration with Ralph-Wiggum loops.

use std::str::FromStr;

use clap::{ArgAction, Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use descartes::workflow::{
    self, default_workflow, RunOptions, StateManager, WorkflowConfig, WorkflowRunner,
};
use descartes::{Config, LoopConfig, LoopMode, Result};

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

    /// Start interactive session (persistent CLI)
    #[command(alias = "i")]
    Interactive {
        /// Start with a workflow
        #[arg(long)]
        workflow: Option<String>,

        /// Start at a specific stage
        #[arg(long)]
        stage: Option<String>,
    },

    /// Initialize default skills
    Skills {
        #[command(subcommand)]
        action: SkillCommands,
    },

    /// Run Ralph Wiggum loop for SCUD tasks
    Ralph {
        /// SCUD tag to execute (required unless --prd creates it)
        #[arg(long)]
        scud_tag: Option<String>,

        // === PRD Initialization Options ===
        /// Initialize tasks from PRD document (runs scud parse, expand, check-deps)
        #[arg(long)]
        prd: Option<std::path::PathBuf>,

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
        plan: Option<std::path::PathBuf>,

        /// Additional spec files to include (can be repeated)
        #[arg(long = "spec-file", action = ArgAction::Append)]
        spec_files: Vec<std::path::PathBuf>,

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
        working_dir: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand)]
enum SkillCommands {
    /// List available skills
    List,

    /// Initialize default skill prompts
    Init {
        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
    },

    /// Show a skill's prompt
    Show {
        /// Skill name
        name: String,
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
            let result = descartes::agent::spawn_subagent(&*harness, cat, prompt, None).await?;
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

        Commands::Next => match descartes::scud::next(&config)? {
            Some(task) => println!("Next task: {} - {}", task.id, task.title),
            None => println!("No tasks ready"),
        },

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

        Commands::Config => match toml::to_string_pretty(&config) {
            Ok(s) => println!("{}", s),
            Err(e) => eprintln!("Failed to serialize config: {}", e),
        },

        Commands::Harness => {
            println!("Active harness: {}", config.harness.kind);
        }

        Commands::Workflow { action } => {
            handle_workflow_command(action, &config).await?;
        }

        Commands::Handoff {
            target,
            extra,
            output,
        } => {
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
            let handoff =
                workflow::quick_handoff(&workflow_config, from_stage, extra.as_deref()).await?;

            // Output
            if let Some(path) = output {
                std::fs::write(&path, &handoff)?;
                info!("Handoff written to {:?}", path);
            } else {
                println!("{}", handoff);
            }
        }

        Commands::Interactive { workflow, stage } => {
            info!("Starting interactive session");
            let _ = stage; // Will be used when we implement stage-specific starting

            // Install panic handler
            descartes::interactive::signals::install_panic_handler();

            // Load workflow config if specified
            let workflow_config = if let Some(ref name) = workflow {
                Some(load_workflow_config(Some(name))?)
            } else {
                None
            };

            // Create harness (convert Box to Arc for shared ownership in session)
            let harness = descartes::harness::create_harness(&config)?;
            let harness: std::sync::Arc<dyn descartes::Harness> = harness.into();

            // Create session
            let mut session =
                descartes::interactive::Session::new(config.clone(), harness, workflow_config);

            // Install signal handler
            let signal_handler = descartes::interactive::SignalHandler::new(
                session.interrupt_flag(),
                session.shutdown_flag(),
            );
            signal_handler.install()?;

            // Run the interactive session
            session.run().await?;
        }

        Commands::Skills { action } => {
            handle_skills_command(action)?;
        }

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
            let working_dir =
                working_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            // Task 4.1: Handle PRD initialization
            let final_tag = if let Some(prd_path) = &prd {
                // Determine tag name from --tag or PRD filename
                let tag_name = tag.unwrap_or_else(|| {
                    prd_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "ralph".to_string())
                });

                info!("Initializing tasks from PRD: {:?}", prd_path);

                // Run scud parse
                let parse_status = std::process::Command::new("scud")
                    .args([
                        "parse",
                        &prd_path.to_string_lossy(),
                        "--tag",
                        &tag_name,
                        "-n",
                        &num_tasks.to_string(),
                    ])
                    .current_dir(&working_dir)
                    .status()?;

                if !parse_status.success() {
                    return Err(descartes::Error::Command("scud parse failed".to_string()));
                }

                // Run scud expand (unless --no-expand)
                if !no_expand {
                    info!("Expanding tasks...");
                    let expand_status = std::process::Command::new("scud")
                        .args(["expand", "--all", "--tag", &tag_name])
                        .current_dir(&working_dir)
                        .status()?;

                    if !expand_status.success() {
                        return Err(descartes::Error::Command("scud expand failed".to_string()));
                    }
                }

                // Run scud check-deps --fix (unless --no-check-deps)
                if !no_check_deps {
                    info!("Checking dependencies...");
                    let check_status = std::process::Command::new("scud")
                        .args(["check-deps", "--fix", "--tag", &tag_name])
                        .current_dir(&working_dir)
                        .status()?;

                    if !check_status.success() {
                        return Err(descartes::Error::Command(
                            "scud check-deps failed".to_string(),
                        ));
                    }
                }

                tag_name
            } else if let Some(tag) = scud_tag {
                tag
            } else {
                return Err(descartes::Error::Config(
                    "Either --scud-tag or --prd must be provided".to_string(),
                ));
            };

            // Task 4.2: Build SpecConfig from CLI options
            let mut spec_config = descartes::SpecConfig::new();
            spec_config.max_spec_tokens = Some(max_spec_tokens);

            // Use PRD as plan if no explicit plan provided
            if let Some(plan_path) = plan.or_else(|| prd.clone()) {
                spec_config.plan_path = Some(plan_path);
            }

            // Add additional spec files
            for spec_file in spec_files {
                spec_config.additional_specs.push(spec_file);
            }

            // Create RalphExecutor and dispatch
            let executor = descartes::RalphExecutor::new(
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
                executor.dry_run().await?;
            } else {
                executor.run(&config).await?;
            }
        }
    }

    Ok(())
}

/// Handle skills subcommands
fn handle_skills_command(action: SkillCommands) -> Result<()> {
    match action {
        SkillCommands::List => {
            let registry = descartes::interactive::SkillRegistry::new();
            println!("Available skills:\n");
            for (name, skill) in registry.list() {
                let aliases = if skill.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", skill.aliases.join(", "))
                };
                println!("  /{}{}", name, aliases);
                println!("    {}", skill.description);
                if let Some(ref cat) = skill.category {
                    println!("    Category: {}", cat);
                }
                println!();
            }
        }

        SkillCommands::Init { force } => {
            let dir = std::path::PathBuf::from(".descartes/skills");

            if dir.exists() && !force {
                eprintln!("Skills directory already exists. Use --force to overwrite.");
                return Ok(());
            }

            descartes::interactive::skills::create_default_skills(&dir)?;
            info!("Created default skill prompts in {:?}", dir);
        }

        SkillCommands::Show { name } => {
            let registry = descartes::interactive::SkillRegistry::new();
            if let Some(skill) = registry.get(&name) {
                println!("Skill: {}", skill.name);
                println!("Description: {}", skill.description);
                println!("Prompt file: {}", skill.prompt_file.display());
                if let Some(ref cat) = skill.category {
                    println!("Category: {}", cat);
                }
                if !skill.aliases.is_empty() {
                    println!("Aliases: {}", skill.aliases.join(", "));
                }
                if !skill.variables.is_empty() {
                    println!("\nVariables:");
                    for var in &skill.variables {
                        let required = if var.required { " (required)" } else { "" };
                        let default = var
                            .default
                            .as_ref()
                            .map(|d| format!(" [default: {}]", d))
                            .unwrap_or_default();
                        println!("  {}{}{}", var.name, required, default);
                        if let Some(ref desc) = var.description {
                            println!("    {}", desc);
                        }
                    }
                }

                // Try to show the prompt content
                if skill.prompt_file.exists() {
                    println!("\n─── Prompt Content ───");
                    if let Ok(content) = std::fs::read_to_string(&skill.prompt_file) {
                        println!("{}", content);
                    }
                } else {
                    println!("\nNote: Prompt file does not exist yet. Run 'descartes skills init' to create it.");
                }
            } else {
                eprintln!("Unknown skill: {}", name);
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
