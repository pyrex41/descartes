//! OpenCode harness implementation
//!
//! Runs OpenCode CLI, parsing streaming JSON output.
//! Uses `opencode run --format json` for structured interaction.

use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult, ToolCall, ToolResult,
};
use crate::config::OpenCodeConfig;
use crate::{Error, Result};

/// OpenCode harness using the CLI
pub struct OpenCodeHarness {
    /// Path to opencode binary
    binary: String,
    /// Default model
    model: String,
    /// Active session states
    sessions: Arc<Mutex<HashMap<String, OpenCodeSession>>>,
}

/// State for an active OpenCode session
#[derive(Debug)]
struct OpenCodeSession {
    /// OpenCode session ID (for --session flag)
    session_id: Option<String>,
    /// Conversation history for context
    messages: Vec<ConversationMessage>,
    /// Agent context to prepend to messages
    agent_context: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ConversationMessage {
    User(String),
    Assistant(String),
    ToolResult { id: String, content: String },
}

impl OpenCodeHarness {
    /// Create a new OpenCode harness
    pub fn new(config: &OpenCodeConfig) -> Result<Self> {
        let binary = config
            .binary
            .clone()
            .unwrap_or_else(|| "opencode".to_string());

        Ok(Self {
            binary,
            model: config.model.clone(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Build command arguments for a session
    fn build_args(&self, session: &SessionHandle, message: &str, session_state: &OpenCodeSession) -> Vec<String> {
        let mut args = vec!["run".to_string()];

        // Output format
        args.push("--format".to_string());
        args.push("json".to_string());

        // Model
        args.push("--model".to_string());
        args.push(session.model.clone());

        // Session ID for continuity
        if let Some(sid) = &session_state.session_id {
            args.push("--session".to_string());
            args.push(sid.clone());
        }

        // Prepend agent context to message if provided
        let full_message = match &session_state.agent_context {
            Some(ctx) => format!("<agent-context>\n{}\n</agent-context>\n\n{}", ctx, message),
            None => message.to_string(),
        };

        // The message/prompt
        args.push(full_message);

        args
    }

    /// Parse a JSON line from OpenCode output
    ///
    /// Handles both Anthropic SSE format and legacy formats:
    /// - Anthropic SSE: `content_block_delta`, `message_stop`, etc.
    /// - Legacy: `text`, `tool_use`, `done`, etc.
    fn parse_output_line(&self, line: &str) -> Option<ResponseChunk> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Handle SSE "data: " prefix if present
        let json_str = line.strip_prefix("data: ").unwrap_or(line);
        if json_str == "[DONE]" {
            return Some(ResponseChunk::Done);
        }

        let json: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                debug!("Failed to parse JSON line: {} - {}", e, line);
                return None;
            }
        };

        let msg_type = json.get("type").and_then(|t| t.as_str())?;

        match msg_type {
            // ============ Anthropic SSE Format ============

            // Content block start - may contain initial text or tool_use block
            "content_block_start" => {
                if let Some(block) = json.get("content_block") {
                    let block_type = block.get("type").and_then(|t| t.as_str());
                    match block_type {
                        Some("text") => {
                            // Initial text (usually empty in streaming)
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                if !text.is_empty() {
                                    return Some(ResponseChunk::Text(text.to_string()));
                                }
                            }
                        }
                        Some("tool_use") => {
                            // Tool use block with full info
                            if let (Some(id), Some(name)) = (
                                block.get("id").and_then(|i| i.as_str()),
                                block.get("name").and_then(|n| n.as_str()),
                            ) {
                                // Input comes in deltas, but we can emit the tool call start
                                // For now, return None and wait for input deltas
                                debug!("Tool use block started: {} ({})", name, id);
                            }
                        }
                        _ => {}
                    }
                }
                None
            }

            // Content block delta - streaming text or tool input
            "content_block_delta" => {
                if let Some(delta) = json.get("delta") {
                    let delta_type = delta.get("type").and_then(|t| t.as_str());
                    match delta_type {
                        Some("text_delta") => {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                return Some(ResponseChunk::Text(text.to_string()));
                            }
                        }
                        Some("input_json_delta") => {
                            // Tool input accumulation - logged for debugging
                            if let Some(partial) = delta.get("partial_json").and_then(|p| p.as_str()) {
                                debug!("Tool input delta: {}", partial);
                            }
                        }
                        _ => {}
                    }
                }
                None
            }

            // Content block finished
            "content_block_stop" => {
                // Could emit accumulated tool call here in the future
                None
            }

            // Message complete
            "message_stop" => Some(ResponseChunk::Done),

            // Message start (metadata)
            "message_start" => {
                debug!("Message start: {:?}", json);
                None
            }

            // Message delta (stop reason, usage)
            "message_delta" => {
                if let Some(delta) = json.get("delta") {
                    if delta.get("stop_reason").is_some() {
                        debug!("Stop reason: {:?}", delta.get("stop_reason"));
                    }
                }
                None
            }

            // ============ Legacy/Direct Formats ============

            // Direct text content
            "text" | "assistant" | "content" => {
                json.get("text")
                    .or_else(|| json.get("content"))
                    .and_then(|t| t.as_str())
                    .map(|s| ResponseChunk::Text(s.to_string()))
            }

            // Text delta (streaming, non-Anthropic format)
            "delta" | "content_delta" => {
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    return Some(ResponseChunk::Text(text.to_string()));
                }
                if let Some(delta) = json.get("delta") {
                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                        return Some(ResponseChunk::Text(text.to_string()));
                    }
                }
                None
            }

