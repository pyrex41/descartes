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
}

#[derive(Debug, Clone)]
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

        // The message/prompt
        args.push(message.to_string());

        args
    }

    /// Parse a JSON line from OpenCode output
    fn parse_output_line(&self, line: &str) -> Option<ResponseChunk> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

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
                // Text content
                "assistant" | "content" | "text" => {
                    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                        return Some(ResponseChunk::Text(text.to_string()));
                    }
                    if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                        return Some(ResponseChunk::Text(content.to_string()));
                    }
                }

                // Text delta (streaming)
                "delta" | "content_delta" => {
                    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                        return Some(ResponseChunk::Text(text.to_string()));
                    }
                    if let Some(delta) = json.get("delta") {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            return Some(ResponseChunk::Text(text.to_string()));
                        }
                    }
                }

                // Tool use
                "tool_use" | "tool_call" => {
                    if let (Some(name), Some(id)) = (
                        json.get("name").and_then(|n| n.as_str()),
                        json.get("id").and_then(|i| i.as_str()),
                    ) {
                        let args = json
                            .get("input")
                            .or_else(|| json.get("arguments"))
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);

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

                // Tool result
                "tool_result" => {
                    if let Some(id) = json.get("tool_use_id").and_then(|i| i.as_str()) {
                        let content = json
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        let success = !json
                            .get("is_error")
                            .and_then(|e| e.as_bool())
                            .unwrap_or(false);

                        return Some(ResponseChunk::ToolResult(ToolResult {
                            tool_call_id: id.to_string(),
                            content,
                            success,
                        }));
                    }
                }

                // Message complete
                "done" | "complete" | "end" | "message_stop" => {
                    return Some(ResponseChunk::Done);
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
}
