//! CLI Backend abstraction for Claude Code, OpenCode, etc.
//!
//! Provides a trait-based interface for interacting with AI CLI tools,
//! enabling streaming output with thinking blocks and session management.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Stream chunk from CLI output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    /// Text content (assistant response)
    Text { content: String },
    /// Thinking/reasoning block
    Thinking { content: String },
    /// Tool use started
    ToolUseStart { tool_name: String, tool_id: String },
    /// Tool use input
    ToolUseInput {
        tool_id: String,
        input: serde_json::Value,
    },
    /// Tool result
    ToolResult {
        tool_id: String,
        result: String,
        is_error: bool,
    },
    /// Sub-agent spawned (detected from Task tool)
    ///
    /// Emitted when Claude Code's Task tool spawns a sub-agent.
    /// The agent_id can be used to:
    /// - Track sub-agent hierarchy in a DAG
    /// - Read agent session from ~/.claude/projects/.../agent-{agent_id}.jsonl
    /// - Monitor sub-agent progress
    SubAgentSpawned {
        /// Short agent ID (e.g., "a9a57a7")
        agent_id: String,
        /// Parent session's UUID (shared between parent and sub-agent)
        session_id: String,
        /// The prompt given to the sub-agent
        prompt: String,
        /// The sub-agent type (e.g., "general-purpose", "Explore")
        subagent_type: Option<String>,
        /// Parent tool_use_id that spawned this agent
        parent_tool_id: String,
    },
    /// Turn complete
    TurnComplete { turn_number: u32 },
    /// Session complete
    Complete { exit_code: i32 },
    /// Error
    Error { message: String },
}

/// Configuration for a chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionConfig {
    /// Working directory for the CLI
    pub working_dir: String,
    /// Initial prompt to send when session starts
    #[serde(default)]
    pub initial_prompt: String,
    /// Enable extended thinking
    pub enable_thinking: bool,
    /// Thinking budget level: "normal", "hard", "harder", "ultra"
    pub thinking_level: String,
    /// Maximum turns (0 = unlimited)
    pub max_turns: u32,
    /// Additional CLI flags
    pub extra_flags: Vec<String>,
}

impl Default for ChatSessionConfig {
    fn default() -> Self {
        Self {
            working_dir: ".".to_string(),
            initial_prompt: String::new(),
            enable_thinking: true,
            thinking_level: "normal".to_string(),
            max_turns: 0,
            extra_flags: vec![],
        }
    }
}

/// Result of starting a chat session
#[derive(Debug)]
pub struct ChatSessionHandle {
    pub session_id: Uuid,
    pub stream_rx: mpsc::UnboundedReceiver<StreamChunk>,
}

/// Trait for CLI backends (Claude Code, OpenCode, etc.)
#[async_trait]
pub trait CliBackend: Send + Sync {
    /// Get the backend name (e.g., "claude", "opencode")
    fn name(&self) -> &str;

    /// Check if the CLI is available on the system
    async fn is_available(&self) -> bool;

    /// Get the CLI version
    async fn version(&self) -> Result<String, String>;

    /// Start a new chat session with the given config
    /// Returns a handle with the session ID and stream receiver
    async fn start_session(&self, config: ChatSessionConfig)
        -> Result<ChatSessionHandle, String>;

    /// Send a prompt to an existing session
    async fn send_prompt(&self, session_id: Uuid, prompt: String) -> Result<(), String>;

    /// Stop a session gracefully
    async fn stop_session(&self, session_id: Uuid) -> Result<(), String>;

    /// Kill a session forcefully
    async fn kill_session(&self, session_id: Uuid) -> Result<(), String>;

    /// Check if a session is active
    fn is_session_active(&self, session_id: Uuid) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_chunk_serialization() {
        let chunk = StreamChunk::Text {
            content: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));

        let deserialized: StreamChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(chunk, deserialized);
    }

    #[test]
    fn test_thinking_chunk_serialization() {
        let chunk = StreamChunk::Thinking {
            content: "Let me think about this...".to_string(),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"type\":\"thinking\""));

        let deserialized: StreamChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(chunk, deserialized);
    }

    #[test]
    fn test_chat_session_config_default() {
        let config = ChatSessionConfig::default();
        assert_eq!(config.working_dir, ".");
        assert!(config.enable_thinking);
        assert_eq!(config.thinking_level, "normal");
        assert_eq!(config.max_turns, 0);
        assert!(config.extra_flags.is_empty());
    }
}
