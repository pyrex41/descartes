//! Example: Unix Socket RPC Client
//!
//! This example demonstrates how to connect to the jsonrpsee-based RPC server
//! via Unix socket and make RPC calls.
//!
//! Prerequisites:
//!   Start the server first: cargo run --example unix_socket_server
//!
//! Usage:
//!   cargo run --example unix_socket_client

// Note: jsonrpsee 0.21 doesn't expose client_transport module with current features
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::ws_client::WsClientBuilder;
use std::path::PathBuf;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Unix Socket RPC Client Example");

    // Connect to the Unix socket
    // Note: jsonrpsee doesn't directly support Unix sockets for clients,
    // so this is a simplified example showing the general pattern.
    // In practice, you might need to use a different client library
    // or implement a custom transport layer.

    let socket_path = PathBuf::from("/tmp/descartes-rpc.sock");

    info!("Connecting to Unix socket: {:?}", socket_path);

    // This is a placeholder - actual Unix socket client implementation
    // would require custom transport or using a library that supports it
    info!("Note: This is a demonstration of the client pattern.");
    info!("For Unix socket clients, you would typically use:");
    info!("  - A custom jsonrpsee transport implementation");
    info!("  - Or a library like `tokio::net::UnixStream` with manual RPC handling");
    info!("");
    info!("Example RPC calls that would be made:");
    info!("");

    // Example 1: Spawn an agent
    info!("1. Spawning an agent:");
    info!("   Method: spawn");
    info!("   Params: {{");
    info!("     name: 'test-agent',");
    info!("     agent_type: 'worker',");
    info!("     config: {{}}");
    info!("   }}");
    info!("");

    // Example 2: List tasks
    info!("2. Listing tasks:");
    info!("   Method: list_tasks");
    info!("   Params: {{");
    info!("     filter: {{ status: 'pending' }}");
    info!("   }}");
    info!("");

    // Example 3: Approve a task
    info!("3. Approving a task:");
    info!("   Method: approve");
    info!("   Params: {{");
    info!("     task_id: 'task-123',");
    info!("     approved: true");
    info!("   }}");
    info!("");

    // Example 4: Get state
    info!("4. Getting state:");
    info!("   Method: get_state");
    info!("   Params: {{");
    info!("     entity_id: 'agent-456'");
    info!("   }}");
    info!("");

    info!("To implement a real client, see the documentation at:");
    info!("  - jsonrpsee: https://docs.rs/jsonrpsee/");
    info!("  - tokio Unix sockets: https://docs.rs/tokio/latest/tokio/net/struct.UnixStream.html");

    Ok(())
}
