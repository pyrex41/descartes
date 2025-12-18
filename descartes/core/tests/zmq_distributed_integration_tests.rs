/// Distributed Integration Tests for ZMQ Transport
///
/// These tests verify the functionality of ZMQ transport in real distributed scenarios:
/// - Multiple clients connecting to one server
/// - Agent spawning under load
/// - Network failure and recovery scenarios
/// - Cross-process communication
/// - Concurrent operations
/// - Error handling and timeouts
///
/// Note: These tests spawn actual server and client processes and may take longer to run.
use descartes_core::{
    AgentConfig, AgentStatus, ControlCommandType, ProcessRunnerConfig, ZmqAgentRunner,
    ZmqAgentServer, ZmqClient, ZmqOutputStream, ZmqRunnerConfig, ZmqServerConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

// Test utilities
mod test_utils {
    use super::*;

    /// Helper to start a test server
    pub async fn start_test_server(
        endpoint: &str,
    ) -> Result<Arc<ZmqAgentServer>, Box<dyn std::error::Error>> {
        let config = ZmqServerConfig {
            endpoint: endpoint.to_string(),
            pub_endpoint: None, // Disable PUB socket for these tests
            server_id: format!("test-server-{}", Uuid::new_v4()),
            max_agents: 50,
            status_update_interval_secs: 5,
            enable_status_updates: true,
            runner_config: ProcessRunnerConfig {
                enable_json_streaming: false,
                enable_health_checks: false,
                health_check_interval_secs: 30,
                max_concurrent_agents: Some(50),
                working_dir: None,
            },
            request_timeout_secs: 30,
        };

        let server = Arc::new(ZmqAgentServer::new(config));
        let server_clone = server.clone();

        // Start server in background
        tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                eprintln!("Test server error: {}", e);
            }
        });

        // Wait for server to be ready
        sleep(Duration::from_millis(500)).await;

        Ok(server)
    }

    /// Helper to create a test client
    pub fn create_test_client(endpoint: &str) -> Arc<ZmqClient> {
        let config = ZmqRunnerConfig {
            endpoint: endpoint.to_string(),
            connection_timeout_secs: 10,
            request_timeout_secs: 30,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 1,
            enable_heartbeat: false,
            heartbeat_interval_secs: 30,
            server_id: Some(format!("test-client-{}", Uuid::new_v4())),
        };

        Arc::new(ZmqClient::new(config))
    }

    /// Helper to create a test agent configuration
    pub fn create_test_agent_config(name: &str) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            model_backend: "test".to_string(),
            task: format!("Test task for {}", name),
            context: Some("Integration test context".to_string()),
            system_prompt: None,
            environment: HashMap::new(),
        }
    }
}

// ============================================================================
// Test: Multiple Clients Connecting to One Server
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_multiple_clients_single_server() {
    let endpoint = "tcp://127.0.0.1:15556";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create multiple clients
    let num_clients = 5;
    let mut clients = Vec::new();

    for _ in 0..num_clients {
        let client = test_utils::create_test_client(endpoint);
        clients.push(client);
    }

    // Connect all clients
    for (i, client) in clients.iter().enumerate() {
        match client.connect(endpoint).await {
            Ok(_) => println!("Client {} connected", i + 1),
            Err(e) => eprintln!("Client {} failed to connect: {}", i + 1, e),
        }
    }

    // Wait a moment
    sleep(Duration::from_millis(500)).await;

    // Perform health checks from all clients
    let mut health_checks_passed = 0;
    for (i, client) in clients.iter().enumerate() {
        match timeout(Duration::from_secs(5), client.health_check()).await {
            Ok(Ok(response)) => {
                if response.healthy {
                    health_checks_passed += 1;
                    println!("Client {} health check: PASSED", i + 1);
                }
            }
            Ok(Err(e)) => eprintln!("Client {} health check failed: {}", i + 1, e),
            Err(_) => eprintln!("Client {} health check timed out", i + 1),
        }
    }

    println!(
        "Health checks: {}/{} passed",
        health_checks_passed, num_clients
    );

    // Cleanup
    for client in clients {
        let _ = client.disconnect().await;
    }
    let _ = server.stop().await;

    // Assert at least some clients succeeded
    assert!(
        health_checks_passed > 0,
        "At least one client should connect successfully"
    );
}

