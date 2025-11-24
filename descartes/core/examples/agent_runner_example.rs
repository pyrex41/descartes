/// Example: Using the LocalProcessRunner to spawn and manage agent processes
///
/// This example demonstrates:
/// 1. Creating a process runner with custom configuration
/// 2. Spawning multiple agents
/// 3. Managing agent lifecycle (start, pause, resume, stop)
/// 4. Streaming stdio (stdin/stdout/stderr)
/// 5. Health monitoring
/// 6. Graceful shutdown
///
/// Run with: cargo run --example agent_runner_example

use descartes_core::{
    AgentConfig, AgentRunner, AgentSignal, AgentStatus, LocalProcessRunner, ProcessRunnerConfig,
    GracefulShutdown,
};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Agent Runner Example ===\n");

    // Example 1: Basic Process Runner Setup
    println!("1. Creating a LocalProcessRunner...");
    let config = ProcessRunnerConfig {
        working_dir: None,
        enable_json_streaming: true,
        enable_health_checks: true,
        health_check_interval_secs: 10,
        max_concurrent_agents: Some(5),
    };
    let runner = LocalProcessRunner::with_config(config);
    println!("   ✓ Process runner created\n");

    // Example 2: Spawning an Agent
    println!("2. Spawning a Claude CLI agent...");
    let mut env = HashMap::new();
    env.insert("ANTHROPIC_API_KEY".to_string(), "your-api-key".to_string());

    let agent_config = AgentConfig {
        name: "research-assistant".to_string(),
        model_backend: "claude".to_string(),
        task: "Research the latest AI developments and summarize findings".to_string(),
        context: Some("Focus on large language models and agent frameworks".to_string()),
        system_prompt: Some("You are a research assistant specializing in AI".to_string()),
        environment: env,
    };

    // Note: This will fail if Claude CLI is not installed
    match runner.spawn(agent_config.clone()).await {
        Ok(mut handle) => {
            println!("   ✓ Agent spawned with ID: {}", handle.id());
            println!("   Status: {:?}", handle.status());

            // Example 3: Interacting with the Agent
            println!("\n3. Sending input to agent...");
            let input = "Please begin the research task.\n";
            handle.write_stdin(input.as_bytes()).await?;
            println!("   ✓ Input sent");

            // Example 4: Reading Output
            println!("\n4. Reading agent output...");
            sleep(Duration::from_secs(2)).await;

            let mut output_count = 0;
            while let Ok(Some(output)) = handle.read_stdout().await {
                println!("   [stdout] {}", String::from_utf8_lossy(&output));
                output_count += 1;
                if output_count >= 5 {
                    break; // Limit output for example
                }
            }

            // Check stderr
            while let Ok(Some(output)) = handle.read_stderr().await {
                println!("   [stderr] {}", String::from_utf8_lossy(&output));
            }

            // Example 5: Graceful Shutdown
            println!("\n5. Performing graceful shutdown...");
            let shutdown = GracefulShutdown::new(5);
            shutdown.shutdown(&mut handle).await?;
            println!("   ✓ Agent shut down gracefully");
        }
        Err(e) => {
            println!("   ✗ Failed to spawn agent: {}", e);
            println!("   (This is expected if Claude CLI is not installed)");
        }
    }

    // Example 6: Managing Multiple Agents
    println!("\n6. Spawning multiple agents...");

    for i in 1..=3 {
        let config = AgentConfig {
            name: format!("agent-{}", i),
            model_backend: "claude".to_string(),
            task: format!("Task {}", i),
            context: None,
            system_prompt: None,
            environment: HashMap::new(),
        };

        match runner.spawn(config).await {
            Ok(handle) => {
                println!("   ✓ Spawned agent-{} with ID: {}", i, handle.id());
            }
            Err(e) => {
                println!("   ✗ Failed to spawn agent-{}: {}", i, e);
            }
        }
    }

    // List all running agents
    println!("\n7. Listing all agents...");
    let agents = runner.list_agents().await?;
    println!("   Total agents: {}", agents.len());
    for agent in &agents {
        println!("   - {} ({}): {:?}", agent.name, agent.id, agent.status);
    }

    // Example 8: Signal Handling
    if let Some(agent) = agents.first() {
        println!("\n8. Sending signals to agent {}...", agent.id);

        // Send interrupt signal (SIGINT)
        println!("   Sending SIGINT...");
        if let Err(e) = runner.signal(&agent.id, AgentSignal::Interrupt).await {
            println!("   ✗ Failed: {}", e);
        } else {
            println!("   ✓ SIGINT sent");
        }

        sleep(Duration::from_secs(1)).await;

        // Send terminate signal (SIGTERM)
        println!("   Sending SIGTERM...");
        if let Err(e) = runner.signal(&agent.id, AgentSignal::Terminate).await {
            println!("   ✗ Failed: {}", e);
        } else {
            println!("   ✓ SIGTERM sent");
        }
    }

    // Example 9: Cleanup
    println!("\n9. Cleaning up all agents...");
    let agents = runner.list_agents().await?;
    for agent in agents {
        println!("   Killing agent {}...", agent.id);
        if let Err(e) = runner.kill(&agent.id).await {
            println!("   ✗ Failed: {}", e);
        } else {
            println!("   ✓ Agent killed");
        }
    }

    println!("\n=== Example Complete ===");
    Ok(())
}

