/// Descartes Daemon: JSON-RPC 2.0 Server for Remote Agent Control
/// Provides HTTP and WebSocket interfaces for managing agents, workflows, and state
pub mod agent_monitor; // Phase 3:5.3 - Agent monitoring with RPC integration
pub mod attach_session; // Attach session management for paused agents
pub mod auth;
pub mod claude_code_tui; // Claude Code TUI attachment handler (Phase 4)
pub mod client;
pub mod config;
pub mod opencode_tui; // OpenCode TUI attachment handler (Phase 5)
pub mod errors;
pub mod event_client;
pub mod event_stream;
pub mod events;
pub mod handlers;
pub mod metrics;
pub mod openapi;
pub mod pool;
pub mod rpc;
pub mod rpc_agent_methods; // RPC methods for agent monitoring (Phase 3:5.3)
pub mod rpc_client; // Unix socket RPC client
pub mod rpc_server; // New jsonrpsee-based Unix socket server
pub mod server;
pub mod task_event_emitter;
pub mod scg_task_event_emitter; // SCG file-based task event emitter
pub mod types;

// Re-export commonly used types
pub use agent_monitor::{AgentMonitor, AgentMonitorConfig, HealthSummary, MonitorStats};
pub use attach_session::{
    AttachCredentials, AttachSession, AttachSessionConfig, AttachSessionInfo, AttachSessionManager,
    ClientType,
};
pub use claude_code_tui::{
    start_attach_server, ClaudeCodeTuiConfig, ClaudeCodeTuiHandler, OutputBuffer,
};
pub use client::{RpcClient, RpcClientBuilder, RpcClientConfig};
pub use opencode_tui::{start_opencode_attach_server, OpenCodeTuiConfig, OpenCodeTuiHandler};
pub use config::DaemonConfig;
pub use errors::{DaemonError, DaemonResult};
pub use event_client::{EventClient, EventClientBuilder, EventClientConfig, EventClientState};
pub use events::{
    AgentEvent, DescartesEvent, EventBus, EventFilter, SystemEvent, TaskEvent, TaskEventType,
};
pub use rpc_agent_methods::{AgentMonitoringRpcImpl, AgentMonitoringRpcServer, AgentStatusFilter};
pub use rpc_client::{UnixSocketRpcClient, UnixSocketRpcClientBuilder};
pub use rpc_server::{
    ApprovalResult, DescartesRpcServer, TaskInfo, UnixServerHandle, UnixSocketRpcServer,
};
pub use server::RpcServer;
pub use task_event_emitter::{
    TaskChangeEvent, TaskEmitterStatistics, TaskEventEmitter, TaskEventEmitterConfig,
};
pub use scg_task_event_emitter::{
    ScgTaskEventEmitter, ScgTaskEventEmitterConfig,
};
pub use types::{RpcRequest, RpcResponse};

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
