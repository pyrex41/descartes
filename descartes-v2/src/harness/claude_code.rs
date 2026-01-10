//! Claude Code harness implementation
//!
//! Runs Claude Code CLI in headless mode, intercepting tool calls
//! and subagent spawns. Uses `--output-format stream-json` for structured output.

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult, ToolCall, ToolResult,
};
use crate::config::ClaudeCodeConfig;
use crate::{Error, Result};

/// Claude Code harness using the CLI in headless/print mode
pub struct ClaudeCodeHarness {
    /// Path to claude binary
    binary: String,
    /// Default model
    model: String,
    /// Skip permission prompts
    skip_permissions: bool,
    /// Active session processes (for stateful sessions if needed)
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
}

/// State for an active Claude Code session
#[derive(Debug)]
struct SessionState {
    /// Conversation history for context
    messages: Vec<ConversationMessage>,
    /// Working directory
    working_dir: Option<String>,
}

#[derive(Debug, Clone)]
enum ConversationMessage {
    User(String),
    Assistant(String),
    ToolResult { id: String, content: String },
}

impl ClaudeCodeHarness {
    /// Create a new Claude Code harness
    pub fn new(config: &ClaudeCodeConfig) -> Result<Self> {
        let binary = config
            .binary
            .clone()
            .unwrap_or_else(|| "claude".to_string());

        Ok(Self {
            binary,
            model: config.model.clone(),
            skip_permissions: config.dangerously_skip_permissions,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Build command arguments for a session
    fn build_args(&self, session: &SessionHandle, message: &str, resume: bool) -> Vec<String> {
        let mut args = vec![];

        // Print mode for streaming JSON output
        args.push("-p".to_string());
        args.push(message.to_string());

        // Output format
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());

        // Model
        args.push("--model".to_string());
        args.push(session.model.clone());

        // Permissions
        if self.skip_permissions {
            args.push("--dangerously-skip-permissions".to_string());
        }

        // Resume if we have session context
        if resume {
            args.push("--resume".to_string());
            args.push(session.id.clone());
        }

        args
    }

    /// Parse a JSON line from Claude Code stream-json output
    fn parse_output_line(&self, line: &str) -> Option<ResponseChunk> {
        // Skip empty lines
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Claude Code stream-json format outputs JSON objects, one per line
        let json: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                debug!("Failed to parse JSON line: {} - {}", e, line);
                return None;
            }
        };

        // Check for different message types
        if let Some(msg_type) = json.get("type").and_then(|t| t.as_str()) {
            match msg_type {
                // Text content from assistant
                "assistant" | "content_block_start" => {
                    // Look for text in content blocks
                    if let Some(content) = json.get("content_block") {
                        if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                            return Some(ResponseChunk::Text(text.to_string()));
                        }
                    }
                    // Or direct text content
                    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                        return Some(ResponseChunk::Text(text.to_string()));
                    }
                }

                // Text delta (streaming text)
                "content_block_delta" => {
                    if let Some(delta) = json.get("delta") {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            return Some(ResponseChunk::Text(text.to_string()));
                        }
                    }
                }

                // Tool use
                "tool_use" => {
                    if let (Some(name), Some(id)) = (
                        json.get("name").and_then(|n| n.as_str()),
                        json.get("id").and_then(|i| i.as_str()),
                    ) {
                        let args = json.get("input").cloned().unwrap_or(serde_json::Value::Null);

                        // Check for subagent spawn patterns
                        if self.is_subagent_tool(name) {
                            if let Some(req) = self.extract_subagent_request(name, &args) {
                                return Some(ResponseChunk::SubagentSpawn(req));
                            }
                        }

                        return Some(ResponseChunk::ToolCall(ToolCall {
                            name: name.to_string(),
                            arguments: args,
                            id: id.to_string(),
                        }));
                    }
                }

