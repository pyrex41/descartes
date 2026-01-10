//! Descartes CLI
//!
//! Visible subagent orchestration with Ralph-Wiggum loops.

use std::str::FromStr;

use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
    }

    Ok(())
}
