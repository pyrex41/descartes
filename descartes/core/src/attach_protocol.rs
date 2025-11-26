//! Protocol definitions for TUI attachment sessions.
//!
//! This module defines the wire protocol for communication between external TUIs
//! (Claude Code, OpenCode, etc.) and the Descartes daemon when attaching to paused agents.

use serde::{Deserialize, Serialize};

/// Protocol version for attach sessions.
pub const ATTACH_PROTOCOL_VERSION: &str = "1.0";

/// Message types in the attach protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttachMessageType {
    /// Initial handshake from client to server
    Handshake,
    /// Handshake response from server to client
    HandshakeResponse,
    /// Historical output sent after successful handshake
    HistoricalOutput,
    /// Stdin data from client to agent
    Stdin,
    /// Stdout data from agent to client
    Stdout,
    /// Stderr data from agent to client
    Stderr,
    /// Client requesting to read available output
    ReadOutput,
    /// Client disconnecting
    Disconnect,
    /// Ping/keepalive
    Ping,
    /// Pong response to ping
    Pong,
    /// Error message
    Error,
}

/// Wrapper for attach protocol messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachMessage {
    /// Message type
    #[serde(rename = "type")]
    pub msg_type: AttachMessageType,
    /// Message payload (type-specific)
    pub payload: serde_json::Value,
    /// Optional sequence number for request/response correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
}

impl AttachMessage {
    /// Create a new attach message.
    pub fn new(msg_type: AttachMessageType, payload: serde_json::Value) -> Self {
        Self {
            msg_type,
            payload,
            seq: None,
        }
    }

    /// Create a new attach message with sequence number.
    pub fn with_seq(msg_type: AttachMessageType, payload: serde_json::Value, seq: u64) -> Self {
        Self {
            msg_type,
            payload,
            seq: Some(seq),
        }
    }

    /// Create an error message.
    pub fn error(message: &str) -> Self {
        Self::new(
            AttachMessageType::Error,
            serde_json::json!({ "error": message }),
        )
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Initial handshake message from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachHandshake {
    /// Protocol version
    pub version: String,
    /// Authentication token
    pub token: String,
    /// Client type identifier (e.g., "claude-code", "opencode")
    pub client_type: String,
    /// Client version string
    pub client_version: String,
    /// Optional capabilities the client supports
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl AttachHandshake {
    /// Create a new handshake message.
    pub fn new(token: String, client_type: String, client_version: String) -> Self {
        Self {
            version: ATTACH_PROTOCOL_VERSION.to_string(),
            token,
            client_type,
            client_version,
            capabilities: Vec::new(),
        }
    }

    /// Add a capability to the handshake.
    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }

    /// Convert to AttachMessage.
    pub fn to_message(&self) -> AttachMessage {
        AttachMessage::new(
            AttachMessageType::Handshake,
            serde_json::to_value(self).unwrap_or_default(),
        )
    }
}

/// Server response to client handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachHandshakeResponse {
    /// Whether the handshake was successful
    pub success: bool,
    /// Agent ID if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Agent name if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// Agent's current task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_task: Option<String>,
    /// Number of buffered output lines available
    pub buffered_output_lines: usize,
    /// Server capabilities
    #[serde(default)]
    pub server_capabilities: Vec<String>,
    /// Error message if unsuccessful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AttachHandshakeResponse {
    /// Create a successful handshake response.
    pub fn success(
        agent_id: String,
        agent_name: String,
        agent_task: String,
        buffered_output_lines: usize,
    ) -> Self {
        Self {
            success: true,
            agent_id: Some(agent_id),
            agent_name: Some(agent_name),
            agent_task: Some(agent_task),
            buffered_output_lines,
            server_capabilities: vec![
                "stdin".to_string(),
                "stdout".to_string(),
                "stderr".to_string(),
                "history".to_string(),
            ],
            error: None,
        }
    }

    /// Create a failed handshake response.
    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            agent_id: None,
            agent_name: None,
            agent_task: None,
            buffered_output_lines: 0,
            server_capabilities: Vec::new(),
            error: Some(error.to_string()),
        }
    }

    /// Convert to AttachMessage.
    pub fn to_message(&self) -> AttachMessage {
        AttachMessage::new(
            AttachMessageType::HandshakeResponse,
            serde_json::to_value(self).unwrap_or_default(),
        )
    }
}

/// Historical output sent after successful handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalOutput {
    /// Buffered stdout lines (base64 encoded)
    pub stdout: Vec<String>,
    /// Buffered stderr lines (base64 encoded)
    pub stderr: Vec<String>,
    /// Unix timestamp of first buffered line
    pub timestamp_start: i64,
    /// Unix timestamp of last buffered line
    pub timestamp_end: i64,
    /// Total bytes in stdout
    pub stdout_bytes: usize,
    /// Total bytes in stderr
    pub stderr_bytes: usize,
}

