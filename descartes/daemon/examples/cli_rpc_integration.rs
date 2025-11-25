//! CLI RPC Integration Example
//!
//! This example demonstrates how a CLI tool (like scud) can integrate with
//! the Descartes RPC server via Unix sockets.
//!
//! Usage:
//!   1. Start the RPC server: cargo run --bin descartes-daemon
//!   2. Run this example: cargo run --example cli_rpc_integration

use descartes_daemon::{DaemonError, UnixSocketRpcClient};
use serde_json::json;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== CLI RPC Integration Example ===\n");

    // Create RPC client (connects to Unix socket)
    let socket_path = PathBuf::from("/tmp/descartes-rpc.sock");
    let client = UnixSocketRpcClient::new(socket_path)?;

    println!("Connecting to RPC server...");

    // Test connection
    match client.test_connection().await {
        Ok(_) => println!("✓ Connected successfully\n"),
        Err(e) => {
            eprintln!("✗ Connection failed: {}", e);
            eprintln!("\nMake sure the RPC server is running:");
            eprintln!("  cargo run --bin descartes-daemon\n");
            return Err(e.into());
        }
    }

    // Example 1: Spawn an agent (like 'scud spawn')
    println!("--- Example 1: Spawn Agent ---");
    let agent_config = json!({
        "task": "Write a hello world program in Rust",
        "environment": {
            "RUST_LOG": "info"
        },
        "system_prompt": "You are an expert Rust programmer."
    });

    match client
        .spawn("hello-world-agent", "rust-dev", agent_config)
        .await
    {
        Ok(agent_id) => {
            println!("✓ Agent spawned successfully");
            println!("  Agent ID: {}\n", agent_id);
        }
        Err(e) => {
            eprintln!("✗ Failed to spawn agent: {}\n", e);
        }
    }

    // Example 2: List tasks (like 'scud ps --tasks')
    println!("--- Example 2: List All Tasks ---");
    match client.list_tasks(None).await {
        Ok(tasks) => {
            println!("✓ Found {} tasks:", tasks.len());
            for task in tasks.iter().take(5) {
                println!("  - {} [{}]: {}", task.id, task.status, task.name);
            }
            if tasks.len() > 5 {
                println!("  ... and {} more", tasks.len() - 5);
            }
            println!();
        }
        Err(e) => {
            eprintln!("✗ Failed to list tasks: {}\n", e);
        }
    }

    // Example 3: Filter tasks by status (like 'scud ps --status todo')
    println!("--- Example 3: Filter Tasks by Status ---");
    let filter = json!({ "status": "todo" });
    match client.list_tasks(Some(filter)).await {
        Ok(tasks) => {
            println!("✓ Found {} TODO tasks:", tasks.len());
            for task in tasks {
                println!("  - {}: {}", task.id, task.name);
            }
            println!();
        }
        Err(e) => {
            eprintln!("✗ Failed to filter tasks: {}\n", e);
        }
    }

    // Example 4: Get system state (like 'scud status')
    println!("--- Example 4: Get System State ---");
    match client.get_state(None).await {
        Ok(state) => {
            println!("✓ System state:");
            println!("  {}", serde_json::to_string_pretty(&state)?);
            println!();
        }
        Err(e) => {
            eprintln!("✗ Failed to get state: {}\n", e);
        }
    }

    // Example 5: Approve a task (like 'scud approve <task-id>')
    // Note: This would require an actual pending task
    println!("--- Example 5: Task Approval ---");
    println!("(Skipped - requires an existing pending task)");
    println!("Usage: client.approve(task_id, true).await");
    println!();

    // Example 6: Error handling
    println!("--- Example 6: Error Handling ---");
    match client.approve("invalid-task-id", true).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => {
            println!("✓ Error handled gracefully:");
            println!("  Error: {}", e);
            match e {
                DaemonError::RpcError(code, msg) => {
                    println!("  RPC Error Code: {}", code);
                    println!("  Message: {}", msg);
                }
                _ => println!("  Other error type"),
            }
            println!();
        }
    }

    println!("=== CLI Integration Examples Complete ===\n");

    Ok(())
}
