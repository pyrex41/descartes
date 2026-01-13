//! Harness abstraction for different AI backends
//!
//! Supports:
//! - Claude Code (headless mode)
//! - OpenCode (TUI via IPC)
//! - Codex (API)

mod claude_code;
mod codex;
mod opencode;
mod proxy;

pub use claude_code::ClaudeCodeHarness;
pub use codex::CodexHarness;
pub use opencode::OpenCodeHarness;
pub use proxy::SubagentProxy;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::{Config, Error, Result};

/// Session handle for tracking active sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHandle {
    /// Unique session ID
    pub id: String,
    /// Harness that created this session
    pub harness: String,
    /// Model being used
    pub model: String,
    /// Parent session ID (for subagents)
    pub parent: Option<String>,
}

/// Configuration for creating a new session
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Model to use
    pub model: String,
    /// Tools available to this session
    pub tools: Vec<String>,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Parent session (for subagents)
    pub parent: Option<SessionHandle>,
    /// Whether this is a subagent (prevents nested spawning)
    pub is_subagent: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: "sonnet".to_string(),
            tools: vec![
                "read".to_string(),
                "write".to_string(),
                "edit".to_string(),
                "bash".to_string(),
            ],
            system_prompt: None,
            parent: None,
            is_subagent: false,
        }
    }
}

/// A chunk of response from the harness
#[derive(Debug, Clone)]
pub enum ResponseChunk {
    /// Text content
    Text(String),
    /// Tool call detected
    ToolCall(ToolCall),
    /// Tool result
    ToolResult(ToolResult),
    /// Subagent spawn attempt (may be blocked)
    SubagentSpawn(SubagentRequest),
    /// Stream finished
    Done,
    /// Error occurred
    Error(String),
}

/// A tool call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Tool arguments (JSON)
    pub arguments: serde_json::Value,
    /// Tool call ID for tracking
    pub id: String,
}

/// Result from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool call ID this is responding to
    pub tool_call_id: String,
    /// Result content
    pub content: String,
    /// Whether execution succeeded
    pub success: bool,
}

/// Request to spawn a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentRequest {
    /// Requested category/role
    pub category: String,
    /// Task for the subagent
    pub prompt: String,
    /// Requested model (optional)
    pub model: Option<String>,
}

/// Result from a subagent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    /// Session ID of the subagent
    pub session_id: String,
    /// Final output/summary
    pub output: String,
    /// Whether the subagent succeeded
    pub success: bool,
    /// Metrics about the execution
    pub metrics: SubagentMetrics,
}

impl SubagentResult {
    /// Create a blocked result (for nested spawn attempts)
    pub fn blocked(reason: &str) -> Self {
        Self {
            session_id: String::new(),
            output: format!("Blocked: {}", reason),
            success: false,
            metrics: SubagentMetrics::default(),
        }
    }

    /// Get a summary of the result
    pub fn summary(&self) -> String {
        if self.success {
            format!(
                "Session {}: completed in {}ms, {} tokens",
                self.session_id, self.metrics.duration_ms, self.metrics.tokens_total
            )
        } else {
            format!("Session {}: failed - {}", self.session_id, self.output)
        }
    }
}

/// Metrics from subagent execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubagentMetrics {
    /// Tokens sent to model
    pub tokens_in: usize,
    /// Tokens received from model
    pub tokens_out: usize,
    /// Total tokens
    pub tokens_total: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of tool calls made
    pub tools_called: usize,
}

/// Response stream from a harness
pub type ResponseStream = Pin<Box<dyn futures::Stream<Item = ResponseChunk> + Send>>;

/// Harness kind enum for config
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessKind {
    ClaudeCode,
    OpenCode,
    Codex,
}

impl std::fmt::Display for HarnessKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessKind::ClaudeCode => write!(f, "claude-code"),
            HarnessKind::OpenCode => write!(f, "opencode"),
            HarnessKind::Codex => write!(f, "codex"),
        }
    }
}

impl std::str::FromStr for HarnessKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude" => Ok(HarnessKind::ClaudeCode),
            "opencode" | "open-code" => Ok(HarnessKind::OpenCode),
            "codex" => Ok(HarnessKind::Codex),
            _ => Err(Error::Config(format!("Unknown harness kind: {}", s))),
        }
    }
}

/// Core harness trait
#[async_trait]
pub trait Harness: Send + Sync {
    /// Name of this harness
    fn name(&self) -> &str;

    /// What kind of harness this is
    fn kind(&self) -> HarnessKind;

    /// Start a new session
    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle>;

    /// Send a message and get streaming response
    async fn send(&self, session: &SessionHandle, message: &str) -> Result<ResponseStream>;

    /// Detect if a response chunk contains a subagent spawn request
    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest>;

    /// Inject a subagent result back into the parent session
    async fn inject_result(
        &self,
        session: &SessionHandle,
        result: SubagentResult,
    ) -> Result<()>;

    /// Close a session
    async fn close_session(&self, session: &SessionHandle) -> Result<()>;
}

/// Create a harness based on configuration
pub fn create_harness(config: &Config) -> Result<Box<dyn Harness>> {
    let kind: HarnessKind = config.harness.kind.parse()?;

    match kind {
        HarnessKind::ClaudeCode => {
            Ok(Box::new(ClaudeCodeHarness::new(&config.harness.claude_code)?))
        }
        HarnessKind::OpenCode => {
            Ok(Box::new(OpenCodeHarness::new(&config.harness.opencode)?))
        }
        HarnessKind::Codex => {
            Ok(Box::new(CodexHarness::new(&config.harness.codex)?))
        }
    }
}