// Additional example: Custom agent implementation
#[allow(dead_code)]
async fn custom_agent_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Custom Agent Example ===\n");

    // Create a process runner with custom configuration
    let config = ProcessRunnerConfig {
        working_dir: Some(std::path::PathBuf::from("/tmp")),
        enable_json_streaming: true,
        enable_health_checks: true,
        health_check_interval_secs: 5,
        max_concurrent_agents: Some(10),
    };

    let runner = LocalProcessRunner::with_config(config);

    // Spawn an agent with full configuration
    let mut env = HashMap::new();
    env.insert("LOG_LEVEL".to_string(), "debug".to_string());
    env.insert("API_KEY".to_string(), "secret-key".to_string());

    let agent_config = AgentConfig {
        name: "data-processor".to_string(),
        model_backend: "opencode-cli".to_string(),
        task: "Process data files in /tmp/data".to_string(),
        context: Some("CSV files with customer data".to_string()),
        system_prompt: Some("You are a data processing assistant".to_string()),
        environment: env,
    };

    let mut handle = runner.spawn(agent_config).await?;

    // Stream processing loop
    println!("Starting agent processing loop...");
    for i in 1..=10 {
        // Send work item
        let work = format!("Process item {}\n", i);
        handle.write_stdin(work.as_bytes()).await?;

        // Read response
        sleep(Duration::from_millis(100)).await;
        if let Ok(Some(output)) = handle.read_stdout().await {
            println!("Processed: {}", String::from_utf8_lossy(&output));
        }
    }

    // Wait for completion
    println!("Waiting for agent to complete...");
    let exit_status = handle.wait().await?;
    println!("Agent completed with status: {:?}", exit_status);

    if let Some(code) = handle.exit_code() {
        println!("Exit code: {}", code);
    }

    Ok(())
}

// Example: Error handling and recovery
#[allow(dead_code)]
async fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Error Handling Example ===\n");

    let runner = LocalProcessRunner::new();

    // Try to spawn an agent with invalid backend
    let config = AgentConfig {
        name: "invalid-agent".to_string(),
        model_backend: "nonexistent-backend".to_string(),
        task: "This will fail".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    match runner.spawn(config).await {
        Ok(_) => println!("Unexpectedly succeeded"),
        Err(e) => {
            println!("Expected error: {}", e);
            // Handle error appropriately
            // - Log the error
            // - Retry with different configuration
            // - Notify monitoring system
            // - etc.
        }
    }

    // Try to kill non-existent agent
    let fake_id = uuid::Uuid::new_v4();
    match runner.kill(&fake_id).await {
        Ok(_) => println!("Unexpectedly succeeded"),
        Err(e) => {
            println!("Expected error: {}", e);
        }
    }

    // Try to get non-existent agent
    match runner.get_agent(&fake_id).await {
        Ok(None) => println!("Agent not found (as expected)"),
        Ok(Some(_)) => println!("Unexpectedly found agent"),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
