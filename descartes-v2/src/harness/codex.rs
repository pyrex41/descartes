//! Codex harness implementation
//!
//! Uses the OpenAI-compatible API directly for agent execution with
//! function calling support for tool use and subagent dispatch.

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult, ToolCall, ToolResult,
};
use crate::config::CodexConfig;
use crate::{Error, Result};

/// Codex harness using the OpenAI-compatible API
pub struct CodexHarness {
    /// API base URL
    api_base: String,
    /// API key
    api_key: String,
    /// HTTP client
    client: reqwest::Client,
    /// Default model
    model: String,
    /// Active sessions
    sessions: Arc<Mutex<HashMap<String, CodexSession>>>,
}

/// State for a Codex session
struct CodexSession {
    /// Conversation messages
    messages: Vec<ChatMessage>,
    /// Available tools
    tools: Vec<ToolDefinition>,
}

/// Chat message format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// Tool call from API response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

/// Tool definition for API
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Chat completion request
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// Streaming response chunk
#[derive(Debug, Deserialize)]
struct StreamChunk {
    id: Option<String>,
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    index: u32,
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct DeltaToolCall {
    index: u32,
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type", default)]
    call_type: Option<String>,
    #[serde(default)]
    function: Option<DeltaFunction>,
}

#[derive(Debug, Deserialize)]
struct DeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl CodexHarness {
    /// Create a new Codex harness
    pub fn new(config: &CodexConfig) -> Result<Self> {
        let api_base = config
            .api_base
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| Error::Config("Codex API key not configured".to_string()))?;

