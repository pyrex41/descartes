/// Descartes CLI - Command-line interface for the orchestration system
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "descartes")]
#[command(about = "Composable AI Agent Orchestration System", long_about = None)]
#[command(version = "0.1.0")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Descartes project
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
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
    },

    /// List running agents
    Ps,

    /// Kill an agent
    Kill {
        /// Agent ID
        id: String,
    },

    /// View agent logs
    Logs {
        /// Agent ID (optional - show all if not provided)
        id: Option<String>,

        /// Follow logs
        #[arg(short, long)]
        follow: bool,
    },

    /// Launch the GUI
    Gui,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Commands::Init { name } => {
            println!(
                "Initializing Descartes project: {}",
                name.as_deref().unwrap_or("current")
            );
            println!("Feature: Phase 1 - Foundation (Planned)");
        }

        Commands::Spawn {
            task,
            provider,
            model,
            system,
        } => {
            println!("Spawning agent...");
            println!("  Task: {}", task);
            println!("  Provider: {}", provider.as_deref().unwrap_or("anthropic"));
            println!("  Model: {}", model.as_deref().unwrap_or("default"));
            if let Some(sys) = system {
                println!("  System: {}", sys);
            }
            println!("Feature: Phase 1 - Agent execution (Planned)");
        }

        Commands::Ps => {
            println!("Running agents:");
            println!("Feature: Phase 1 - Agent listing (Planned)");
        }

        Commands::Kill { id } => {
            println!("Killing agent: {}", id);
            println!("Feature: Phase 1 - Agent termination (Planned)");
        }

        Commands::Logs { id, follow } => {
            if let Some(agent_id) = id {
                println!("Showing logs for agent: {}", agent_id);
            } else {
                println!("Showing all logs");
            }
            if follow {
                println!("Following logs...");
            }
            println!("Feature: Phase 1 - Logging (Planned)");
        }

        Commands::Gui => {
            println!("Launching Descartes GUI...");
            println!("Feature: Phase 3 - Iced UI (Planned)");
        }
    }

    Ok(())
}
