//! Example: Unix Socket RPC Server
//!
//! This example demonstrates how to start a jsonrpsee-based RPC server
//! listening on a Unix socket for IPC communication.
//!
//! Usage:
//!   cargo run --example unix_socket_server

use descartes_daemon::{DaemonResult, UnixSocketRpcServer};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> DaemonResult<()> {
    // Setup logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Unix Socket RPC Server Example");

    // Create socket path in /tmp for testing
    let socket_path = PathBuf::from("/tmp/descartes-rpc.sock");

    // Create and start the RPC server
    let server = UnixSocketRpcServer::new(socket_path.clone());

    info!("Server configured to listen on: {:?}", socket_path);

    // Start the server
    let handle = server.start().await?;

    info!("Server started successfully!");
    info!(
        "You can now connect to the Unix socket at: {:?}",
        socket_path
    );
    info!("");
    info!("Available RPC methods:");
    info!("  - spawn(name, agent_type, config): Spawn a new agent");
    info!("  - list_tasks(filter): List all tasks");
    info!("  - approve(task_id, approved): Approve a task");
    info!("  - get_state(entity_id): Get current state");
    info!("");
    info!("Press Ctrl+C to stop the server");

    // Wait for server to stop (which happens on Ctrl+C or error)
    handle.stopped().await;

    info!("Server stopped");

    Ok(())
}
