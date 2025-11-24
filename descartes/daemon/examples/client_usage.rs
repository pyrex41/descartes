/// Example demonstrating RPC client usage
///
/// This example shows how to use the RPC client to interact with the Descartes daemon.
/// Before running this example, start the daemon:
///   cargo run --bin descartes-daemon
///
/// Then run this example:
///   cargo run --example client_usage

use descartes_daemon::{RpcClient, RpcClientBuilder};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Descartes RPC Client Example ===\n");

    // Method 1: Create client with default settings
    println!("1. Creating client with default URL (http://127.0.0.1:8080)...");
    let client = RpcClient::default_client()?;
    println!("   ✓ Client created\n");

    // Method 2: Create client with custom URL
    println!("2. Creating client with custom URL...");
    let _custom_client = RpcClient::with_url("http://localhost:8080")?;
    println!("   ✓ Custom client created\n");

    // Method 3: Create client with builder pattern
    println!("3. Creating client with builder pattern...");
    let _builder_client = RpcClientBuilder::new()
        .url("http://127.0.0.1:8080")
        .timeout(60)
        .max_retries(3)
        .retry_delay(100)
        .pool_size(20)
        .build()?;
    println!("   ✓ Builder client created\n");

    // Test connection
    println!("4. Testing connection to daemon...");
    match client.test_connection().await {
        Ok(_) => println!("   ✓ Connection successful\n"),
        Err(e) => {
            eprintln!("   ✗ Connection failed: {}", e);
            eprintln!("\n   Please make sure the daemon is running:");
            eprintln!("   cargo run --bin descartes-daemon\n");
            return Ok(());
        }
    }

    // Get health status
    println!("5. Checking system health...");
    let health = client.health().await?;
    println!("   Status: {}", health.status);
    println!("   Version: {}", health.version);
    println!("   Uptime: {} seconds", health.uptime_secs);
    println!("   Timestamp: {}\n", health.timestamp);

    // List agents
    println!("6. Listing agents...");
    let agents_response = client.list_agents().await?;
    println!("   Found {} agents", agents_response.count);
    for agent in &agents_response.agents {
        println!("   - {} ({}): {:?}", agent.name, agent.id, agent.status);
    }
    println!();

    // Spawn a new agent
    println!("7. Spawning a test agent...");
    let spawn_result = client
        .spawn_agent("example-agent", "basic", json!({"test": true}))
        .await?;
    println!("   ✓ Agent spawned");
    println!("   Agent ID: {}", spawn_result.agent_id);
    println!("   Status: {:?}", spawn_result.status);
    println!("   Message: {}\n", spawn_result.message);

    let agent_id = spawn_result.agent_id;

    // Get agent logs
    println!("8. Fetching agent logs...");
    let logs_response = client.get_agent_logs(&agent_id, Some(10), None).await?;
    println!("   Total logs: {}", logs_response.total);
    for (i, log) in logs_response.logs.iter().enumerate() {
        println!("   [{}] {} - {}: {}", i + 1, log.timestamp, log.level, log.message);
    }
    println!();

    // Query state
    println!("9. Querying system state...");
    let state_response = client.query_state(Some(&agent_id), None).await?;
    println!("   State entries: {}", state_response.state.len());
    println!("   Timestamp: {}\n", state_response.timestamp);

    // Get metrics
    println!("10. Getting system metrics...");
    let metrics = client.metrics().await?;
    println!("   Agent metrics:");
    println!("     Total: {}", metrics.agents.total);
    println!("     Running: {}", metrics.agents.running);
    println!("     Paused: {}", metrics.agents.paused);
    println!("     Stopped: {}", metrics.agents.stopped);
    println!("   System metrics:");
    println!("     Uptime: {} seconds", metrics.system.uptime_secs);
    println!("     Memory: {:.2} MB", metrics.system.memory_usage_mb);
    println!("     CPU: {:.1}%", metrics.system.cpu_usage_percent);
    println!("     Connections: {}\n", metrics.system.active_connections);

    // Execute workflow (example)
    println!("11. Executing workflow...");
    match client
        .execute_workflow("test-workflow", vec![agent_id.clone()], json!({}))
        .await
    {
        Ok(workflow_result) => {
            println!("   ✓ Workflow started");
            println!("   Execution ID: {}", workflow_result.execution_id);
            println!("   Status: {}\n", workflow_result.status);
        }
        Err(e) => {
            println!("   ✗ Workflow execution failed: {}\n", e);
        }
    }

    // Batch request example
    println!("12. Sending batch requests...");
    let batch_requests = vec![
        ("system.health", None),
        ("agent.list", Some(json!({}))),
        ("system.metrics", None),
    ];

    let batch_results = client.batch_call(batch_requests).await?;
    println!("   ✓ Batch completed with {} responses\n", batch_results.len());

    // Kill the agent
    println!("13. Killing test agent...");
    let kill_result = client.kill_agent(&agent_id, false).await?;
    println!("   ✓ Agent killed");
    println!("   Agent ID: {}", kill_result.agent_id);
    println!("   Status: {:?}", kill_result.status);
    println!("   Message: {}\n", kill_result.message);

    // Final agent list
    println!("14. Final agent list...");
    let final_agents = client.list_agents().await?;
    println!("   Found {} agents\n", final_agents.count);

    println!("=== Example completed successfully! ===");

    Ok(())
}