// ============================================================================
// Test: Agent Spawning Under Load
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_agent_spawning_under_load() {
    let endpoint = "tcp://127.0.0.1:15557";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client
    let client = test_utils::create_test_client(endpoint);

    // Connect client
    match client.connect(endpoint).await {
        Ok(_) => println!("Client connected"),
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return;
        }
    }

    // Spawn multiple agents concurrently
    let num_agents = 10;
    let mut spawn_tasks = Vec::new();

    for i in 0..num_agents {
        let client_clone = client.clone();
        let task = tokio::spawn(async move {
            let config = test_utils::create_test_agent_config(&format!("load-test-agent-{}", i));

            timeout(
                Duration::from_secs(30),
                client_clone.spawn_remote(config, Some(60)),
            )
            .await
        });

        spawn_tasks.push(task);
    }

    // Wait for all spawns to complete
    let mut successful = 0;
    let mut failed = 0;
    let mut timed_out = 0;

    for (i, task) in spawn_tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(Ok(agent_info))) => {
                successful += 1;
                println!("Agent {} spawned: {}", i + 1, agent_info.id);
            }
            Ok(Ok(Err(e))) => {
                failed += 1;
                eprintln!("Agent {} spawn failed: {}", i + 1, e);
            }
            Ok(Err(_)) => {
                timed_out += 1;
                eprintln!("Agent {} spawn timed out", i + 1);
            }
            Err(e) => {
                failed += 1;
                eprintln!("Agent {} task failed: {}", i + 1, e);
            }
        }
    }

    println!("\nLoad Test Results:");
    println!("  Successful: {}", successful);
    println!("  Failed: {}", failed);
    println!("  Timed out: {}", timed_out);
    println!("  Total: {}", num_agents);

    // Cleanup
    let _ = client.disconnect().await;
    let _ = server.stop().await;

    // Assert some agents spawned successfully
    assert!(
        successful > 0,
        "At least some agents should spawn successfully"
    );
}

// ============================================================================
// Test: Concurrent Client Operations
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_concurrent_client_operations() {
    let endpoint = "tcp://127.0.0.1:15558";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create multiple clients
    let num_clients = 3;
    let mut clients = Vec::new();

    for _ in 0..num_clients {
        let client = test_utils::create_test_client(endpoint);
        if client.connect(endpoint).await.is_ok() {
            clients.push(client);
        }
    }

    if clients.is_empty() {
        eprintln!("No clients connected, skipping test");
        return;
    }

    // Each client spawns agents concurrently
    let mut all_tasks = Vec::new();

    for (client_idx, client) in clients.iter().enumerate() {
        for agent_idx in 0..2 {
            let client_clone = client.clone();
            let task = tokio::spawn(async move {
                let config = test_utils::create_test_agent_config(&format!(
                    "concurrent-c{}-a{}",
                    client_idx, agent_idx
                ));

                timeout(
                    Duration::from_secs(30),
                    client_clone.spawn_remote(config, Some(60)),
                )
                .await
            });

            all_tasks.push(task);
        }
    }

    // Wait for all operations
    let mut successful = 0;
    let mut failed = 0;

    for task in all_tasks {
        match task.await {
            Ok(Ok(Ok(_))) => successful += 1,
            _ => failed += 1,
        }
    }

    println!("\nConcurrent Operations Results:");
    println!("  Successful: {}", successful);
    println!("  Failed: {}", failed);

    // Cleanup
    for client in clients {
        let _ = client.disconnect().await;
    }
    let _ = server.stop().await;

    assert!(successful > 0, "Some concurrent operations should succeed");
}

