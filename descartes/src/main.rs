//! Descartes CLI
//!
//! Visible subagent orchestration with Swarm loops.

use std::str::FromStr;

use clap::{ArgAction, Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use descartes::{Config, Result};

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

    /// Start interactive session (persistent CLI)
    #[command(alias = "i")]
    Interactive,

    /// Initialize default skills
    Skills {
        #[command(subcommand)]
        action: SkillCommands,
    },

    /// Run Swarm loop for SCUD tasks
    Swarm {
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

        /// Harness to use: claude-code, opencode, codex
        #[arg(long, env = "DESCARTES_HARNESS", default_value = "claude-code")]
        harness: String,

        /// Model to use (default: from config or DESCARTES_MODEL env var)
        #[arg(long, env = "DESCARTES_MODEL")]
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

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (before anything else)
    // Tries: .env, .descartes/.env, ~/.descartes/.env
    if dotenvy::dotenv().is_err() {
        // Try .descartes/.env
        let _ = dotenvy::from_filename(".descartes/.env");
    }
    // Also try home directory
    if let Some(home) = dirs::home_dir() {
        let _ = dotenvy::from_path(home.join(".descartes/.env"));
    }

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
        Commands::Spawn { category, prompt } => {
            info!("Spawning {} subagent", category);
            let cat = descartes::agent::AgentCategory::from_str(&category)?;
            let harness = descartes::harness::create_harness(&config)?;
            let result = descartes::agent::spawn_subagent(&*harness, cat, prompt, None, None).await?;
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
            use descartes::scud::{Storage, Task};

            let storage = Storage::new(None);
            let phase = match storage.load_active_group() {
                Ok(p) => p,
                Err(_) => {
                    println!("No active phase found.");
                    return Ok(());
                }
            };

            let waves_ids = descartes::scud::waves(&config)?;
            if waves_ids.is_empty() {
                println!("No pending tasks.");
                return Ok(());
            }

            const ROUND_SIZE: usize = 5;
            let total_tasks: usize = waves_ids.iter().map(|w| w.len()).sum();
            let total_rounds: usize = waves_ids.iter().map(|w| if w.is_empty() { 0 } else { (w.len() + ROUND_SIZE - 1) / ROUND_SIZE }).sum();
            let total_waves = waves_ids.len();
            let speedup = if total_rounds == 0 { 1.0f64 } else { total_tasks as f64 / total_rounds as f64 };

            println!("Execution Waves (max {} parallel)", ROUND_SIZE);
            println!("{}", "═".repeat(50));

            for (wave_idx, wave_ids) in waves_ids.iter().enumerate() {
                let n_tasks = wave_ids.len();
                let n_rounds_wave = if n_tasks == 0 { 0 } else { (n_tasks + ROUND_SIZE - 1) / ROUND_SIZE };
                println!("Wave {}: {} tasks (batched into {} rounds)", wave_idx + 1, n_tasks, n_rounds_wave);

                let rounds: Vec<Vec<&Task>> = wave_ids.chunks(ROUND_SIZE).map(|chunk_ids| {
                    chunk_ids.iter().filter_map(|id| phase.get_task(id)).collect()
                }).collect();

                for (round_idx, round_tasks) in rounds.iter().enumerate() {
                    println!("  Round {}", round_idx + 1);
                    for task in round_tasks.iter() {
                        let deps_ids: Vec<_> = task.dependencies.iter()
                            .filter(|d| phase.get_task(d).is_some())
                            .cloned()
                            .collect();
                        let mut deps_sorted = deps_ids.clone();
                        deps_sorted.sort_unstable();
                        let deps_str = deps_sorted.join(", ");
                        let deps_display = if !deps_ids.is_empty() {
                            format!(" [{}] <- {}", deps_ids.len(), deps_str)
                        } else {
                            String::new()
                        };
                        let title_trunc: String = task.title.chars().take(60).collect();
                        println!("    ○ {} {}{}", task.id, title_trunc, deps_display);
                    }
                }
            }

            println!("\nSummary");
            println!("{}", "─".repeat(30));
            println!("  Total tasks:   {}", total_tasks);
            println!("  Total waves:   {}", total_waves);
            println!("  Total rounds:  {}", total_rounds);
            println!("  Speedup:       {:.1}x", speedup);
            println!("  (from {} sequential to {} parallel rounds)", total_tasks, total_rounds);
        }

        Commands::Init => {
            descartes::config::init()?;
            info!("Initialized .descartes directory");
        }

        Commands::Config => {
            let content = toml::to_string_pretty(&config)
                .map_err(|e| descartes::Error::Config(e.to_string()))?;
            println!("{}", content);
        }

        Commands::Harness => {
            println!("Active harness: {}", config.harness.kind);
        }

        Commands::Interactive => {
            info!("Starting interactive session");

            // Install panic handler
            descartes::interactive::signals::install_panic_handler();

            // Create harness (convert Box to Arc for shared ownership in session)
            let harness = descartes::harness::create_harness(&config)?;
            let harness: std::sync::Arc<dyn descartes::Harness> = harness.into();

            // Create session
            let mut session =
                descartes::interactive::Session::new(config.clone(), harness);

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

        Commands::Swarm {
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
                        .unwrap_or_else(|| "swarm".to_string())
                });

                info!("Initializing tasks from PRD: {:?}", prd_path);

                // Get SCUD env vars from config
                let scud_env_vars = config.scud.env_vars();

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
                    .envs(scud_env_vars.clone())
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
                        .envs(scud_env_vars.clone())
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
                        .envs(scud_env_vars)
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

            // Create SwarmExecutor and dispatch
            let executor = descartes::SwarmExecutor::new(
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
