//! Descartes CLI
//!
//! Visible subagent orchestration with Swarm loops.

use std::str::FromStr;

use clap::{ArgAction, Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use descartes::{Config, Result, views::{agents, scud, settings, skills, transcripts}};

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
    Waves {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Output as streaming JSON events (one per wave)
        #[arg(long)]
        json_events: bool,
    },

    /// Initialize .descartes directory
    Init {
        /// Also run interactive wizard for configuration
        #[arg(long)]
        wizard: bool,

        /// Also initialize SCUD task management
        #[arg(long)]
        with_scud: bool,

        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },

    /// Run interactive configuration wizard (launches Claude Code)
    Wizard {
        /// Skip SCUD initialization
        #[arg(long)]
        no_scud: bool,

        /// Reconfigure existing setup (don't skip if already initialized)
        #[arg(long)]
        reconfigure: bool,
    },

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

    /// Manage agent definitions
    Agents {
        #[command(subcommand)]
        action: AgentCommands,
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

    /// Spawn SCUD agents for the next available tasks (thin wrapper around scud spawn)
    ScudSpawn {
        /// SCUD tag (uses active tag if not provided)
        #[arg(long)]
        scud_tag: Option<String>,

        /// Path to implementation plan document for spec context
        #[arg(long)]
        plan: Option<std::path::PathBuf>,

        /// Additional spec files to include (can be repeated)
        #[arg(long = "spec-file", action = ArgAction::Append)]
        spec_files: Vec<std::path::PathBuf>,

        /// Max tokens for spec section (default: 5000)
        #[arg(long, default_value = "5000")]
        max_spec_tokens: usize,

        /// Maximum agents to spawn (default: 5)
        #[arg(short = 'n', long, default_value = "5")]
        limit: usize,

        /// Harness to use: claude-code, opencode
        #[arg(long, env = "DESCARTES_HARNESS", default_value = "claude-code")]
        harness: String,

        /// Start TUI monitor after spawn (default: true)
        #[arg(long, default_value = "true")]
        monitor: bool,

        /// Disable TUI monitor
        #[arg(long)]
        no_monitor: bool,

        /// Working directory (default: current)
        #[arg(long)]
        working_dir: Option<std::path::PathBuf>,

        /// Show spawn plan without executing
        #[arg(long)]
        dry_run: bool,
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
enum AgentCommands {
    /// List available agent definitions
    List,

    /// Show details of an agent definition
    Show {
        /// Agent name
        name: String,
    },

    /// Initialize default agents directory
    Init {
        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
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
            transcripts::show_transcripts(&config, today, session)?;
        }

        Commands::Show { session_id } => {
            transcripts::show_transcript(&config, &session_id)?;
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

        Commands::Waves { json, json_events } => {
            let format = if json_events {
                scud::WavesOutputFormat::JsonEvents
            } else if json {
                scud::WavesOutputFormat::Json
            } else {
                scud::WavesOutputFormat::Human
            };
            scud::show_waves_with_format(&config, format)?;
        }

        Commands::Init {
            wizard,
            with_scud,
            force,
        } => {
            // Basic init
            descartes::config::init_with_options(force)?;
            info!("Initialized .descartes directory");

            // Initialize SCUD if requested
            if with_scud {
                let status = std::process::Command::new("scud").arg("init").status();
                match status {
                    Ok(s) if s.success() => info!("Initialized SCUD"),
                    Ok(_) => eprintln!("Warning: scud init returned non-zero"),
                    Err(_) => eprintln!("Warning: scud not found, skipping SCUD init"),
                }
            }

            // Launch wizard if requested
            if wizard {
                if let Ok(claude_path) = which::which("claude") {
                    let prompt = include_str!("../skills/wizard.md");
                    let _ = std::process::Command::new(claude_path)
                        .args(["-p", prompt, "--dangerously-skip-permissions"])
                        .status();
                } else {
                    eprintln!("Warning: claude CLI not found, skipping wizard");
                }
            }
        }

        Commands::Wizard { no_scud, reconfigure } => {
            // Check if claude binary exists
            let claude_path = which::which("claude").map_err(|_| {
                descartes::Error::Command(
                    "claude CLI not found. Install from https://claude.ai/code".to_string(),
                )
            })?;

            // Build a simple prompt to invoke the wizard
            let mut prompt =
                "Help me configure Descartes. Walk me through setting up the project.".to_string();

            if no_scud {
                prompt.push_str(" Skip SCUD initialization.");
            }
            if reconfigure {
                prompt.push_str(" Reconfigure even if already set up.");
            }

            // Launch Claude Code
            let status = std::process::Command::new(claude_path)
                .arg(&prompt)
                .current_dir(std::env::current_dir()?)
                .status()?;

            if !status.success() {
                return Err(descartes::Error::Command(
                    "Wizard exited with error".to_string(),
                ));
            }
        }

        Commands::Config => {
            settings::show_config(&config)?;
        }

        Commands::Harness => {
            settings::show_harness(&config)?;
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

        Commands::Agents { action } => {
            handle_agents_command(&config, action)?;
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
            verify: _,
            harness,
            model: _,
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

            // Task 9.2: Enrich spec config with SCUD/backpressure integration
            let spec_config = descartes::spec::enrich_spec_config(
                spec_config,
                &working_dir,
                Some(&final_tag),
            )?;

            // Write spec to SCUD guidance
            descartes::spec::write_spec_to_guidance(&spec_config, &working_dir)?;

            // Delegate to scud swarm
            run_scud_swarm(
                &final_tag,
                &harness,
                round_size,
                no_validate,
                dry_run,
                &working_dir,
            )?;
        }

        Commands::ScudSpawn {
            scud_tag,
            plan,
            spec_files,
            max_spec_tokens,
            limit,
            harness,
            monitor,
            no_monitor,
            working_dir,
            dry_run,
        } => {
            let working_dir =
                working_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            // Build SpecConfig
            let mut spec_config = descartes::SpecConfig::new();
            spec_config.max_spec_tokens = Some(max_spec_tokens);

            if let Some(plan_path) = plan {
                spec_config.plan_path = Some(plan_path);
            }

            for spec_file in spec_files {
                spec_config.additional_specs.push(spec_file);
            }

            // Task 9.2: Enrich spec config with SCUD/backpressure integration
            let spec_config = descartes::spec::enrich_spec_config(
                spec_config,
                &working_dir,
                scud_tag.as_deref(),
            )?;

            // Write spec to SCUD guidance
            descartes::spec::write_spec_to_guidance(&spec_config, &working_dir)?;

            // Delegate to scud spawn
            run_scud_spawn(
                scud_tag.as_deref(),
                &harness,
                limit,
                monitor && !no_monitor,
                dry_run,
                &working_dir,
            )?;
        }
    }

    Ok(())
}

/// Handle agents subcommands
fn handle_agents_command(config: &Config, action: AgentCommands) -> Result<()> {
    match action {
        AgentCommands::List => {
            agents::show_agents_list(config)?;
        }

        AgentCommands::Show { name } => {
            agents::show_agent_detail(config, &name)?;
        }

        AgentCommands::Init { force } => {
            let agents_dir = config.agents.directory.clone();

            // Check if any agent already exists
            if agents_dir.exists() && agents_dir.read_dir()?.next().is_some() && !force {
                eprintln!(
                    "Agents directory already exists at {:?}. Use --force to overwrite.",
                    agents_dir
                );
                return Ok(());
            }

            // Default agents to create - one or more per category
            let default_agents = [
                // Searcher category
                ("codebase-locator", include_str!("../agents/codebase-locator.md")),
                ("thoughts-locator", include_str!("../agents/thoughts-locator.md")),
                ("web-search-researcher", include_str!("../agents/web-search-researcher.md")),
                // Analyzer category
                ("codebase-analyzer", include_str!("../agents/codebase-analyzer.md")),
                ("codebase-pattern-finder", include_str!("../agents/codebase-pattern-finder.md")),
                ("thoughts-analyzer", include_str!("../agents/thoughts-analyzer.md")),
                // Builder category
                ("builder", include_str!("../agents/builder.md")),
                // Fast-builder category
                ("fast-builder", include_str!("../agents/fast-builder.md")),
                // Planner category
                ("planner", include_str!("../agents/planner.md")),
                // Validator category
                ("validator", include_str!("../agents/validator.md")),
                // Builder-reviewer category
                ("builder-reviewer", include_str!("../agents/builder-reviewer.md")),
                // Orchestrator category
                ("orchestrator", include_str!("../agents/orchestrator.md")),
            ];

            // Create each agent
            for (name, content) in default_agents {
                let agent_dir = agents_dir.join(name);
                std::fs::create_dir_all(&agent_dir)?;
                std::fs::write(agent_dir.join("AGENT.md"), content)?;
                info!("Created agent: {}", name);
            }

            println!("Created {} default agents in {:?}", default_agents.len(), agents_dir);
            println!("\nAgents created:");
            for (name, _) in default_agents {
                println!("  - {}", name);
            }
            println!("\nYou can now:");
            println!("  - List agents: descartes agents list");
            println!("  - Show an agent: descartes agents show <name>");
            println!("  - Create more agents by adding AGENT.md files to subdirectories");
        }
    }

    Ok(())
}

/// Handle skills subcommands
fn handle_skills_command(action: SkillCommands) -> Result<()> {
    match action {
        SkillCommands::List => {
            skills::show_skills_list()?;
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
            skills::show_skill_detail(&name)?;
        }
    }

    Ok(())
}

/// Map Descartes harness name to SCUD harness name
fn map_harness_name(descartes_harness: &str) -> &str {
    match descartes_harness {
        "claude-code" => "claude",
        "opencode" => "opencode",
        "codex" => "opencode", // Codex uses OpenCode harness in SCUD
        other => other,
    }
}

/// Run SCUD swarm as a subprocess
///
/// Delegates execution to `scud swarm` which provides:
/// - Visible terminal windows
/// - Wave-based orchestration
/// - TUI monitoring
/// - Backpressure validation
fn run_scud_swarm(
    tag: &str,
    harness: &str,
    round_size: usize,
    no_validate: bool,
    dry_run: bool,
    working_dir: &std::path::Path,
) -> Result<()> {
    // Check if scud is in PATH
    if which::which("scud").is_err() {
        return Err(descartes::Error::Command(
            "scud CLI not found in PATH. Install SCUD for swarm execution.".to_string(),
        ));
    }

    let scud_harness = map_harness_name(harness);

    let mut args = vec![
        "swarm".to_string(),
        "--tag".to_string(),
        tag.to_string(),
        "--harness".to_string(),
        scud_harness.to_string(),
        "--round-size".to_string(),
        round_size.to_string(),
    ];

    if no_validate {
        args.push("--no-validate".to_string());
    }

    if dry_run {
        args.push("--dry-run".to_string());
    }

    info!("Delegating to: scud {}", args.join(" "));

    let status = std::process::Command::new("scud")
        .args(&args)
        .current_dir(working_dir)
        .status()?;

    if !status.success() {
        return Err(descartes::Error::Command(format!(
            "scud swarm exited with status: {}",
            status
        )));
    }

    Ok(())
}

/// Run SCUD spawn as a subprocess
///
/// Delegates execution to `scud spawn` which provides:
/// - Visible terminal windows for each agent
/// - Task claiming
/// - Optional TUI monitoring
fn run_scud_spawn(
    tag: Option<&str>,
    harness: &str,
    limit: usize,
    monitor: bool,
    dry_run: bool,
    working_dir: &std::path::Path,
) -> Result<()> {
    // Check if scud is in PATH
    if which::which("scud").is_err() {
        return Err(descartes::Error::Command(
            "scud CLI not found in PATH. Install SCUD to use scud-spawn.".to_string(),
        ));
    }

    let scud_harness = map_harness_name(harness);

    let mut args = vec![
        "spawn".to_string(),
        "--harness".to_string(),
        scud_harness.to_string(),
        "--limit".to_string(),
        limit.to_string(),
        "--claim".to_string(), // Always claim tasks
    ];

    if let Some(t) = tag {
        args.push("--tag".to_string());
        args.push(t.to_string());
    }

    if monitor {
        args.push("--monitor".to_string());
    }

    if dry_run {
        args.push("--dry-run".to_string());
    }

    info!("Delegating to: scud {}", args.join(" "));

    let status = std::process::Command::new("scud")
        .args(&args)
        .current_dir(working_dir)
        .status()?;

    if !status.success() {
        return Err(descartes::Error::Command(format!(
            "scud spawn exited with status: {}",
            status
        )));
    }

    Ok(())
}