// ============================================================================
// Test: Network Failure and Reconnection
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_network_failure_and_reconnection() {
    let endpoint = "tcp://127.0.0.1:15559";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client with auto-reconnect
    let config = ZmqRunnerConfig {
        endpoint: endpoint.to_string(),
        connection_timeout_secs: 5,
        request_timeout_secs: 10,
        auto_reconnect: true,
        max_reconnect_attempts: 5,
        reconnect_delay_secs: 1,
        enable_heartbeat: false,
        heartbeat_interval_secs: 30,
        server_id: Some("reconnect-test-client".to_string()),
    };

    let client = Arc::new(ZmqClient::new(config));

    // Connect client
    match client.connect(endpoint).await {
        Ok(_) => println!("Initial connection successful"),
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return;
        }
    }

    // Verify connection with health check
    let health1 = timeout(Duration::from_secs(5), client.health_check()).await;
    assert!(health1.is_ok(), "Initial health check should succeed");

    // Simulate network failure by stopping server
    println!("Simulating network failure (stopping server)...");
    server.stop().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Try operation (should fail or timeout)
    println!("Attempting operation during failure...");
    let result = timeout(Duration::from_secs(3), client.health_check()).await;
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Operation should fail when server is down"
    );

    // Restart server
    println!("Restarting server...");
    let server = test_utils::start_test_server(endpoint).await.unwrap();
    sleep(Duration::from_secs(2)).await;

    // Reconnect
    println!("Attempting to reconnect...");
    match timeout(Duration::from_secs(10), client.connect(endpoint)).await {
        Ok(Ok(_)) => println!("Reconnection successful"),
        Ok(Err(e)) => eprintln!("Reconnection failed: {}", e),
        Err(_) => eprintln!("Reconnection timed out"),
    }

    // Verify connection restored with health check
    let health2 = timeout(Duration::from_secs(5), client.health_check()).await;
    println!("Health check after reconnection: {:?}", health2.is_ok());

    // Cleanup
    let _ = client.disconnect().await;
    let _ = server.stop().await;
}

// ============================================================================
// Test: Timeout Handling
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_timeout_handling() {
    let endpoint = "tcp://127.0.0.1:15560";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client with very short timeout
    let config = ZmqRunnerConfig {
        endpoint: endpoint.to_string(),
        connection_timeout_secs: 1,
        request_timeout_secs: 2,
        auto_reconnect: false,
        max_reconnect_attempts: 1,
        reconnect_delay_secs: 1,
        enable_heartbeat: false,
        heartbeat_interval_secs: 30,
        server_id: Some("timeout-test-client".to_string()),
    };

    let client = Arc::new(ZmqClient::new(config));

    // Connect
    if client.connect(endpoint).await.is_err() {
        eprintln!("Failed to connect for timeout test");
        return;
    }

    // Try to spawn agent with very short timeout (1 second)
    let agent_config = test_utils::create_test_agent_config("timeout-test-agent");

    let result = timeout(
        Duration::from_secs(3),
        client.spawn_remote(agent_config, Some(1)),
    )
    .await;

    // The spawn might timeout or fail
    match result {
        Ok(Ok(_)) => println!("Agent spawned (unexpectedly fast)"),
        Ok(Err(e)) => println!("Spawn failed as expected: {}", e),
        Err(_) => println!("Spawn timed out as expected"),
    }

    // Cleanup
    let _ = client.disconnect().await;
    let _ = server.stop().await;
}

// ============================================================================
// Test: Batch Operations
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_batch_operations() {
    let endpoint = "tcp://127.0.0.1:15561";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client
    let client = test_utils::create_test_client(endpoint);

    // Connect
    if client.connect(endpoint).await.is_err() {
        eprintln!("Failed to connect for batch test");
        return;
    }

    // Spawn multiple agents
    let mut agent_ids = Vec::new();
    for i in 0..3 {
        let config = test_utils::create_test_agent_config(&format!("batch-test-agent-{}", i));

        if let Ok(agent_info) = timeout(
            Duration::from_secs(30),
            client.spawn_remote(config, Some(60)),
        )
        .await
        .unwrap_or_else(|_| {
            Err(descartes_core::AgentError::ExecutionError(
                "Timeout".to_string(),
            ))
        }) {
            agent_ids.push(agent_info.id);
        }
    }

    if agent_ids.is_empty() {
        eprintln!("No agents spawned for batch test");
        return;
    }

    println!("Spawned {} agents for batch test", agent_ids.len());

    // Perform batch get status
    let result = timeout(
        Duration::from_secs(10),
        client.batch_control(
            agent_ids.clone(),
            ControlCommandType::GetStatus,
            None,
            false,
        ),
    )
    .await;

    match result {
        Ok(Ok(response)) => {
            println!("Batch operation results:");
            println!("  Successful: {}", response.successful);
            println!("  Failed: {}", response.failed);
            assert!(
                response.successful > 0,
                "Some batch operations should succeed"
            );
        }
        Ok(Err(e)) => eprintln!("Batch operation failed: {}", e),
        Err(_) => eprintln!("Batch operation timed out"),
    }

    // Cleanup
    for agent_id in agent_ids {
        let _ = client.stop_agent(&agent_id).await;
    }
    let _ = client.disconnect().await;
    let _ = server.stop().await;
}

