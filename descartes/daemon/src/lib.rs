/// Descartes Daemon: JSON-RPC 2.0 Server for Remote Agent Control
/// Provides HTTP and WebSocket interfaces for managing agents, workflows, and state

pub mod auth;
pub mod config;
pub mod errors;
pub mod handlers;
pub mod metrics;
pub mod openapi;
pub mod pool;
pub mod rpc;
pub mod server;
pub mod types;

// Re-export commonly used types
pub use config::DaemonConfig;
pub use errors::{DaemonError, DaemonResult};
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
