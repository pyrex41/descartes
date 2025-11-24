/// Descartes Daemon: JSON-RPC 2.0 Server for Remote Agent Control
/// Provides HTTP and WebSocket interfaces for managing agents, workflows, and state

pub mod agent_monitor; // Phase 3:5.3 - Agent monitoring with RPC integration
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
pub mod rpc_agent_methods; // RPC methods for agent monitoring (Phase 3:5.3)
pub mod rpc_client; // Unix socket RPC client
pub mod server;
pub mod task_event_emitter;
pub mod types;

// Re-export commonly used types
pub use agent_monitor::{AgentMonitor, AgentMonitorConfig, HealthSummary, MonitorStats};
pub use client::{RpcClient, RpcClientBuilder, RpcClientConfig};
pub use config::DaemonConfig;
pub use errors::{DaemonError, DaemonResult};
pub use events::{EventBus, DescartesEvent, EventFilter, AgentEvent, TaskEvent, SystemEvent};
pub use event_client::{EventClient, EventClientBuilder, EventClientConfig, EventClientState};
pub use rpc_server::{UnixSocketRpcServer, DescartesRpc, TaskInfo, ApprovalResult};
pub use rpc_agent_methods::{AgentMonitoringRpc, AgentMonitoringRpcImpl, AgentStatusFilter};
pub use rpc_client::{UnixSocketRpcClient, UnixSocketRpcClientBuilder};
pub use server::RpcServer;
pub use task_event_emitter::{TaskEventEmitter, TaskEventEmitterConfig, TaskChangeEvent, TaskEmitterStatistics};
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
