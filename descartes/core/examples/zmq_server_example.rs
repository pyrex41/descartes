/// Example demonstrating ZMQ Agent Server usage
///
/// This example shows how to:
/// 1. Start a ZMQ agent server
/// 2. Handle incoming spawn requests
/// 3. Manage agent lifecycle
/// 4. Monitor server statistics
/// 5. Gracefully shutdown
///
/// Run this example with:
/// ```bash
/// cargo run --example zmq_server_example
/// ```
///
/// Then in another terminal, run the client example:
/// ```bash
/// cargo run --example zmq_client_example
/// ```
use descartes_core::{ProcessRunnerConfig, ZmqAgentServer, ZmqServerConfig};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Starting ZMQ Agent Server example...");

    // Configure the server
    let config = ZmqServerConfig {
        endpoint: "tcp://0.0.0.0:5555".to_string(),
        server_id: "example-server-01".to_string(),
        max_agents: 10,
        status_update_interval_secs: 10,
        enable_status_updates: true,
        runner_config: ProcessRunnerConfig {
            enable_json_streaming: true,
            enable_health_checks: true,
            health_check_interval_secs: 30,
            max_concurrent_agents: Some(10),
            working_dir: None,
        },
        request_timeout_secs: 30,
    };

    info!("Server configuration:");
    info!("  Endpoint: {}", config.endpoint);
    info!("  Server ID: {}", config.server_id);
    info!("  Max agents: {}", config.max_agents);
    info!(
        "  Status updates: enabled (every {}s)",
        config.status_update_interval_secs
    );

    // Create the server
    let server = Arc::new(ZmqAgentServer::new(config));
    let server_clone = server.clone();

    // Spawn server in background task
    let server_task = tokio::spawn(async move {
        info!("Server starting...");
        match server_clone.start().await {
            Ok(_) => info!("Server stopped normally"),
            Err(e) => error!("Server error: {}", e),
        }
    });

    // Wait a moment for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("Server is now running and accepting connections");
    info!("Listening on tcp://0.0.0.0:5555");
    info!("");
    info!("To test the server, run the client example in another terminal:");
    info!("  cargo run --example zmq_client_example");
    info!("");
    info!("Press Ctrl+C to shutdown...");

    // Spawn statistics monitoring task
    let server_stats = server.clone();
    let stats_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            let stats = server_stats.stats();
            let active = server_stats.active_agent_count();
            let uptime = server_stats.uptime_secs().unwrap_or(0);

            info!("=== Server Statistics ===");
            info!("  Uptime: {}s", uptime);
            info!("  Active agents: {}", active);
            info!("  Spawn requests: {}", stats.spawn_requests);
            info!("  Successful spawns: {}", stats.successful_spawns);
            info!("  Failed spawns: {}", stats.failed_spawns);
            info!("  Control commands: {}", stats.control_commands);
            info!("  Health checks: {}", stats.health_checks);
            info!("  Errors: {}", stats.errors);
            info!("========================");
        }
    });

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal (Ctrl+C)");
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown
    info!("Initiating graceful shutdown...");

    // Stop the server
    if let Err(e) = server.stop().await {
        error!("Error stopping server: {}", e);
    }

    // Print final statistics
    let final_stats = server.stats();
    info!("");
    info!("=== Final Statistics ===");
    info!("  Total spawn requests: {}", final_stats.spawn_requests);
    info!("  Successful spawns: {}", final_stats.successful_spawns);
    info!("  Failed spawns: {}", final_stats.failed_spawns);
    info!("  Control commands: {}", final_stats.control_commands);
    info!("  Health checks: {}", final_stats.health_checks);
    info!("  Total errors: {}", final_stats.errors);
    info!("=======================");

    // Cancel background tasks
    stats_task.abort();
    server_task.abort();

    info!("Server shutdown complete");

    Ok(())
}
