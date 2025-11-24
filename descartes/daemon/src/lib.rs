/// Descartes Daemon: JSON-RPC 2.0 Server for Remote Agent Control
/// Provides HTTP and WebSocket interfaces for managing agents, workflows, and state

pub mod auth;
pub mod client;
pub mod config;
pub mod errors;
pub mod events;
pub mod event_stream;
pub mod event_client;
pub mod handlers;
pub mod metrics;
pub mod openapi;
pub mod pool;
pub mod rpc;
pub mod rpc_server; // New jsonrpsee-based Unix socket server
pub mod server;
pub mod types;

// Re-export commonly used types
pub use client::{RpcClient, RpcClientBuilder, RpcClientConfig};
pub use config::DaemonConfig;
pub use errors::{DaemonError, DaemonResult};
pub use events::{EventBus, DescartesEvent, EventFilter, AgentEvent, TaskEvent, SystemEvent};
pub use event_client::{EventClient, EventClientBuilder, EventClientConfig, EventClientState};
pub use rpc_server::{UnixSocketRpcServer, DescartesRpc, TaskInfo, ApprovalResult};
pub use server::RpcServer;
pub use types::{RpcResponse, RpcRequest};

/// Daemon version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