                // Tool result from Claude Code's tool execution
                "tool_result" => {
                    if let Some(id) = json.get("tool_use_id").and_then(|i| i.as_str()) {
                        let content = json
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        let success = !json.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);

                        return Some(ResponseChunk::ToolResult(ToolResult {
                            tool_call_id: id.to_string(),
                            content,
                            success,
                        }));
                    }
                }

                // Message complete
                "message_stop" | "message_delta" => {
                    if json.get("stop_reason").is_some() {
                        return Some(ResponseChunk::Done);
                    }
                }

                // Error
                "error" => {
                    let msg = json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .or_else(|| json.get("message").and_then(|m| m.as_str()))
                        .unwrap_or("Unknown error");
                    return Some(ResponseChunk::Error(msg.to_string()));
                }

                // Result message (final output)
                "result" => {
                    // Extract final text if present
                    if let Some(text) = json.get("result").and_then(|r| r.as_str()) {
                        return Some(ResponseChunk::Text(text.to_string()));
                    }
                    return Some(ResponseChunk::Done);
                }

                _ => {
                    debug!("Unknown message type: {} - {:?}", msg_type, json);
                }
            }
        }

        None
    }

    /// Check if a tool call is a subagent spawn
    fn is_subagent_tool(&self, name: &str) -> bool {
        matches!(
            name.to_lowercase().as_str(),
            "task" | "spawn" | "subagent" | "agent" | "dispatch" | "delegate"
        )
    }

    /// Extract subagent request from tool call arguments
    fn extract_subagent_request(
        &self,
        _name: &str,
        args: &serde_json::Value,
    ) -> Option<SubagentRequest> {
        // Look for common patterns in subagent spawn calls
        let prompt = args
            .get("prompt")
            .or_else(|| args.get("task"))
            .or_else(|| args.get("message"))
            .or_else(|| args.get("description"))
            .and_then(|p| p.as_str())?;

        let category = args
            .get("category")
            .or_else(|| args.get("type"))
            .or_else(|| args.get("subagent_type"))
            .or_else(|| args.get("agent_type"))
            .and_then(|c| c.as_str())
            .unwrap_or("searcher")
            .to_string();

        let model = args
            .get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        Some(SubagentRequest {
            category,
            prompt: prompt.to_string(),
            model,
        })
    }

    /// Execute claude CLI and return output stream
    async fn execute_claude(
        &self,
        args: Vec<String>,
    ) -> Result<(Child, tokio::io::Lines<BufReader<tokio::process::ChildStdout>>)> {
        debug!("Running: {} {:?}", self.binary, args);

        let mut child = Command::new(&self.binary)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Harness(format!("Failed to spawn claude: {}", e)))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Harness("Failed to capture stdout".to_string()))?;

        let reader = BufReader::new(stdout);
        let lines = reader.lines();

        Ok((child, lines))
    }
}

#[async_trait]
impl Harness for ClaudeCodeHarness {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn kind(&self) -> HarnessKind {
        HarnessKind::ClaudeCode
    }

    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle> {
        let session_id = Uuid::new_v4().to_string();

        let model = if config.model.is_empty() {
            self.model.clone()
        } else {
            config.model
        };

        info!(
            "Starting Claude Code session {} with model {}",
            session_id, model
        );

        // Initialize session state
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            session_id.clone(),
            SessionState {
                messages: Vec::new(),
                working_dir: None,
            },
        );

