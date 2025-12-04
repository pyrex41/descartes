/// Demo CLI for testing the agent runner
///
/// Usage:
///   cargo run --bin agent_runner_demo spawn <name> <backend> <task>
///   cargo run --bin agent_runner_demo list
///   cargo run --bin agent_runner_demo kill <id>
use descartes_core::{AgentConfig, AgentRunner, LocalProcessRunner, ProcessRunnerConfig};
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "spawn" => {
            if args.len() < 5 {
                eprintln!("Usage: agent_runner_demo spawn <name> <backend> <task>");
                return Ok(());
            }

            let name = &args[2];
            let backend = &args[3];
            let task = args[4..].join(" ");

            spawn_agent(name, backend, &task).await?;
        }
        "list" => {
            list_agents().await?;
        }
        "kill" => {
            if args.len() < 3 {
                eprintln!("Usage: agent_runner_demo kill <id>");
                return Ok(());
            }

            let id = &args[2];
            kill_agent(id).await?;
        }
        "help" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Agent Runner Demo");
    println!();
    println!("Commands:");
    println!("  spawn <name> <backend> <task>  - Spawn a new agent");
    println!("  list                           - List all agents");
    println!("  kill <id>                      - Kill an agent");
    println!("  help                           - Show this help");
    println!();
    println!("Examples:");
    println!("  cargo run --bin agent_runner_demo spawn my-agent claude 'Write a poem'");
    println!("  cargo run --bin agent_runner_demo list");
    println!("  cargo run --bin agent_runner_demo kill <agent-id>");
}

async fn spawn_agent(
    name: &str,
    backend: &str,
    task: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Spawning agent...");
    println!("  Name: {}", name);
    println!("  Backend: {}", backend);
    println!("  Task: {}", task);

    let config = ProcessRunnerConfig::default();
    let runner = LocalProcessRunner::with_config(config);

    let agent_config = AgentConfig {
        name: name.to_string(),
        model_backend: backend.to_string(),
        task: task.to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    match runner.spawn(agent_config).await {
        Ok(handle) => {
            println!("\n✓ Agent spawned successfully!");
            println!("  ID: {}", handle.id());
            println!("  Status: {:?}", handle.status());
            println!("\nNote: Use the ID to interact with this agent");
        }
        Err(e) => {
            eprintln!("\n✗ Failed to spawn agent: {}", e);
            eprintln!("\nPossible reasons:");
            eprintln!("  - CLI tool '{}' not installed", backend);
            eprintln!("  - Backend '{}' not supported", backend);
            eprintln!("  - Insufficient permissions");
            return Err(e.into());
        }
    }

    Ok(())
}

async fn list_agents() -> Result<(), Box<dyn std::error::Error>> {
    let runner = LocalProcessRunner::new();

    let agents = runner.list_agents().await?;

    if agents.is_empty() {
        println!("No agents running");
        return Ok(());
    }

    println!("Running agents: {}", agents.len());
    println!();

    for agent in agents {
        println!("┌─ {} ({})", agent.name, agent.id);
        println!("├─ Backend: {}", agent.model_backend);
        println!("├─ Status: {:?}", agent.status);
        println!("├─ Started: {:?}", agent.started_at);
        println!("└─ Task: {}", agent.task);
        println!();
    }

    Ok(())
}

async fn kill_agent(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    use uuid::Uuid;

    let agent_id = Uuid::parse_str(id)?;

    println!("Killing agent {}...", agent_id);

    let runner = LocalProcessRunner::new();

    match runner.kill(&agent_id).await {
        Ok(_) => {
            println!("✓ Agent killed successfully");
        }
        Err(e) => {
            eprintln!("✗ Failed to kill agent: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
