/// Error types for the RPC daemon

use serde_json::json;
use std::fmt;
use thiserror::Error;

/// Result type for daemon operations
pub type DaemonResult<T> = Result<T, DaemonError>;

/// Daemon error types
#[derive(Debug, Error)]
pub enum DaemonError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Authentication/authorization error
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// RPC method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid RPC request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Server error
    #[error("Server error: {0}")]
    ServerError(String),

    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Agent spawn error
    #[error("Failed to spawn agent: {0}")]
    SpawnError(String),

    /// Agent kill error
    #[error("Failed to kill agent: {0}")]
    KillError(String),

    /// Workflow execution error
    #[error("Workflow execution error: {0}")]
    WorkflowError(String),

    /// State query error
    #[error("State query error: {0}")]
    StateError(String),

    /// Connection pool error
    #[error("Connection pool error: {0}")]
    PoolError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Metrics error
    #[error("Metrics error: {0}")]
    MetricsError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// RPC error with code and message
    #[error("RPC error {0}: {1}")]
    RpcError(i64, String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl DaemonError {
    /// Convert to JSON-RPC error response
    pub fn to_rpc_error(&self) -> serde_json::Value {
        let (code, message) = match self {
            DaemonError::ConfigError(msg) => (-32600, format!("Invalid configuration: {}", msg)),
            DaemonError::AuthError(msg) => (-32001, format!("Authentication failed: {}", msg)),
            DaemonError::MethodNotFound(method) => (-32601, format!("Method not found: {}", method)),
            DaemonError::InvalidRequest(msg) => (-32700, format!("Parse error: {}", msg)),
            DaemonError::ServerError(msg) => (-32603, format!("Internal server error: {}", msg)),
            DaemonError::AgentNotFound(id) => (-32002, format!("Agent not found: {}", id)),
            DaemonError::SpawnError(msg) => (-32003, format!("Failed to spawn agent: {}", msg)),
            DaemonError::KillError(msg) => (-32004, format!("Failed to kill agent: {}", msg)),
            DaemonError::WorkflowError(msg) => (-32005, format!("Workflow error: {}", msg)),
            DaemonError::StateError(msg) => (-32006, format!("State error: {}", msg)),
            DaemonError::PoolError(msg) => (-32007, format!("Pool error: {}", msg)),
            DaemonError::SerializationError(msg) => (-32700, format!("Serialization error: {}", msg)),
            DaemonError::MetricsError(msg) => (-32008, format!("Metrics error: {}", msg)),
            DaemonError::IoError(e) => (-32603, format!("IO error: {}", e)),
            DaemonError::Timeout => (-32009, "Operation timed out".to_string()),
            DaemonError::ConnectionError(msg) => (-32010, format!("Connection error: {}", msg)),
            DaemonError::RpcError(code, msg) => (*code, msg.clone()),
            DaemonError::Other(msg) => (-32000, msg.clone()),
        };

        json!({
            "code": code,
            "message": message
        })
    }

    /// Get the error code for this error
    pub fn code(&self) -> i64 {
        match self {
            DaemonError::ConfigError(_) => -32600,
            DaemonError::AuthError(_) => -32001,
            DaemonError::MethodNotFound(_) => -32601,
            DaemonError::InvalidRequest(_) => -32700,
            DaemonError::ServerError(_) => -32603,
            DaemonError::AgentNotFound(_) => -32002,
            DaemonError::SpawnError(_) => -32003,
            DaemonError::KillError(_) => -32004,
            DaemonError::WorkflowError(_) => -32005,
            DaemonError::StateError(_) => -32006,
            DaemonError::PoolError(_) => -32007,
            DaemonError::SerializationError(_) => -32700,
            DaemonError::MetricsError(_) => -32008,
            DaemonError::IoError(_) => -32603,
            DaemonError::Timeout => -32009,
            DaemonError::ConnectionError(_) => -32010,
            DaemonError::RpcError(code, _) => *code,
            DaemonError::Other(_) => -32000,
        }
    }
}

impl From<serde_json::error::Error> for DaemonError {
    fn from(e: serde_json::error::Error) -> Self {
        DaemonError::SerializationError(e.to_string())
    }
}

impl From<String> for DaemonError {
    fn from(e: String) -> Self {
        DaemonError::Other(e)
    }
}

impl From<&str> for DaemonError {
    fn from(e: &str) -> Self {
        DaemonError::Other(e.to_string())
    }
}
