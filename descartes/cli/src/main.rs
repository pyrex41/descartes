/// Descartes CLI - Command-line interface for the orchestration system
use clap::{Parser, Subcommand};
use colored::Colorize;
use descartes_core::{ConfigManager, DescaratesConfig};
use std::path::{Path, PathBuf};

mod commands;
mod rpc;
mod state;

/// Load configuration from the given path or default location
fn load_config(config_path: Option<&Path>) -> anyhow::Result<DescaratesConfig> {
    let manager = ConfigManager::load(config_path)?;
    Ok(manager.config().clone())
}

use commands::{attach, init, kill, logs, pause, plugins, ps, resume, spawn, tasks};

#[derive(Parser)]
#[command(name = "descartes")]
#[command(about = "Composable AI Agent Orchestration System", long_about = None)]
#[command(version = "0.1.0")]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// Config file path (defaults to ~/.descartes/config.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Override log level
    #[arg(long, global = true)]
    log_level: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Descartes project
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,

        /// Base directory (defaults to ~/.descartes)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Spawn an AI agent
    Spawn {
        /// Task or prompt for the agent
        #[arg(short, long)]
        task: String,

        /// Model provider (anthropic, openai, ollama, claude-code-cli)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,

        /// System prompt/context
        #[arg(short, long)]
        system: Option<String>,

        /// Stream output in real-time
        #[arg(long, default_value = "true")]
        stream: bool,
    },

    /// List running agents
    Ps {
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show all agents (including completed)
        #[arg(short, long)]
        all: bool,
    },

    /// Kill an agent
    Kill {
        /// Agent ID
        id: String,

        /// Force kill (SIGKILL instead of SIGTERM)
        #[arg(short, long)]
        force: bool,
    },

    /// Pause a running agent
    Pause {
        /// Agent ID
        id: String,

        /// Force pause (SIGSTOP instead of cooperative)
        #[arg(short, long)]
        force: bool,
    },

    /// Resume a paused agent
    Resume {
        /// Agent ID
        id: String,
    },

    /// Get attach credentials for a paused agent
    Attach {
        /// Agent ID
        id: String,

        /// Client type (claude-code, opencode, or custom)
        #[arg(short, long, default_value = "claude-code")]
        client: String,

        /// Output JSON for scripting
        #[arg(long)]
        json: bool,

        /// Launch the TUI client after obtaining credentials
        #[arg(short, long)]
        launch: bool,
    },

    /// View agent logs
    Logs {
        /// Agent ID (optional - show all if not provided)
        id: Option<String>,

        /// Follow logs
        #[arg(short, long)]
        follow: bool,

        /// Filter by event type
        #[arg(long)]
        event_type: Option<String>,

        /// Limit number of entries
        #[arg(short, long, default_value = "100")]
        limit: usize,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Launch the GUI
    Gui,

    /// Manage plugins
    #[command(subcommand)]
    Plugins(plugins::PluginCommands),

    /// Manage tasks (uses SCG file storage)
    #[command(subcommand)]
    Tasks(tasks::TaskCommands),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing with custom level if provided
    let log_level = args.log_level.as_deref().unwrap_or("info");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    match args.command {
        Commands::Init { name, dir } => {
            init::execute(name.as_deref(), dir.as_deref()).await?;
        }

        Commands::Spawn {
            task,
            provider,
            model,
            system,
            stream,
        } => {
            let config = load_config(args.config.as_deref())?;

            spawn::execute(
                &config,
                &task,
                provider.as_deref(),
                model.as_deref(),
                system.as_deref(),
                stream,
            )
            .await?;
        }

        Commands::Ps { format, all } => {
            let config = load_config(args.config.as_deref())?;
            ps::execute(&config, &format, all).await?;
        }

        Commands::Kill { id, force } => {
            let config = load_config(args.config.as_deref())?;
            kill::execute(&config, &id, force).await?;
        }

        Commands::Pause { id, force } => {
            let config = load_config(args.config.as_deref())?;
            pause::execute(&config, &id, force).await?;
        }

        Commands::Resume { id } => {
            let config = load_config(args.config.as_deref())?;
            resume::execute(&config, &id).await?;
        }

        Commands::Attach { id, client, json, launch } => {
            let config = load_config(args.config.as_deref())?;
            attach::execute(&config, &id, &client, json, launch).await?;
        }

        Commands::Logs {
            id,
            follow,
            event_type,
            limit,
            format,
        } => {
            let config = load_config(args.config.as_deref())?;
            logs::execute(
                &config,
                id.as_deref(),
                follow,
                event_type.as_deref(),
                limit,
                &format,
            )
            .await?;
        }

        Commands::Gui => {
            println!("{}", "Launching Descartes GUI...".cyan());
            println!("{}", "Feature: Phase 3 - Iced UI (Planned)".yellow());
        }

        Commands::Plugins(cmd) => {
            plugins::execute(&cmd).await?;
        }

        Commands::Tasks(cmd) => {
            // Tasks use project-local SCG storage, not config-based path
            tasks::execute(&cmd, None).await?;
        }
    }

    Ok(())
}
