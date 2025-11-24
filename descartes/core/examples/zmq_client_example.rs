/// Example usage of the ZMQ client for remote agent management
///
/// This example demonstrates how to:
/// - Create a ZMQ client
/// - Connect to a remote server
/// - Spawn remote agents
/// - Control agents (pause, resume, stop)
/// - Monitor agent status
/// - Handle I/O operations
/// - Subscribe to status updates

use descartes_core::{
    AgentConfig, AgentStatus, ZmqAgentRunner, ZmqClient, ZmqRunnerConfig,
};
use futures::StreamExt;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("descartes_core=debug,zmq_client_example=info")
        .init();

    println!("=== ZMQ Client Example ===\n");

    // Example 1: Basic client creation and connection
    basic_client_example().await?;

    // Example 2: Spawn and manage agents
    spawn_agent_example().await?;

    // Example 3: List and query agents
    list_agents_example().await?;

    // Example 4: Health check
    health_check_example().await?;

    // Example 5: Status update subscription
    status_update_example().await?;

    Ok(())
}

/// Example 1: Basic client creation and connection
async fn basic_client_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Basic Client Creation\n");

    // Create configuration
    let config = ZmqRunnerConfig {
        endpoint: "tcp://localhost:5555".to_string(),
        connection_timeout_secs: 30,
        request_timeout_secs: 30,
        auto_reconnect: true,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        enable_heartbeat: true,
        heartbeat_interval_secs: 30,
        server_id: Some("example-server".to_string()),
    };

    // Create client
    let client = ZmqClient::new(config);

    println!("✓ Created ZMQ client");
    println!("✓ Endpoint: tcp://localhost:5555");
    println!("✓ Auto-reconnect: enabled");
    println!("✓ Heartbeat: enabled\n");

    // Note: In a real application, you would call:
    // client.connect("tcp://localhost:5555").await?;

    Ok(())
}

/// Example 2: Spawn and manage agents
async fn spawn_agent_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Spawn and Manage Agents\n");

    let config = ZmqRunnerConfig::default();
    let client = ZmqClient::new(config);

    // Note: In a real application, connect first:
    // client.connect("tcp://localhost:5555").await?;

    // Create agent configuration
    let agent_config = AgentConfig {
        name: "data-processor".to_string(),
        model_backend: "claude".to_string(),
        task: "Process and analyze system logs".to_string(),
        context: Some("Production server logs from the last 24 hours".to_string()),
        system_prompt: Some("You are an expert log analyst. Focus on identifying errors and security issues.".to_string()),
        environment: {
            let mut env = HashMap::new();
            env.insert("LOG_LEVEL".to_string(), "DEBUG".to_string());
            env.insert("OUTPUT_FORMAT".to_string(), "JSON".to_string());
            env
        },
    };

    println!("Agent Configuration:");
    println!("  Name: {}", agent_config.name);
    println!("  Backend: {}", agent_config.model_backend);
    println!("  Task: {}", agent_config.task);
    println!("\nNote: In a real application, you would spawn with:");
    println!("  let agent = client.spawn_remote(agent_config, Some(300)).await?;");
    println!("  println!(\"Spawned agent: {{}} ({{}})\", agent.name, agent.id);\n");

    // Agent control operations
    println!("Agent Control Operations:");
    println!("  client.pause_agent(&agent_id).await?;  // Pause");
    println!("  client.resume_agent(&agent_id).await?; // Resume");
    println!("  client.stop_agent(&agent_id).await?;   // Stop\n");

    Ok(())
}

/// Example 3: List and query agents
async fn list_agents_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: List and Query Agents\n");

    let config = ZmqRunnerConfig::default();
    let client = ZmqClient::new(config);

    println!("List all running agents:");
    println!("  let agents = client.list_remote_agents(Some(AgentStatus::Running), None).await?;");
    println!("  for agent in agents {{");
    println!("      println!(\"{{}} - {{}}\", agent.name, agent.id);");
    println!("  }}\n");

    println!("Get specific agent:");
    println!("  if let Some(agent) = client.get_remote_agent(&agent_id).await? {{");
    println!("      println!(\"Agent: {{:?}}\", agent);");
    println!("  }}\n");

    println!("Get agent status:");
    println!("  let status = client.get_agent_status(&agent_id).await?;");
    println!("  println!(\"Status: {{:?}}\", status);\n");

    Ok(())
}

/// Example 4: Health check
async fn health_check_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Health Check\n");

    let config = ZmqRunnerConfig::default();
    let client = ZmqClient::new(config);

    println!("Perform health check:");
    println!("  let health = client.health_check().await?;");
    println!("  ");
    println!("  if health.healthy {{");
    println!("      println!(\"Server is healthy\");");
    println!("      println!(\"Protocol version: {{}}\", health.protocol_version);");
    println!("      ");
    println!("      if let Some(uptime) = health.uptime_secs {{");
    println!("          println!(\"Uptime: {{}} seconds\", uptime);");
    println!("      }}");
    println!("      ");
    println!("      if let Some(active) = health.active_agents {{");
    println!("          println!(\"Active agents: {{}}\", active);");
    println!("      }}");
    println!("  }} else {{");
    println!("      println!(\"Server is unhealthy!\");");
    println!("  }}\n");

    Ok(())
}

/// Example 5: Status update subscription
async fn status_update_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 5: Status Update Subscription\n");

    let config = ZmqRunnerConfig::default();
    let client = ZmqClient::new(config);

    println!("Subscribe to status updates:");
    println!("  let mut updates = client.subscribe_status_updates(None).await?;");
    println!("  ");
    println!("  while let Some(update_result) = updates.next().await {{");
    println!("      match update_result {{");
    println!("          Ok(update) => {{");
    println!("              println!(\"Update: {{}} - {{:?}}\", ");
    println!("                       update.agent_id, update.update_type);");
    println!("              ");
    println!("              if let Some(status) = update.status {{");
    println!("                  println!(\"  Status: {{:?}}\", status);");
    println!("              }}");
    println!("              ");
    println!("              if let Some(msg) = update.message {{");
    println!("                  println!(\"  Message: {{}}\", msg);");
    println!("              }}");
    println!("          }}");
    println!("          Err(e) => {{");
    println!("              eprintln!(\"Error: {{}}\", e);");
    println!("          }}");
    println!("      }}");
    println!("  }}\n");

    Ok(())
}
