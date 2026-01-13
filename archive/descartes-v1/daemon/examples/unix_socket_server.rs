//! Example: Unix Socket RPC Server
//!
//! This example demonstrates how to start a jsonrpsee-based RPC server
//! listening on a Unix socket for IPC communication.
//!
//! Usage:
//!   cargo run --example unix_socket_server
//!
//! Note: This example requires mock implementations of AgentRunner and StateStore
//! which are not provided here. See the integration tests for complete examples.

use descartes_daemon::{DaemonResult, UnixSocketRpcServer};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> DaemonResult<()> {
    // Setup logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Unix Socket RPC Server Example");

    // Create socket path in /tmp for testing
    let socket_path = PathBuf::from("/tmp/descartes-rpc.sock");

    info!("Note: This example requires AgentRunner and StateStore implementations.");
    info!("See integration tests for complete working examples.");
    info!("Skipping server creation as it requires 3 parameters:");
    info!("  1. socket_path: PathBuf");
    info!("  2. agent_runner: Arc<dyn AgentRunner>");
    info!("  3. state_store: Arc<dyn StateStore>");

    // Create and start the RPC server (commented out - needs mock implementations)
    // let agent_runner = Arc::new(MockAgentRunner::new());
    // let state_store = Arc::new(MockStateStore::new());
    // let server = UnixSocketRpcServer::new(socket_path.clone(), agent_runner, state_store);

    info!("Server configured to listen on: {:?}", socket_path);

    // Start the server (commented out - needs implementations)
    // let handle = server.start().await?;

    // info!("Server started successfully!");
    // info!(
    //     "You can now connect to the Unix socket at: {:?}",
    //     socket_path
    // );
    info!("");
    info!("Available RPC methods:");
    info!("  - spawn(name, agent_type, config): Spawn a new agent");
    info!("  - list_tasks(filter): List all tasks");
    info!("  - approve(task_id, approved): Approve a task");
    info!("  - get_state(entity_id): Get current state");
    info!("");
    // info!("Press Ctrl+C to stop the server");

    // Wait for server to stop (which happens on Ctrl+C or error)
    // handle.stopped().await;

    // info!("Server stopped");

    Ok(())
}