            // Tool use (direct format)
            "tool_use" | "tool_call" => {
                let name = json.get("name").and_then(|n| n.as_str())?;
                let id = json.get("id").and_then(|i| i.as_str())?;
                let args = json
                    .get("input")
                    .or_else(|| json.get("arguments"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);

                if self.is_subagent_tool(name) {
                    if let Some(req) = self.extract_subagent_request(name, &args) {
                        return Some(ResponseChunk::SubagentSpawn(req));
                    }
                }

                Some(ResponseChunk::ToolCall(ToolCall {
                    name: name.to_string(),
                    arguments: args,
                    id: id.to_string(),
                }))
            }

            // Tool result
            "tool_result" => {
                let id = json.get("tool_use_id").and_then(|i| i.as_str())?;
                let content = json
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let success = !json
                    .get("is_error")
                    .and_then(|e| e.as_bool())
                    .unwrap_or(false);

                Some(ResponseChunk::ToolResult(ToolResult {
                    tool_call_id: id.to_string(),
                    content,
                    success,
                }))
            }

            // Message complete (various formats)
            "done" | "complete" | "end" => Some(ResponseChunk::Done),

            // Error
            "error" => {
                let msg = json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .or_else(|| json.get("message").and_then(|m| m.as_str()))
                    .unwrap_or("Unknown error");
                Some(ResponseChunk::Error(msg.to_string()))
            }

            // Result message (final output)
            "result" => {
                if let Some(text) = json.get("result").and_then(|r| r.as_str()) {
                    return Some(ResponseChunk::Text(text.to_string()));
                }
                Some(ResponseChunk::Done)
            }

            _ => {
                debug!("Unknown message type: {} - {:?}", msg_type, json);
                None
            }
        }
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

        // Extract optional agent_name
        let agent_name = args
            .get("agent_name")
            .or_else(|| args.get("agent"))
            .and_then(|a| a.as_str())
            .map(|s| s.to_string());

        Some(SubagentRequest {
            category,
            prompt: prompt.to_string(),
            model,
            agent_name,
        })
    }

    /// Execute opencode CLI and return output stream
    async fn execute_opencode(
        &self,
        args: Vec<String>,
    ) -> Result<(
        Child,
        tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    )> {
        debug!("Running: {} {:?}", self.binary, args);

        let mut child = Command::new(&self.binary)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Harness(format!("Failed to spawn opencode: {}", e)))?;

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
impl Harness for OpenCodeHarness {
    fn name(&self) -> &str {
        "opencode"
    }

