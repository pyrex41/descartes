/// Type definitions for RPC daemon
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<serde_json::Value>,
    #[serde(skip)]
    pub auth_token: Option<String>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcResponse {
    /// Create a successful response
    pub fn success(result: serde_json::Value, id: Option<serde_json::Value>) -> Self {
        RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(code: i64, message: String, id: Option<serde_json::Value>) -> Self {
        RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pid: Option<u32>,
    pub config: serde_json::Value,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Running,
    Paused,
    Stopped,
    Failed,
    Terminated,
}

/// Agent spawn request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnRequest {
    pub name: String,
    pub agent_type: String,
    pub config: serde_json::Value,
}

/// Agent spawn response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnResponse {
    pub agent_id: String,
    pub status: AgentStatus,
    pub message: String,
}

/// Agent list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentInfo>,
    pub count: usize,
}

/// Agent kill request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKillRequest {
    pub agent_id: String,
    #[serde(default)]
    pub force: bool,
}

/// Agent kill response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKillResponse {
    pub agent_id: String,
    pub status: AgentStatus,
    pub message: String,
}

/// Agent logs request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogsRequest {
    pub agent_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

/// Agent logs response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogsResponse {
    pub agent_id: String,
    pub logs: Vec<LogEntry>,
    pub total: usize,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

/// Workflow execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecuteRequest {
    pub workflow_id: String,
    pub agents: Vec<String>,
    pub config: serde_json::Value,
}

/// Workflow execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecuteResponse {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// State query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateQueryRequest {
    pub agent_id: Option<String>,
    pub key: Option<String>,
}

/// State query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateQueryResponse {
    pub state: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Metrics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub agents: MetricsAgents,
    pub system: MetricsSystem,
    pub timestamp: DateTime<Utc>,
}

/// Agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAgents {
    pub total: usize,
    pub running: usize,
    pub paused: usize,
    pub stopped: usize,
    pub failed: usize,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSystem {
    pub uptime_secs: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: usize,
}

/// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub scope: Vec<String>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub timestamp: DateTime<Utc>,
}

/// Connection info
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub client_addr: String,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}
