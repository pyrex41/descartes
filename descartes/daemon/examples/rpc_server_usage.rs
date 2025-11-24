/// Example demonstrating how to use the Unix Socket RPC Server
///
/// This example shows how to:
/// 1. Initialize the agent runner and state store
/// 2. Create and start the RPC server
/// 3. Connect to it and make RPC calls
///
/// Run with: cargo run --example rpc_server_usage

use descartes_core::agent_runner::LocalProcessRunner;
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::StateStore;
use descartes_daemon::UnixSocketRpcServer;
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Descartes RPC Server Example ===\n");

    // Step 1: Create the agent runner
    println!("1. Creating agent runner...");
    let agent_runner = Arc::new(LocalProcessRunner::new())
        as Arc<dyn descartes_core::traits::AgentRunner>;
    println!("   ✓ Agent runner created\n");

    // Step 2: Create and initialize the state store
    println!("2. Creating state store...");
    let db_path = PathBuf::from("/tmp/descartes_rpc_example.db");
    let mut state_store = SqliteStateStore::new(&db_path, false).await?;
    state_store.initialize().await?;
    let state_store = Arc::new(state_store) as Arc<dyn descartes_core::traits::StateStore>;
    println!("   ✓ State store initialized at: {:?}\n", db_path);

    // Step 3: Create the RPC server
    println!("3. Creating RPC server...");
    let socket_path = PathBuf::from("/tmp/descartes_rpc.sock");
    let server = UnixSocketRpcServer::new(
        socket_path.clone(),
        agent_runner,
        state_store,
    );
    println!("   ✓ RPC server created\n");

    // Step 4: Start the server
    println!("4. Starting RPC server on Unix socket: {:?}", socket_path);
    let handle = server.start().await?;
    println!("   ✓ Server started successfully!\n");

    println!("=== RPC Methods Available ===");
    println!("• spawn(name, agent_type, config) - Spawn a new agent");
    println!("• list_tasks(filter) - List all tasks with optional filtering");
    println!("• approve(task_id, approved) - Approve or reject a task");
    println!("• get_state(entity_id) - Get system or agent state\n");

    println!("=== Example JSON-RPC Calls ===\n");

    println!("Spawn an agent:");
    println!(r#"{{
  "jsonrpc": "2.0",
  "method": "spawn",
  "params": [
    "my-agent",
    "claude-code-cli",
    {{
      "task": "Write a hello world program",
      "environment": {{}},
      "system_prompt": "You are a helpful coding assistant"
    }}
  ],
  "id": 1
}}"#);
    println!();

    println!("List tasks:");
    println!(r#"{{
  "jsonrpc": "2.0",
  "method": "list_tasks",
  "params": [
    {{ "status": "todo" }}
  ],
  "id": 2
}}"#);
    println!();

    println!("Approve a task:");
    println!(r#"{{
  "jsonrpc": "2.0",
  "method": "approve",
  "params": [
    "550e8400-e29b-41d4-a716-446655440000",
    true
  ],
  "id": 3
}}"#);
    println!();

    println!("Get system state:");
    println!(r#"{{
  "jsonrpc": "2.0",
  "method": "get_state",
  "params": [null],
  "id": 4
}}"#);
    println!();

    println!("Server is running. Press Ctrl+C to stop.");
    println!("Connect using: socat - UNIX-CONNECT:/tmp/descartes_rpc.sock\n");

    // Keep the server running until interrupted
    tokio::signal::ctrl_c().await?;

    println!("\nShutting down server...");
    handle.stop()?;
    println!("Server stopped.");

    Ok(())
}