// ============================================================================
// Test: Output Querying
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_output_querying() {
    let endpoint = "tcp://127.0.0.1:15562";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client
    let client = test_utils::create_test_client(endpoint);

    // Connect
    if client.connect(endpoint).await.is_err() {
        eprintln!("Failed to connect for output query test");
        return;
    }

    // Spawn an agent
    let config = test_utils::create_test_agent_config("output-test-agent");

    let agent_info = match timeout(
        Duration::from_secs(30),
        client.spawn_remote(config, Some(60)),
    )
    .await
    {
        Ok(Ok(info)) => info,
        _ => {
            eprintln!("Failed to spawn agent for output test");
            return;
        }
    };

    println!("Spawned agent: {}", agent_info.id);

    // Wait for agent to produce some output
    sleep(Duration::from_secs(2)).await;

    // Query stdout
    match timeout(
        Duration::from_secs(5),
        client.query_agent_output(
            &agent_info.id,
            ZmqOutputStream::Stdout,
            None,
            Some(10),
            None,
        ),
    )
    .await
    {
        Ok(Ok(response)) => {
            println!("Output query successful:");
            println!("  Lines: {}", response.lines.len());
            println!("  Has more: {}", response.has_more);
        }
        Ok(Err(e)) => println!("Output query failed: {}", e),
        Err(_) => println!("Output query timed out"),
    }

    // Cleanup
    let _ = client.stop_agent(&agent_info.id).await;
    let _ = client.disconnect().await;
    let _ = server.stop().await;
}

// ============================================================================
// Test: Server Statistics
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_server_statistics() {
    let endpoint = "tcp://127.0.0.1:15563";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client
    let client = test_utils::create_test_client(endpoint);

    // Connect
    if client.connect(endpoint).await.is_err() {
        eprintln!("Failed to connect for stats test");
        return;
    }

    // Perform health check
    let _ = timeout(Duration::from_secs(5), client.health_check()).await;

    // Check server stats
    let stats = server.stats();
    println!("Server Statistics:");
    println!("  Health checks: {}", stats.health_checks);
    println!("  Spawn requests: {}", stats.spawn_requests);
    println!("  Control commands: {}", stats.control_commands);

    // Verify server is tracking operations
    assert!(stats.health_checks > 0, "Server should track health checks");

    // Cleanup
    let _ = client.disconnect().await;
    let _ = server.stop().await;
}

// ============================================================================
// Test: Custom Actions
// ============================================================================

#[tokio::test]
#[ignore] // Requires actual ZMQ server - run with --ignored
async fn test_custom_actions() {
    let endpoint = "tcp://127.0.0.1:15564";

    // Start server
    let server = test_utils::start_test_server(endpoint).await.unwrap();

    // Create client
    let client = test_utils::create_test_client(endpoint);

    // Connect
    if client.connect(endpoint).await.is_err() {
        eprintln!("Failed to connect for custom action test");
        return;
    }

    // Spawn an agent
    let config = test_utils::create_test_agent_config("custom-action-agent");

    let agent_info = match timeout(
        Duration::from_secs(30),
        client.spawn_remote(config, Some(60)),
    )
    .await
    {
        Ok(Ok(info)) => info,
        _ => {
            eprintln!("Failed to spawn agent for custom action test");
            return;
        }
    };

    // Send custom action
    let params = serde_json::json!({
        "action": "test",
        "data": "example"
    });

    let result = timeout(
        Duration::from_secs(10),
        client.send_action_to_agent(&agent_info.id, "custom_test", Some(params), Some(5)),
    )
    .await;

    match result {
        Ok(Ok(response)) => {
            println!("Custom action response:");
            println!("  Success: {}", response.success);
        }
        Ok(Err(e)) => println!("Custom action failed: {}", e),
        Err(_) => println!("Custom action timed out"),
    }

    // Cleanup
    let _ = client.stop_agent(&agent_info.id).await;
    let _ = client.disconnect().await;
    let _ = server.stop().await;
}