        Ok(SessionHandle {
            id: session_id,
            harness: self.name().to_string(),
            model,
            parent: config.parent.map(|p| p.id),
        })
    }

    async fn send(&self, session: &SessionHandle, message: &str) -> Result<ResponseStream> {
        // Check if we have prior context for this session
        let sessions = self.sessions.lock().await;
        let has_context = sessions.get(&session.id).map(|s| !s.messages.is_empty()).unwrap_or(false);
        drop(sessions);

        let args = self.build_args(session, message, has_context);
        let (mut child, mut lines) = self.execute_claude(args).await?;

        // Collect chunks while streaming (could be improved to true async stream)
        let mut chunks = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(chunk) = self.parse_output_line(&line) {
                // Record assistant messages in session
                if let ResponseChunk::Text(ref text) = chunk {
                    let mut sessions = self.sessions.lock().await;
                    if let Some(state) = sessions.get_mut(&session.id) {
                        // Append to last assistant message or create new one
                        if let Some(ConversationMessage::Assistant(last)) = state.messages.last_mut() {
                            last.push_str(text);
                        } else {
                            state.messages.push(ConversationMessage::Assistant(text.clone()));
                        }
                    }
                }
                chunks.push(chunk);
            }
        }

        // Wait for process to complete
        let status = child.wait().await?;
        if !status.success() {
            warn!("Claude process exited with status: {}", status);
        }

        // Ensure we have a done marker
        if !chunks.iter().any(|c| matches!(c, ResponseChunk::Done)) {
            chunks.push(ResponseChunk::Done);
        }

        // Record user message in session history
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(state) = sessions.get_mut(&session.id) {
                // Insert user message at the appropriate position
                state.messages.insert(
                    state.messages.len().saturating_sub(1),
                    ConversationMessage::User(message.to_string()),
                );
            }
        }

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest> {
        match chunk {
            ResponseChunk::SubagentSpawn(req) => Some(req.clone()),
            ResponseChunk::ToolCall(tool) if self.is_subagent_tool(&tool.name) => {
                self.extract_subagent_request(&tool.name, &tool.arguments)
            }
            _ => None,
        }
    }

    async fn inject_result(
        &self,
        session: &SessionHandle,
        result: SubagentResult,
    ) -> Result<()> {
        // Record the subagent result in session state
        debug!(
            "Injecting subagent result for session {}: {}",
            result.session_id,
            if result.success { "success" } else { "failed" }
        );

        let mut sessions = self.sessions.lock().await;
        if let Some(state) = sessions.get_mut(&session.id) {
            state.messages.push(ConversationMessage::ToolResult {
                id: result.session_id.clone(),
                content: result.output,
            });
        }

        Ok(())
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        info!("Closing Claude Code session {}", session.id);

        // Clean up session state
        let mut sessions = self.sessions.lock().await;
        sessions.remove(&session.id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_harness() -> ClaudeCodeHarness {
        ClaudeCodeHarness {
            binary: "claude".to_string(),
            model: "sonnet".to_string(),
            skip_permissions: false,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[test]
    fn test_parse_subagent_request() {
        let harness = create_test_harness();

        let args = serde_json::json!({
            "prompt": "search for auth code",
            "category": "searcher",
            "model": "sonnet"
        });

        let req = harness.extract_subagent_request("spawn", &args).unwrap();
        assert_eq!(req.prompt, "search for auth code");
        assert_eq!(req.category, "searcher");
        assert_eq!(req.model, Some("sonnet".to_string()));
    }

    #[test]
    fn test_parse_tool_use() {
        let harness = create_test_harness();

        let line = r#"{"type":"tool_use","id":"toolu_123","name":"read","input":{"path":"/test.txt"}}"#;
        let chunk = harness.parse_output_line(line);

        assert!(matches!(chunk, Some(ResponseChunk::ToolCall(_))));
        if let Some(ResponseChunk::ToolCall(tool)) = chunk {
            assert_eq!(tool.name, "read");
            assert_eq!(tool.id, "toolu_123");
        }
    }

    #[test]
    fn test_parse_content_delta() {
        let harness = create_test_harness();

        let line = r#"{"type":"content_block_delta","delta":{"text":"Hello, world!"}}"#;
        let chunk = harness.parse_output_line(line);

        assert!(matches!(chunk, Some(ResponseChunk::Text(_))));
        if let Some(ResponseChunk::Text(text)) = chunk {
            assert_eq!(text, "Hello, world!");
        }
    }

    #[test]
    fn test_is_subagent_tool() {
        let harness = create_test_harness();

        assert!(harness.is_subagent_tool("Task"));
        assert!(harness.is_subagent_tool("spawn"));
        assert!(harness.is_subagent_tool("DELEGATE"));
        assert!(!harness.is_subagent_tool("read"));
        assert!(!harness.is_subagent_tool("bash"));
    }
}
