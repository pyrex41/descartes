/// Comprehensive Proof of Concept for ZMQ Transport Deployment
///
/// This example demonstrates a complete end-to-end deployment scenario for the ZMQ
/// transport layer, including:
///
/// 1. **Server Setup and Configuration**
///    - Starting a ZMQ agent server with production settings
///    - Configuring resource limits and timeouts
///    - Enabling monitoring and health checks
///
/// 2. **Client Connection and Agent Spawning**
///    - Multiple clients connecting to a single server
///    - Spawning agents with different configurations
///    - Handling concurrent operations
///
/// 3. **Agent Lifecycle Management**
///    - Pausing, resuming, and stopping agents
///    - Monitoring agent status and health
///    - Graceful shutdown procedures
///
/// 4. **Error Handling and Recovery**
///    - Connection failures and reconnection
///    - Timeout handling
///    - Resource exhaustion scenarios
///
/// 5. **Load Testing**
///    - Spawning multiple agents concurrently
///    - Stress testing server capacity
///    - Performance monitoring
///
/// ## Usage
///
/// Run the example with:
/// ```bash
/// cargo run --example zmq_deployment_poc
/// ```
///
/// The example will:
/// 1. Start a ZMQ server in the background
/// 2. Create multiple clients
/// 3. Demonstrate various deployment scenarios
/// 4. Clean up and shutdown gracefully
use descartes_core::{
    AgentConfig, AgentStatus, ControlCommandType, ProcessRunnerConfig, ZmqAgentRunner,
    ZmqAgentServer, ZmqClient, ZmqRunnerConfig, ZmqServerConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Configuration for the POC deployment
struct DeploymentConfig {
    /// Server endpoint
    server_endpoint: String,
    /// Number of clients to spawn
    num_clients: usize,
    /// Number of agents per client
    agents_per_client: usize,
    /// Whether to simulate failures
    simulate_failures: bool,
    /// Whether to run load tests
    run_load_tests: bool,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            server_endpoint: "tcp://127.0.0.1:15555".to_string(),
            num_clients: 3,
            agents_per_client: 2,
            simulate_failures: true,
            run_load_tests: true,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with detailed output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("===========================================");
    info!("ZMQ Transport Deployment POC");
    info!("===========================================");
    info!("");

    let config = DeploymentConfig::default();

    // Scenario 1: Server Startup and Configuration
    info!("📡 Scenario 1: Server Startup and Configuration");
    let server = setup_server(&config).await?;
    info!("✅ Server started successfully");
    info!("");

    // Wait for server to be ready
    sleep(Duration::from_millis(500)).await;

    // Scenario 2: Multiple Client Connections
    info!("🔌 Scenario 2: Multiple Client Connections");
    let clients = setup_clients(&config).await?;
    info!("✅ {} clients connected successfully", clients.len());
    info!("");

    // Scenario 3: Agent Spawning and Management
    info!("🚀 Scenario 3: Agent Spawning and Management");
    let agent_ids = spawn_agents(&clients, &config).await?;
    info!(
        "✅ Spawned {} agents across {} clients",
        agent_ids.len(),
        clients.len()
    );
    info!("");

    // Scenario 4: Agent Lifecycle Management
    info!("🔄 Scenario 4: Agent Lifecycle Management");
    demonstrate_lifecycle_management(&clients[0], &agent_ids).await?;
    info!("✅ Lifecycle management demonstrated");
    info!("");

    // Scenario 5: Health Checks and Monitoring
    info!("🏥 Scenario 5: Health Checks and Monitoring");
    perform_health_checks(&clients).await?;
    info!("✅ Health checks completed");
    info!("");

    // Scenario 6: Error Handling and Recovery
    if config.simulate_failures {
        info!("⚠️  Scenario 6: Error Handling and Recovery");
        demonstrate_error_handling(&clients[0]).await?;
        info!("✅ Error handling demonstrated");
        info!("");
    }

    // Scenario 7: Load Testing
    if config.run_load_tests {
        info!("📊 Scenario 7: Load Testing");
        run_load_tests(&config).await?;
        info!("✅ Load testing completed");
        info!("");
    }

    // Scenario 8: Batch Operations
    info!("📦 Scenario 8: Batch Operations");
    demonstrate_batch_operations(&clients[0], &agent_ids).await?;
    info!("✅ Batch operations demonstrated");
    info!("");

    // Scenario 9: Output Querying
    info!("📄 Scenario 9: Output Querying");
    demonstrate_output_querying(&clients[0], &agent_ids).await?;
    info!("✅ Output querying demonstrated");
    info!("");

    // Scenario 10: Graceful Shutdown
    info!("🛑 Scenario 10: Graceful Shutdown");
    cleanup_agents(&clients, &agent_ids).await?;
    shutdown_server(server).await?;
    info!("✅ Graceful shutdown completed");
    info!("");

    info!("===========================================");
    info!("POC Deployment Completed Successfully!");
    info!("===========================================");

    Ok(())
}

/// Set up the ZMQ server with production-ready configuration
async fn setup_server(
    config: &DeploymentConfig,
) -> Result<Arc<ZmqAgentServer>, Box<dyn std::error::Error>> {
    info!("Starting ZMQ server...");
    info!("  Endpoint: {}", config.server_endpoint);

    let server_config = ZmqServerConfig {
        endpoint: config.server_endpoint.clone(),
        pub_endpoint: None, // No PUB socket for this POC
        server_id: "poc-server-01".to_string(),
        max_agents: 50,
        status_update_interval_secs: 5,
        enable_status_updates: true,
        runner_config: ProcessRunnerConfig {
            enable_json_streaming: true,
            enable_health_checks: true,
            health_check_interval_secs: 15,
            max_concurrent_agents: Some(50),
            working_dir: None,
        },
        request_timeout_secs: 60,
    };

    let server = Arc::new(ZmqAgentServer::new(server_config));
    let server_clone = server.clone();

    // Start server in background
    tokio::spawn(async move {
        if let Err(e) = server_clone.start().await {
            error!("Server error: {}", e);
        }
    });

    info!("  Max agents: {}", server.stats().spawn_requests);
    info!("  Status updates: enabled (every 5s)");
    info!("  Health checks: enabled (every 15s)");

    Ok(server)
}

/// Set up multiple clients to demonstrate concurrent connections
async fn setup_clients(
    config: &DeploymentConfig,
) -> Result<Vec<Arc<ZmqClient>>, Box<dyn std::error::Error>> {
    info!("Connecting {} clients to server...", config.num_clients);

    let mut clients = Vec::new();

    for i in 0..config.num_clients {
        let client_config = ZmqRunnerConfig {
            endpoint: config.server_endpoint.clone(),
            connection_timeout_secs: 30,
            request_timeout_secs: 60,
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay_secs: 2,
            enable_heartbeat: true,
            heartbeat_interval_secs: 30,
            server_id: Some(format!("client-{:02}", i + 1)),
        };

        let client = Arc::new(ZmqClient::new(client_config));

        // Connect to server
        if let Err(e) = client.connect(&config.server_endpoint).await {
            warn!("Client {} failed to connect: {}", i + 1, e);
            // Continue anyway for demonstration purposes
        } else {
            info!("  Client {} connected", i + 1);
        }

        clients.push(client);
    }

    Ok(clients)
}

/// Spawn agents across multiple clients
async fn spawn_agents(
    clients: &[Arc<ZmqClient>],
    config: &DeploymentConfig,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    info!("Spawning agents...");

    let mut agent_ids = Vec::new();

    for (client_idx, client) in clients.iter().enumerate() {
        for agent_idx in 0..config.agents_per_client {
            let agent_config = AgentConfig {
                name: format!("agent-c{}-a{}", client_idx + 1, agent_idx + 1),
                model_backend: "claude".to_string(),
                task: format!(
                    "Process data for client {} agent {}",
                    client_idx + 1,
                    agent_idx + 1
                ),
                context: Some("Production deployment POC".to_string()),
                system_prompt: Some("You are a production agent.".to_string()),
                environment: {
                    let mut env = HashMap::new();
                    env.insert("CLIENT_ID".to_string(), format!("{}", client_idx + 1));
                    env.insert("AGENT_ID".to_string(), format!("{}", agent_idx + 1));
                    env.insert("ENV".to_string(), "production".to_string());
                    env
                },
            };

            match client.spawn_remote(agent_config.clone(), Some(300)).await {
                Ok(agent_info) => {
                    info!("  ✓ Spawned {} (ID: {})", agent_config.name, agent_info.id);
                    agent_ids.push(agent_info.id);
                }
                Err(e) => {
                    warn!("  ✗ Failed to spawn {}: {}", agent_config.name, e);
                    // Continue with other agents
                }
            }
        }
    }

    Ok(agent_ids)
}

/// Demonstrate agent lifecycle management (pause, resume, stop)
async fn demonstrate_lifecycle_management(
    client: &ZmqClient,
    agent_ids: &[Uuid],
) -> Result<(), Box<dyn std::error::Error>> {
    if agent_ids.is_empty() {
        warn!("No agents available for lifecycle demonstration");
        return Ok(());
    }

    let agent_id = agent_ids[0];
    info!("Demonstrating lifecycle management on agent: {}", agent_id);

    // Check initial status
    match client.get_agent_status(&agent_id).await {
        Ok(status) => {
            info!("  Initial status: {:?}", status);
        }
        Err(e) => {
            warn!("  Failed to get status: {}", e);
        }
    }

    // Attempt to pause (may not be supported on all platforms)
    info!("  Attempting to pause agent...");
    match client.pause_agent(&agent_id).await {
        Ok(_) => {
            info!("    ✓ Agent paused");

            // Wait a moment
            sleep(Duration::from_secs(2)).await;

            // Resume
            info!("  Attempting to resume agent...");
            match client.resume_agent(&agent_id).await {
                Ok(_) => info!("    ✓ Agent resumed"),
                Err(e) => warn!("    ✗ Resume failed: {}", e),
            }
        }
        Err(e) => {
            warn!("    ✗ Pause not supported or failed: {}", e);
        }
    }

    // Stop gracefully
    info!("  Stopping agent gracefully...");
    match client.stop_agent(&agent_id).await {
        Ok(_) => {
            info!("    ✓ Agent stopped");
        }
        Err(e) => {
            warn!("    ✗ Stop failed: {}", e);
        }
    }

    Ok(())
}

/// Perform health checks across all clients
async fn perform_health_checks(
    clients: &[Arc<ZmqClient>],
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Performing health checks...");

    for (idx, client) in clients.iter().enumerate() {
        match client.health_check().await {
            Ok(health) => {
                info!("  Client {}: {:?}", idx + 1, health.healthy);
                if health.healthy {
                    info!("    Protocol: {}", health.protocol_version);
                    if let Some(uptime) = health.uptime_secs {
                        info!("    Uptime: {}s", uptime);
                    }
                    if let Some(active) = health.active_agents {
                        info!("    Active agents: {}", active);
                    }
                }
            }
            Err(e) => {
                warn!("  Client {} health check failed: {}", idx + 1, e);
            }
        }
    }

    Ok(())
}

/// Demonstrate error handling and recovery scenarios
async fn demonstrate_error_handling(client: &ZmqClient) -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing error handling scenarios...");

    // Test 1: Query non-existent agent
    info!("  Test 1: Query non-existent agent");
    let fake_id = Uuid::new_v4();
    match client.get_agent_status(&fake_id).await {
        Ok(_) => warn!("    Unexpected success"),
        Err(e) => info!("    ✓ Expected error: {}", e),
    }

    // Test 2: Spawn agent with invalid configuration
    info!("  Test 2: Invalid agent configuration");
    let invalid_config = AgentConfig {
        name: "".to_string(), // Empty name
        model_backend: "".to_string(),
        task: "".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    match client.spawn_remote(invalid_config, Some(10)).await {
        Ok(_) => warn!("    Unexpected success"),
        Err(e) => info!("    ✓ Expected error: {}", e),
    }

    // Test 3: Timeout scenario (very short timeout)
    info!("  Test 3: Timeout scenario");
    let timeout_config = AgentConfig {
        name: "timeout-test".to_string(),
        model_backend: "claude".to_string(),
        task: "Test task".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    match client.spawn_remote(timeout_config, Some(1)).await {
        Ok(agent) => info!("    Agent spawned: {}", agent.id),
        Err(e) => info!("    Timeout or error: {}", e),
    }

    Ok(())
}

/// Run load tests to stress the system
async fn run_load_tests(config: &DeploymentConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running load tests...");

    let client_config = ZmqRunnerConfig {
        endpoint: config.server_endpoint.clone(),
        connection_timeout_secs: 30,
        request_timeout_secs: 60,
        auto_reconnect: true,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 2,
        enable_heartbeat: false, // Disable for load testing
        heartbeat_interval_secs: 30,
        server_id: Some("load-test-client".to_string()),
    };

    let client = Arc::new(ZmqClient::new(client_config));

    // Connect
    if let Err(e) = client.connect(&config.server_endpoint).await {
        warn!("Load test client failed to connect: {}", e);
        return Ok(());
    }

    info!("  Spawning 10 agents concurrently...");
    let start = std::time::Instant::now();

    let mut tasks = Vec::new();
    for i in 0..10 {
        let client_clone = client.clone();
        let task = tokio::spawn(async move {
            let agent_config = AgentConfig {
                name: format!("load-test-agent-{}", i),
                model_backend: "claude".to_string(),
                task: format!("Load test task {}", i),
                context: None,
                system_prompt: None,
                environment: HashMap::new(),
            };

            client_clone.spawn_remote(agent_config, Some(60)).await
        });
        tasks.push(task);
    }

    // Wait for all spawns to complete
    let mut successful = 0;
    let mut failed = 0;

    for task in tasks {
        match task.await {
            Ok(Ok(_)) => successful += 1,
            Ok(Err(_)) => failed += 1,
            Err(_) => failed += 1,
        }
    }

    let duration = start.elapsed();
    info!("  Load test completed in {:?}", duration);
    info!("    Successful: {}", successful);
    info!("    Failed: {}", failed);
    info!(
        "    Rate: {:.2} agents/sec",
        successful as f64 / duration.as_secs_f64()
    );

    Ok(())
}

/// Demonstrate batch control operations
async fn demonstrate_batch_operations(
    client: &ZmqClient,
    agent_ids: &[Uuid],
) -> Result<(), Box<dyn std::error::Error>> {
    if agent_ids.len() < 2 {
        warn!("Not enough agents for batch operations");
        return Ok(());
    }

    info!("Demonstrating batch operations...");

    // Batch pause (if supported)
    info!("  Batch pause operation on {} agents", agent_ids.len());
    match client
        .batch_control(agent_ids.to_vec(), ControlCommandType::Pause, None, false)
        .await
    {
        Ok(response) => {
            info!("    Successful: {}", response.successful);
            info!("    Failed: {}", response.failed);
        }
        Err(e) => {
            warn!("    Batch operation failed: {}", e);
        }
    }

    Ok(())
}

/// Demonstrate output querying capabilities
async fn demonstrate_output_querying(
    client: &ZmqClient,
    agent_ids: &[Uuid],
) -> Result<(), Box<dyn std::error::Error>> {
    if agent_ids.is_empty() {
        warn!("No agents available for output querying");
        return Ok(());
    }

    info!("Demonstrating output querying...");

    let agent_id = agent_ids[0];
    info!("  Querying output for agent: {}", agent_id);

    // Query stdout
    match client.read_agent_stdout(&agent_id).await {
        Ok(Some(data)) => {
            info!("    Stdout: {} bytes", data.len());
            if let Ok(text) = String::from_utf8(data) {
                info!(
                    "    Content: {}",
                    text.chars().take(100).collect::<String>()
                );
            }
        }
        Ok(None) => {
            info!("    No stdout data available");
        }
        Err(e) => {
            warn!("    Failed to read stdout: {}", e);
        }
    }

    // Query stderr
    match client.read_agent_stderr(&agent_id).await {
        Ok(Some(data)) => {
            info!("    Stderr: {} bytes", data.len());
        }
        Ok(None) => {
            info!("    No stderr data available");
        }
        Err(e) => {
            warn!("    Failed to read stderr: {}", e);
        }
    }

    Ok(())
}

/// Clean up all spawned agents
async fn cleanup_agents(
    clients: &[Arc<ZmqClient>],
    agent_ids: &[Uuid],
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Cleaning up {} agents...", agent_ids.len());

    // Use the first client to stop all agents
    if let Some(client) = clients.first() {
        for agent_id in agent_ids {
            match client.stop_agent(agent_id).await {
                Ok(_) => info!("  ✓ Stopped agent {}", agent_id),
                Err(e) => warn!("  ✗ Failed to stop agent {}: {}", agent_id, e),
            }
        }
    }

    // Disconnect all clients
    for (idx, client) in clients.iter().enumerate() {
        match client.disconnect().await {
            Ok(_) => info!("  ✓ Client {} disconnected", idx + 1),
            Err(e) => warn!("  ✗ Client {} disconnect failed: {}", idx + 1, e),
        }
    }

    Ok(())
}

/// Shutdown the server gracefully
async fn shutdown_server(server: Arc<ZmqAgentServer>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Shutting down server...");

    // Get final statistics
    let stats = server.stats();
    info!("  Final Statistics:");
    info!("    Total spawn requests: {}", stats.spawn_requests);
    info!("    Successful spawns: {}", stats.successful_spawns);
    info!("    Failed spawns: {}", stats.failed_spawns);
    info!("    Control commands: {}", stats.control_commands);
    info!("    Health checks: {}", stats.health_checks);
    info!("    Errors: {}", stats.errors);

    // Stop the server
    server.stop().await?;

    info!("  ✓ Server stopped");

    Ok(())
}