    fn kind(&self) -> HarnessKind {
        HarnessKind::OpenCode
    }

    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle> {
        let session_id = Uuid::new_v4().to_string();

        let model = if config.model.is_empty() {
            self.model.clone()
        } else {
            config.model
        };

        info!(
            "Starting OpenCode session {} with model {}",
            session_id, model
        );

        // Initialize session state
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            session_id.clone(),
            OpenCodeSession {
                session_id: None, // Will be set after first interaction
                messages: Vec::new(),
                agent_context: config.agent_context,
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
        // Get session state
        let sessions = self.sessions.lock().await;
        let session_state = sessions
            .get(&session.id)
            .ok_or_else(|| Error::Harness("Session not found".to_string()))?;

        let args = self.build_args(session, message, session_state);
        drop(sessions);

        let (mut child, mut lines) = self.execute_opencode(args).await?;

        // Collect chunks while streaming
        let mut chunks = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(chunk) = self.parse_output_line(&line) {
                // Record assistant messages in session
                if let ResponseChunk::Text(ref text) = chunk {
                    let mut sessions = self.sessions.lock().await;
                    if let Some(state) = sessions.get_mut(&session.id) {
                        if let Some(ConversationMessage::Assistant(last)) =
                            state.messages.last_mut()
                        {
                            last.push_str(text);
                        } else {
                            state
                                .messages
                                .push(ConversationMessage::Assistant(text.clone()));
                        }
                    }
                }
                chunks.push(chunk);
            }
        }

        // Wait for process to complete
        let status = child.wait().await?;

        // Check if we got any meaningful output
        let has_text = chunks.iter().any(|c| matches!(c, ResponseChunk::Text(_)));
        let has_tool_calls = chunks.iter().any(|c| matches!(c, ResponseChunk::ToolCall(_)));

        if !status.success() {
            // If process failed and we got no useful output, capture stderr and return error
            if !has_text && !has_tool_calls {
                // Read stderr for error message
                let stderr = child.stderr.take();
                let error_msg = if let Some(mut stderr) = stderr {
                    let mut buf = String::new();
                    use tokio::io::AsyncReadExt;
                    let _ = stderr.read_to_string(&mut buf).await;
                    if buf.is_empty() {
                        format!("OpenCode process failed with status: {}", status)
                    } else {
                        format!("OpenCode error: {}", buf.trim())
                    }
                } else {
                    format!("OpenCode process failed with status: {}", status)
                };
                return Err(Error::Harness(error_msg));
            }
            warn!("OpenCode process exited with status: {} but produced output", status);
        }

        // Ensure we have a done marker
        if !chunks.iter().any(|c| matches!(c, ResponseChunk::Done)) {
            chunks.push(ResponseChunk::Done);
        }

        // Record user message in session history
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(state) = sessions.get_mut(&session.id) {
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

    async fn inject_result(&self, session: &SessionHandle, result: SubagentResult) -> Result<()> {
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
        info!("Closing OpenCode session {}", session.id);

        // Clean up session state
        let mut sessions = self.sessions.lock().await;
        sessions.remove(&session.id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_harness() -> OpenCodeHarness {
        OpenCodeHarness {
            binary: "opencode".to_string(),
            model: "anthropic/claude-sonnet".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[test]
    fn test_extract_subagent_request() {
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
    fn test_is_subagent_tool() {
        let harness = create_test_harness();

        assert!(harness.is_subagent_tool("Task"));
        assert!(harness.is_subagent_tool("spawn"));
        assert!(harness.is_subagent_tool("DELEGATE"));
        assert!(!harness.is_subagent_tool("read"));
        assert!(!harness.is_subagent_tool("bash"));
    }

    #[test]
    fn test_parse_text_content() {
        let harness = create_test_harness();

        let line = r#"{"type":"text","text":"Hello, world!"}"#;
        let chunk = harness.parse_output_line(line);

        assert!(matches!(chunk, Some(ResponseChunk::Text(_))));
        if let Some(ResponseChunk::Text(text)) = chunk {
            assert_eq!(text, "Hello, world!");
        }
    }

    #[test]
    fn test_parse_tool_use() {
        let harness = create_test_harness();

        let line = r#"{"type":"tool_use","id":"tool_123","name":"read","input":{"path":"/test.txt"}}"#;
        let chunk = harness.parse_output_line(line);

        assert!(matches!(chunk, Some(ResponseChunk::ToolCall(_))));
        if let Some(ResponseChunk::ToolCall(tool)) = chunk {
            assert_eq!(tool.name, "read");
            assert_eq!(tool.id, "tool_123");
        }
    }

    #[test]
    fn test_parse_done() {
        let harness = create_test_harness();

        let line = r#"{"type":"done"}"#;
        let chunk = harness.parse_output_line(line);

        assert!(matches!(chunk, Some(ResponseChunk::Done)));
    }

    #[test]
    fn test_parse_anthropic_sse_text_delta() {
        let harness = create_test_harness();

        // Text delta (main streaming format)
        let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let chunk = harness.parse_output_line(line);
        assert!(matches!(chunk, Some(ResponseChunk::Text(ref t)) if t == "Hello"));
    }

    #[test]
    fn test_parse_anthropic_sse_message_stop() {
        let harness = create_test_harness();

        let line = r#"{"type":"message_stop"}"#;
        let chunk = harness.parse_output_line(line);
        assert!(matches!(chunk, Some(ResponseChunk::Done)));
    }

    #[test]
    fn test_parse_anthropic_sse_with_data_prefix() {
        let harness = create_test_harness();

        // SSE with data prefix
        let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"World"}}"#;
        let chunk = harness.parse_output_line(line);
        assert!(matches!(chunk, Some(ResponseChunk::Text(ref t)) if t == "World"));
    }

    #[test]
    fn test_parse_sse_done_marker() {
        let harness = create_test_harness();

        let line = "data: [DONE]";
        let chunk = harness.parse_output_line(line);
        assert!(matches!(chunk, Some(ResponseChunk::Done)));
    }

    #[test]
    fn test_parse_anthropic_sse_message_start_ignored() {
        let harness = create_test_harness();

        // Message start should return None (metadata only)
        let line = r#"{"type":"message_start","message":{"id":"msg_123","model":"claude-3"}}"#;
        let chunk = harness.parse_output_line(line);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_parse_anthropic_sse_content_block_stop_ignored() {
        let harness = create_test_harness();

        // Content block stop should return None
        let line = r#"{"type":"content_block_stop","index":0}"#;
        let chunk = harness.parse_output_line(line);
        assert!(chunk.is_none());
    }
}