        // Build HTTP client with default headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| Error::Config(format!("Invalid API key format: {}", e)))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| Error::Harness(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            api_base,
            api_key,
            client,
            model: "gpt-4-turbo-preview".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create default tool definitions
    fn default_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "read_file".to_string(),
                    description: "Read the contents of a file".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to read"
                            }
                        },
                        "required": ["path"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to write to"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write"
                            }
                        },
                        "required": ["path", "content"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "run_command".to_string(),
                    description: "Run a shell command".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "Command to run"
                            }
                        },
                        "required": ["command"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "dispatch_agent".to_string(),
                    description: "Dispatch a subagent to handle a task".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "task": {
                                "type": "string",
                                "description": "Task for the subagent"
                            },
                            "agent_type": {
                                "type": "string",
                                "enum": ["searcher", "analyzer", "builder", "validator"],
                                "description": "Type of agent to dispatch"
                            }
                        },
                        "required": ["task", "agent_type"]
                    }),
                },
            },
        ]
    }

    /// Parse SSE stream data
    fn parse_sse_line(&self, line: &str) -> Option<StreamChunk> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(':') {
            return None;
        }

        // Parse data lines
        if let Some(data) = line.strip_prefix("data: ") {
            // Check for stream end
            if data == "[DONE]" {
                return None;
            }

            // Parse JSON
            match serde_json::from_str(data) {
                Ok(chunk) => Some(chunk),
                Err(e) => {
                    debug!("Failed to parse SSE data: {} - {}", e, data);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Check if a function is a subagent dispatch
    fn is_subagent_function(&self, name: &str) -> bool {
        matches!(
            name.to_lowercase().as_str(),
            "dispatch_agent" | "spawn_agent" | "delegate" | "task"
        )
    }

    /// Extract subagent request from function arguments
    fn extract_subagent_request(&self, name: &str, args_str: &str) -> Option<SubagentRequest> {
        let args: serde_json::Value = serde_json::from_str(args_str).ok()?;

        let prompt = args
            .get("task")
            .or_else(|| args.get("prompt"))
            .or_else(|| args.get("message"))
            .and_then(|p| p.as_str())?;

        let category = args
            .get("agent_type")
            .or_else(|| args.get("category"))
            .or_else(|| args.get("type"))
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
impl Harness for CodexHarness {
    fn name(&self) -> &str {
        "codex"
    }

    fn kind(&self) -> HarnessKind {
        HarnessKind::Codex
    }

    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle> {
        let session_id = Uuid::new_v4().to_string();

        let model = if config.model.is_empty() {
            self.model.clone()
        } else {
            config.model
        };

        info!(
            "Starting Codex session {} with model {}",
            session_id, model
        );

        // Initialize session with system prompt if provided
        let mut messages = Vec::new();
        if let Some(system_prompt) = config.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system_prompt),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Create session state
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            session_id.clone(),
            CodexSession {
                messages,
                tools: Self::default_tools(),
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
        let mut sessions = self.sessions.lock().await;
        let session_state = sessions.get_mut(&session.id)
            .ok_or_else(|| Error::Harness("Session not found".to_string()))?;

        // Add user message
        session_state.messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(message.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        // Build request
        let request = ChatCompletionRequest {
            model: session.model.clone(),
            messages: session_state.messages.clone(),
            tools: Some(session_state.tools.clone()),
            stream: true,
            max_tokens: Some(4096),
        };

        drop(sessions);

        // Send request
        let url = format!("{}/chat/completions", self.api_base);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Harness(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Harness(format!("API error {}: {}", status, body)));
        }

        // Process SSE stream
        let mut chunks = Vec::new();
        let mut current_tool_calls: HashMap<u32, (String, String, String)> = HashMap::new(); // index -> (id, name, args)
        let mut assistant_content = String::new();

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(item) = stream.next().await {
            let bytes = item.map_err(|e| Error::Harness(format!("Stream error: {}", e)))?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // Process complete lines
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(chunk) = self.parse_sse_line(&line) {
                    for choice in chunk.choices {
                        // Handle text content
                        if let Some(content) = choice.delta.content {
                            assistant_content.push_str(&content);
                            chunks.push(ResponseChunk::Text(content));
                        }

                        // Handle tool calls
                        if let Some(tool_calls) = choice.delta.tool_calls {
                            for tc in tool_calls {
                                let entry = current_tool_calls.entry(tc.index)
                                    .or_insert_with(|| (String::new(), String::new(), String::new()));

                                if let Some(id) = tc.id {
                                    entry.0 = id;
                                }
                                if let Some(func) = tc.function {
                                    if let Some(name) = func.name {
                                        entry.1 = name;
                                    }
                                    if let Some(args) = func.arguments {
                                        entry.2.push_str(&args);
                                    }
                                }
                            }
                        }

                        // Handle finish
                        if let Some(reason) = choice.finish_reason {
                            if reason == "tool_calls" {
                                // Emit accumulated tool calls
                                for (_, (id, name, args)) in current_tool_calls.drain() {
                                    if self.is_subagent_function(&name) {
                                        if let Some(req) = self.extract_subagent_request(&name, &args) {
                                            chunks.push(ResponseChunk::SubagentSpawn(req));
                                            continue;
                                        }
                                    }

                                    let arguments: serde_json::Value = serde_json::from_str(&args)
                                        .unwrap_or(serde_json::Value::Null);

                                    chunks.push(ResponseChunk::ToolCall(ToolCall {
                                        name,
                                        arguments,
                                        id,
                                    }));
                                }
                            } else if reason == "stop" {
                                chunks.push(ResponseChunk::Done);
                            }
                        }
                    }
                }
            }
        }

        // Record assistant message in session
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(state) = sessions.get_mut(&session.id) {
                if !assistant_content.is_empty() {
                    state.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: Some(assistant_content),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
        }

        // Ensure done marker
        if !chunks.iter().any(|c| matches!(c, ResponseChunk::Done)) {
            chunks.push(ResponseChunk::Done);
        }

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest> {
        match chunk {
            ResponseChunk::SubagentSpawn(req) => Some(req.clone()),
            ResponseChunk::ToolCall(tool) if self.is_subagent_function(&tool.name) => {
                let args_str = serde_json::to_string(&tool.arguments).unwrap_or_default();
                self.extract_subagent_request(&tool.name, &args_str)
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
            "Injecting subagent result for Codex session {}: {}",
            result.session_id,
            if result.success { "success" } else { "failed" }
        );

        let mut sessions = self.sessions.lock().await;
        if let Some(state) = sessions.get_mut(&session.id) {
            // Add tool result as a message
            state.messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(result.output),
                name: Some("dispatch_agent".to_string()),
                tool_calls: None,
                tool_call_id: Some(result.session_id),
            });
        }

        Ok(())
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        info!("Closing Codex session {}", session.id);

        let mut sessions = self.sessions.lock().await;
        sessions.remove(&session.id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_subagent_function() {
        // We can't easily test the full harness without API key
        // but we can test the helper functions
        let harness = CodexHarness {
            api_base: "http://localhost".to_string(),
            api_key: "test".to_string(),
            client: reqwest::Client::new(),
            model: "gpt-4".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };

        assert!(harness.is_subagent_function("dispatch_agent"));
        assert!(harness.is_subagent_function("DELEGATE"));
        assert!(!harness.is_subagent_function("read_file"));
    }

    #[test]
    fn test_extract_subagent_request() {
        let harness = CodexHarness {
            api_base: "http://localhost".to_string(),
            api_key: "test".to_string(),
            client: reqwest::Client::new(),
            model: "gpt-4".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };

        let args = r#"{"task": "find all tests", "agent_type": "searcher"}"#;
        let req = harness.extract_subagent_request("dispatch_agent", args).unwrap();

        assert_eq!(req.prompt, "find all tests");
        assert_eq!(req.category, "searcher");
    }

    #[test]
    fn test_parse_sse_line() {
        let harness = CodexHarness {
            api_base: "http://localhost".to_string(),
            api_key: "test".to_string(),
            client: reqwest::Client::new(),
            model: "gpt-4".to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };

        // Test DONE marker
        assert!(harness.parse_sse_line("data: [DONE]").is_none());

        // Test empty line
        assert!(harness.parse_sse_line("").is_none());

        // Test comment
        assert!(harness.parse_sse_line(": ping").is_none());
    }
}
