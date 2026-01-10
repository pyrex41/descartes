//! Claude Code harness implementation
//!
//! Runs Claude Code CLI in headless mode, intercepting tool calls
//! and subagent spawns.

use async_trait::async_trait;
use futures::stream;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult, ToolCall,
};
use crate::config::ClaudeCodeConfig;
use crate::{Error, Result};

/// Claude Code harness using the CLI in headless/print mode
pub struct ClaudeCodeHarness {
    /// Path to claude binary
    binary: String,
    /// Default model
    model: String,
    /// Whether to use headless mode
    headless: bool,
    /// Skip permission prompts
    skip_permissions: bool,
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
            headless: config.headless,
            skip_permissions: config.dangerously_skip_permissions,
        })
    }

    /// Build command arguments
    fn build_args(&self, session: &SessionHandle, message: &str) -> Vec<String> {
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

        // Verbose for debugging
        args.push("--verbose".to_string());

        args
    }

    /// Parse a JSON line from Claude Code output
    fn parse_output_line(&self, line: &str) -> Option<ResponseChunk> {
        // Claude Code stream-json format outputs JSON objects, one per line
        let json: serde_json::Value = serde_json::from_str(line).ok()?;

        // Check for different message types
        if let Some(msg_type) = json.get("type").and_then(|t| t.as_str()) {
            match msg_type {
                "text" | "content" => {
                    if let Some(text) = json.get("content").and_then(|c| c.as_str()) {
                        return Some(ResponseChunk::Text(text.to_string()));
                    }
                }
                "tool_use" | "tool_call" => {
                    if let (Some(name), Some(args)) = (
                        json.get("name").and_then(|n| n.as_str()),
                        json.get("input").or_else(|| json.get("arguments")),
                    ) {
                        let id = json
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("")
                            .to_string();

                        // Check for subagent spawn patterns
                        if self.is_subagent_tool(name) {
                            if let Some(req) = self.extract_subagent_request(name, args) {
                                return Some(ResponseChunk::SubagentSpawn(req));
                            }
                        }

                        return Some(ResponseChunk::ToolCall(ToolCall {
                            name: name.to_string(),
                            arguments: args.clone(),
                            id,
                        }));
                    }
                }
                "tool_result" => {
                    // Tool result handling
                }
                "error" => {
                    if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
                        return Some(ResponseChunk::Error(msg.to_string()));
                    }
                }
                "result" | "end" => {
                    return Some(ResponseChunk::Done);
                }
                _ => {
                    debug!("Unknown message type: {}", msg_type);
                }
            }
        }

        None
    }

    /// Check if a tool call is a subagent spawn
    fn is_subagent_tool(&self, name: &str) -> bool {
        matches!(
            name.to_lowercase().as_str(),
            "task" | "spawn" | "subagent" | "agent" | "dispatch"
        )
    }

    /// Extract subagent request from tool call
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

        Ok(SessionHandle {
            id: session_id,
            harness: self.name().to_string(),
            model,
            parent: config.parent.map(|p| p.id),
        })
    }

    async fn send(&self, session: &SessionHandle, message: &str) -> Result<ResponseStream> {
        let args = self.build_args(session, message);

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
        let mut lines = reader.lines();

        // Collect all chunks (in a real impl, this would be a proper async stream)
        let mut chunks = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(chunk) = self.parse_output_line(&line) {
                chunks.push(chunk);
            }
        }

        // Wait for process to complete
        let status = child.wait().await?;
        if !status.success() {
            warn!("Claude process exited with status: {}", status);
        }

        // Add done marker
        chunks.push(ResponseChunk::Done);

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
        _session: &SessionHandle,
        result: SubagentResult,
    ) -> Result<()> {
        // For Claude Code, we inject results by continuing the conversation
        // with the result as a tool result or assistant message.
        // In practice, this means the next send() call should include the context.
        debug!(
            "Injecting subagent result for session {}: {}",
            result.session_id,
            if result.success { "success" } else { "failed" }
        );

        // The actual injection happens through conversation state management
        // which we handle at a higher level in the loop
        Ok(())
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        info!("Closing Claude Code session {}", session.id);
        // Claude Code CLI sessions are stateless, nothing to close
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subagent_request() {
        let harness = ClaudeCodeHarness {
            binary: "claude".to_string(),
            model: "sonnet".to_string(),
            headless: true,
            skip_permissions: false,
        };

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
}