impl HistoricalOutput {
    /// Create empty historical output.
    pub fn empty() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            timestamp_start: now,
            timestamp_end: now,
            stdout_bytes: 0,
            stderr_bytes: 0,
        }
    }

    /// Convert to AttachMessage.
    pub fn to_message(&self) -> AttachMessage {
        AttachMessage::new(
            AttachMessageType::HistoricalOutput,
            serde_json::to_value(self).unwrap_or_default(),
        )
    }
}

/// Stdin data message from client to agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdinData {
    /// Base64 encoded stdin data
    pub data: String,
    /// Number of bytes
    pub bytes: usize,
}

impl StdinData {
    /// Create from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
            bytes: data.len(),
        }
    }

    /// Decode to raw bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &self.data)
    }

    /// Convert to AttachMessage.
    pub fn to_message(&self) -> AttachMessage {
        AttachMessage::new(
            AttachMessageType::Stdin,
            serde_json::to_value(self).unwrap_or_default(),
        )
    }
}

/// Stdout/Stderr data message from agent to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData {
    /// Base64 encoded output data
    pub data: String,
    /// Number of bytes
    pub bytes: usize,
    /// Unix timestamp
    pub timestamp: i64,
}

impl OutputData {
    /// Create from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
            bytes: data.len(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Decode to raw bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &self.data)
    }

    /// Convert to AttachMessage for stdout.
    pub fn to_stdout_message(&self) -> AttachMessage {
        AttachMessage::new(
            AttachMessageType::Stdout,
            serde_json::to_value(self).unwrap_or_default(),
        )
    }

    /// Convert to AttachMessage for stderr.
    pub fn to_stderr_message(&self) -> AttachMessage {
        AttachMessage::new(
            AttachMessageType::Stderr,
            serde_json::to_value(self).unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_serialization() {
        let handshake =
            AttachHandshake::new("token123".to_string(), "claude-code".to_string(), "1.0.0".to_string());

        let json = serde_json::to_string(&handshake).unwrap();
        assert!(json.contains("token123"));
        assert!(json.contains("claude-code"));

        let deserialized: AttachHandshake = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token, "token123");
        assert_eq!(deserialized.client_type, "claude-code");
    }

    #[test]
    fn test_handshake_response_success() {
        let response = AttachHandshakeResponse::success(
            "agent-123".to_string(),
            "test-agent".to_string(),
            "test task".to_string(),
            100,
        );

        assert!(response.success);
        assert_eq!(response.agent_id, Some("agent-123".to_string()));
        assert_eq!(response.buffered_output_lines, 100);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_handshake_response_failure() {
        let response = AttachHandshakeResponse::failure("Invalid token");

        assert!(!response.success);
        assert!(response.agent_id.is_none());
        assert_eq!(response.error, Some("Invalid token".to_string()));
    }

    #[test]
    fn test_attach_message_serialization() {
        let msg = AttachMessage::new(
            AttachMessageType::Ping,
            serde_json::json!({}),
        );

        let bytes = msg.to_bytes().unwrap();
        let deserialized = AttachMessage::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.msg_type, AttachMessageType::Ping);
    }

    #[test]
    fn test_stdin_data_encoding() {
        let data = StdinData::from_bytes(b"hello world");
        assert_eq!(data.bytes, 11);

        let decoded = data.to_bytes().unwrap();
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn test_output_data_encoding() {
        let data = OutputData::from_bytes(b"test output");
        assert_eq!(data.bytes, 11);
        assert!(data.timestamp > 0);

        let decoded = data.to_bytes().unwrap();
        assert_eq!(decoded, b"test output");
    }

    #[test]
    fn test_historical_output_empty() {
        let history = HistoricalOutput::empty();
        assert!(history.stdout.is_empty());
        assert!(history.stderr.is_empty());
        assert_eq!(history.stdout_bytes, 0);
        assert_eq!(history.stderr_bytes, 0);
    }

    #[test]
    fn test_message_types() {
        let types = [
            AttachMessageType::Handshake,
            AttachMessageType::HandshakeResponse,
            AttachMessageType::HistoricalOutput,
            AttachMessageType::Stdin,
            AttachMessageType::Stdout,
            AttachMessageType::Stderr,
            AttachMessageType::ReadOutput,
            AttachMessageType::Disconnect,
            AttachMessageType::Ping,
            AttachMessageType::Pong,
            AttachMessageType::Error,
        ];

        for msg_type in &types {
            let json = serde_json::to_string(msg_type).unwrap();
            let deserialized: AttachMessageType = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, msg_type);
        }
    }
}
