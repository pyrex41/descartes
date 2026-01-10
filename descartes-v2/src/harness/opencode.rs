//! OpenCode harness implementation
//!
//! Communicates with OpenCode TUI via IPC (Unix socket).
//! OpenCode exposes a local socket for programmatic interaction.

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult, ToolCall, ToolResult,
};
use crate::config::OpenCodeConfig;
use crate::{Error, Result};

/// OpenCode harness using Unix socket IPC
pub struct OpenCodeHarness {
    /// Socket path for IPC
    socket_path: PathBuf,
    /// Default model
    model: String,
    /// Active sessions
    sessions: Arc<Mutex<HashMap<String, OpenCodeSession>>>,
}

/// State for an OpenCode session
struct OpenCodeSession {
    /// Stream connection to OpenCode
    stream: Option<UnixStream>,
    /// Session conversation history
    history: Vec<String>,
}

/// Message format for OpenCode IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeRequest {
    /// Request ID
    id: String,
    /// Request type
    #[serde(rename = "type")]
    request_type: String,
    /// Session ID
    session_id: Option<String>,
    /// Message content
    message: Option<String>,
    /// Model override
    model: Option<String>,
    /// Additional options
    #[serde(default)]
    options: HashMap<String, serde_json::Value>,
}

/// Response format from OpenCode IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeResponse {
    /// Request ID this responds to
    id: String,
    /// Response type
    #[serde(rename = "type")]
    response_type: String,
    /// Success status
    #[serde(default)]
    success: bool,
    /// Error message if failed
    error: Option<String>,
    /// Response content
    content: Option<OpenCodeContent>,
    /// Session ID
    session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum OpenCodeContent {
    Text(String),
    Structured(StructuredContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StructuredContent {
    /// Text content
    #[serde(default)]
    text: Option<String>,
    /// Tool call
    #[serde(default)]
    tool_call: Option<ToolCallContent>,
    /// Tool result
    #[serde(default)]
    tool_result: Option<ToolResultContent>,
    /// Is complete
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCallContent {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolResultContent {
    tool_call_id: String,
    content: String,
    success: bool,
}

impl OpenCodeHarness {
    /// Create a new OpenCode harness
    pub fn new(config: &OpenCodeConfig) -> Result<Self> {
        let socket_path = config
            .socket_path
            .clone()
            .unwrap_or_else(|| {
                // Default socket locations
                let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                    .unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(runtime_dir).join("opencode.sock")
            });

        let model = config
            .model
            .clone()
            .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

        Ok(Self {
            socket_path,
            model,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Connect to OpenCode socket
    async fn connect(&self) -> Result<UnixStream> {
        if !self.socket_path.exists() {
            return Err(Error::Harness(format!(
                "OpenCode socket not found at {:?}. Is OpenCode running?",
                self.socket_path
            )));
        }

        UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| Error::Harness(format!("Failed to connect to OpenCode: {}", e)))
    }

    /// Send a request and receive response
    async fn send_request(&self, stream: &mut UnixStream, request: &OpenCodeRequest) -> Result<OpenCodeResponse> {
        let mut request_json = serde_json::to_string(request)
            .map_err(|e| Error::Harness(format!("Failed to serialize request: {}", e)))?;
        request_json.push('\n');

        stream
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| Error::Harness(format!("Failed to write to socket: {}", e)))?;

        stream
            .flush()
            .await
            .map_err(|e| Error::Harness(format!("Failed to flush socket: {}", e)))?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| Error::Harness(format!("Failed to read from socket: {}", e)))?;

        serde_json::from_str(&response_line)
            .map_err(|e| Error::Harness(format!("Failed to parse response: {}", e)))
    }

    /// Stream responses from OpenCode
    async fn stream_responses(&self, stream: &mut UnixStream) -> Result<Vec<ResponseChunk>> {
        let mut chunks = Vec::new();
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<OpenCodeResponse>(&line) {
                Ok(response) => {
                    if let Some(chunk) = self.response_to_chunk(&response) {
                        let is_done = matches!(&chunk, ResponseChunk::Done);
                        chunks.push(chunk);
                        if is_done {
                            break;
                        }
                    }

                    if !response.success {
                        if let Some(err) = response.error {
                            chunks.push(ResponseChunk::Error(err));
                            break;
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to parse response line: {} - {}", e, line);
                }
            }
        }

        Ok(chunks)
    }

    /// Convert OpenCode response to ResponseChunk
    fn response_to_chunk(&self, response: &OpenCodeResponse) -> Option<ResponseChunk> {
        match response.response_type.as_str() {
            "text" | "content" => {
                if let Some(OpenCodeContent::Text(text)) = &response.content {
                    return Some(ResponseChunk::Text(text.clone()));
                }
                if let Some(OpenCodeContent::Structured(s)) = &response.content {
                    if let Some(text) = &s.text {
                        return Some(ResponseChunk::Text(text.clone()));
                    }
                }
            }
            "tool_call" => {
                if let Some(OpenCodeContent::Structured(s)) = &response.content {
                    if let Some(tc) = &s.tool_call {
                        // Check for subagent patterns
                        if self.is_subagent_tool(&tc.name) {
                            if let Some(req) = self.extract_subagent_request(&tc.arguments) {
                                return Some(ResponseChunk::SubagentSpawn(req));
                            }
                        }
                        return Some(ResponseChunk::ToolCall(ToolCall {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                            id: tc.id.clone(),
                        }));
                    }
                }
            }
            "tool_result" => {
                if let Some(OpenCodeContent::Structured(s)) = &response.content {
                    if let Some(tr) = &s.tool_result {
                        return Some(ResponseChunk::ToolResult(ToolResult {
                            tool_call_id: tr.tool_call_id.clone(),
                            content: tr.content.clone(),
                            success: tr.success,
                        }));
                    }
                }
            }
            "done" | "complete" | "end" => {
                return Some(ResponseChunk::Done);
            }
            "error" => {
                let msg = response.error.clone().unwrap_or_else(|| "Unknown error".to_string());
                return Some(ResponseChunk::Error(msg));
            }
            _ => {
                debug!("Unknown response type: {}", response.response_type);
            }
        }

        // Check for done in structured content
        if let Some(OpenCodeContent::Structured(s)) = &response.content {
            if s.done {
                return Some(ResponseChunk::Done);
            }
        }

        None
    }

    /// Check if a tool is a subagent spawn
    fn is_subagent_tool(&self, name: &str) -> bool {
        matches!(
            name.to_lowercase().as_str(),
            "task" | "spawn" | "subagent" | "agent" | "dispatch" | "delegate"
        )
    }

    /// Extract subagent request from tool arguments
    fn extract_subagent_request(&self, args: &serde_json::Value) -> Option<SubagentRequest> {
        let prompt = args
            .get("prompt")
            .or_else(|| args.get("task"))
            .or_else(|| args.get("message"))
            .and_then(|p| p.as_str())?;

        let category = args
            .get("category")
            .or_else(|| args.get("type"))
            .or_else(|| args.get("subagent_type"))
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

        // Try to connect and create session
        let stream = match self.connect().await {
            Ok(mut s) => {
                // Send session create request
                let request = OpenCodeRequest {
                    id: Uuid::new_v4().to_string(),
                    request_type: "session.create".to_string(),
                    session_id: Some(session_id.clone()),
                    message: None,
                    model: Some(model.clone()),
                    options: HashMap::new(),
                };

                match self.send_request(&mut s, &request).await {
                    Ok(response) if response.success => {
                        debug!("OpenCode session created: {:?}", response);
                        Some(s)
                    }
                    Ok(response) => {
                        warn!("OpenCode session creation failed: {:?}", response.error);
                        None
                    }
                    Err(e) => {
                        warn!("Failed to create OpenCode session: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("OpenCode not available: {}", e);
                None
            }
        };

        // Store session state
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            session_id.clone(),
            OpenCodeSession {
                stream,
                history: Vec::new(),
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
        // Get or create connection
        let mut sessions = self.sessions.lock().await;
        let session_state = sessions.get_mut(&session.id)
            .ok_or_else(|| Error::Harness("Session not found".to_string()))?;

        // Ensure we have a connection
        if session_state.stream.is_none() {
            let stream = self.connect().await?;
            session_state.stream = Some(stream);
        }

        let stream = session_state.stream.as_mut()
            .ok_or_else(|| Error::Harness("No connection to OpenCode".to_string()))?;

        // Record user message
        session_state.history.push(format!("user: {}", message));

        // Send message request
        let request = OpenCodeRequest {
            id: Uuid::new_v4().to_string(),
            request_type: "message.send".to_string(),
            session_id: Some(session.id.clone()),
            message: Some(message.to_string()),
            model: Some(session.model.clone()),
            options: HashMap::new(),
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| Error::Harness(format!("Failed to serialize request: {}", e)))?;

        stream
            .write_all(format!("{}\n", request_json).as_bytes())
            .await
            .map_err(|e| Error::Harness(format!("Failed to send message: {}", e)))?;

        stream
            .flush()
            .await
            .map_err(|e| Error::Harness(format!("Failed to flush: {}", e)))?;

        // We need to drop the lock before streaming
        drop(sessions);

        // Reconnect for streaming (OpenCode uses separate stream for responses)
        let mut response_stream = self.connect().await?;
        let chunks = self.stream_responses(&mut response_stream).await?;

        // Ensure done marker
        let mut chunks = chunks;
        if !chunks.iter().any(|c| matches!(c, ResponseChunk::Done)) {
            chunks.push(ResponseChunk::Done);
        }

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest> {
        match chunk {
            ResponseChunk::SubagentSpawn(req) => Some(req.clone()),
            ResponseChunk::ToolCall(tool) if self.is_subagent_tool(&tool.name) => {
                self.extract_subagent_request(&tool.arguments)
            }
            _ => None,
        }
    }

    async fn inject_result(
        &self,
        session: &SessionHandle,
        result: SubagentResult,
    ) -> Result<()> {
        debug!(
            "Injecting subagent result for OpenCode session {}: {}",
            result.session_id,
            if result.success { "success" } else { "failed" }
        );

        let mut sessions = self.sessions.lock().await;
        if let Some(session_state) = sessions.get_mut(&session.id) {
            session_state.history.push(format!(
                "subagent_result: {}",
                result.output
            ));

            // Send result to OpenCode if connected
            if let Some(stream) = session_state.stream.as_mut() {
                let request = OpenCodeRequest {
                    id: Uuid::new_v4().to_string(),
                    request_type: "tool.result".to_string(),
                    session_id: Some(session.id.clone()),
                    message: Some(result.output),
                    model: None,
                    options: {
                        let mut opts = HashMap::new();
                        opts.insert("tool_call_id".to_string(), serde_json::json!(result.session_id));
                        opts.insert("success".to_string(), serde_json::json!(result.success));
                        opts
                    },
                };

                let request_json = serde_json::to_string(&request)
                    .map_err(|e| Error::Harness(format!("Failed to serialize: {}", e)))?;

                stream
                    .write_all(format!("{}\n", request_json).as_bytes())
                    .await
                    .map_err(|e| Error::Harness(format!("Failed to send result: {}", e)))?;
            }
        }

        Ok(())
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        info!("Closing OpenCode session {}", session.id);

        let mut sessions = self.sessions.lock().await;
        if let Some(mut session_state) = sessions.remove(&session.id) {
            // Send close request if connected
            if let Some(mut stream) = session_state.stream.take() {
                let request = OpenCodeRequest {
                    id: Uuid::new_v4().to_string(),
                    request_type: "session.close".to_string(),
                    session_id: Some(session.id.clone()),
                    message: None,
                    model: None,
                    options: HashMap::new(),
                };

                if let Ok(json) = serde_json::to_string(&request) {
                    let _ = stream.write_all(format!("{}\n", json).as_bytes()).await;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_subagent_request() {
        let harness = OpenCodeHarness {
            socket_path: PathBuf::from("/tmp/test.sock"),
            model: "sonnet".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };

        let args = serde_json::json!({
            "prompt": "analyze the code",
            "category": "analyzer"
        });

        let req = harness.extract_subagent_request(&args).unwrap();
        assert_eq!(req.prompt, "analyze the code");
        assert_eq!(req.category, "analyzer");
    }

    #[test]
    fn test_is_subagent_tool() {
        let harness = OpenCodeHarness {
            socket_path: PathBuf::from("/tmp/test.sock"),
            model: "sonnet".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };

        assert!(harness.is_subagent_tool("Task"));
        assert!(harness.is_subagent_tool("delegate"));
        assert!(!harness.is_subagent_tool("bash"));
    }
}
