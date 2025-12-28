//! Intercepting Claude Backend
//!
//! A Claude Code backend that intercepts specific tool calls and executes them
//! through Descartes, enabling sub-agent management and visibility.
//!
//! ## Architecture
//!
//! This backend uses `--input-format stream-json` for bidirectional communication
//! with Claude Code. The message format is:
//! ```json
//! {"type":"user","message":{"role":"user","content":"..."},"session_id":"..","parent_tool_use_id":null}
//! ```
//!
//! ## Interception Strategy
//!
//! **Option 1: MCP Server (Recommended)**
//! Run Descartes as an MCP server providing `spawn_agent` tool. Claude Code connects
//! to it, and when Claude calls `spawn_agent`, Descartes receives it via MCP protocol,
//! spawns the subagent, and returns the result. This is the cleanest approach.
//!
//! **Option 2: Pseudo-tool (Experimental)**
//! Disable all tools via `--tools "" --mcp-config /dev/null`, describe a pseudo-tool
//! in the system prompt, and parse Claude's text output for tool call patterns.
//! This requires manual parsing and is less reliable.
//!
//! ## Current Implementation
//!
//! This module implements the infrastructure for both approaches:
//! - Stream-JSON bidirectional communication
//! - Tool call detection and callback mechanism
//! - Tool result injection via stdin

use crate::cli_backend::{ChatSessionConfig, ChatSessionHandle, CliBackend, StreamChunk};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Tool call that was intercepted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedToolCall {
    pub tool_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
}

/// Result to send back after intercepting a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInterceptResult {
    pub tool_id: String,
    pub output: String,
    pub is_error: bool,
}

/// Callback for handling intercepted tool calls
pub type ToolInterceptCallback =
    Arc<dyn Fn(InterceptedToolCall) -> oneshot::Receiver<ToolInterceptResult> + Send + Sync>;

/// Configuration for tool interception
#[derive(Clone)]
pub struct InterceptConfig {
    /// Tools to intercept (by name)
    pub intercept_tools: Vec<String>,
    /// Callback when a tool is intercepted
    pub on_intercept: ToolInterceptCallback,
    /// System prompt addition describing the intercepted tools
    pub tool_descriptions: String,
}

impl std::fmt::Debug for InterceptConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterceptConfig")
            .field("intercept_tools", &self.intercept_tools)
            .field("tool_descriptions", &self.tool_descriptions)
            .finish()
    }
}

/// Active session with stdin writer
struct ActiveSession {
    #[allow(dead_code)]
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
}

/// Intercepting Claude Backend
///
/// Unlike the standard ClaudeBackend, this one:
/// - Uses `--input-format stream-json` for bidirectional communication
/// - Intercepts specified tool calls and executes them through Descartes
/// - Injects tool results back into Claude Code via stdin
pub struct InterceptingClaudeBackend {
    sessions: Arc<DashMap<Uuid, ActiveSession>>,
    intercept_config: Option<InterceptConfig>,
}

impl InterceptingClaudeBackend {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            intercept_config: None,
        }
    }

    /// Create with tool interception enabled
    pub fn with_interception(config: InterceptConfig) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            intercept_config: Some(config),
        }
    }

    fn build_command(&self, config: &ChatSessionConfig) -> Command {
        let mut cmd = Command::new("claude");

        // Enable bidirectional streaming JSON
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--input-format").arg("stream-json");

        // Max turns - we manage the conversation loop
        if config.max_turns > 0 {
            cmd.arg("--max-turns").arg(config.max_turns.to_string());
        }

        // Append system prompt with tool descriptions if intercepting
        if let Some(ref intercept) = self.intercept_config {
            if !intercept.tool_descriptions.is_empty() {
                cmd.arg("--append-system-prompt")
                    .arg(&intercept.tool_descriptions);
            }
        }

        // Extra flags
        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        // Add initial prompt
        if !config.initial_prompt.is_empty() {
            let thinking_prefix = Self::thinking_prefix(&config.thinking_level);
            let full_prompt = if config.enable_thinking && !thinking_prefix.is_empty() {
                format!("{}{}", thinking_prefix, config.initial_prompt)
            } else {
                config.initial_prompt.clone()
            };
            cmd.arg(&full_prompt);
        }

        // Working directory
        cmd.current_dir(&config.working_dir);

        // I/O setup - need stdin for injecting tool results
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd
    }

    fn thinking_prefix(level: &str) -> &'static str {
        match level {
            "hard" => "think hard: ",
            "harder" => "think harder: ",
            "ultra" => "ultrathink: ",
            _ => "",
        }
    }

    /// Check if a tool should be intercepted
    /// Note: Currently the check is done inline in the async block to avoid capturing self
    #[allow(dead_code)]
    fn should_intercept(&self, tool_name: &str) -> bool {
        if let Some(ref config) = self.intercept_config {
            config.intercept_tools.contains(&tool_name.to_string())
        } else {
            false
        }
    }

    /// Format a tool result as stream-json for injection
    ///
    /// The format matches Claude Code's expected input:
    /// ```json
    /// {"type":"user","message":{"role":"user","content":[{"tool_use_id":"...","type":"tool_result","content":"...","is_error":false}]},"session_id":"default","parent_tool_use_id":null}
    /// ```
    fn format_tool_result(tool_id: &str, output: &str, is_error: bool, session_id: &str) -> String {
        let msg = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [{
                    "tool_use_id": tool_id,
                    "type": "tool_result",
                    "content": output,
                    "is_error": is_error
                }]
            },
            "session_id": session_id,
            "parent_tool_use_id": null
        });
        format!("{}\n", msg)
    }
}

impl Default for InterceptingClaudeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CliBackend for InterceptingClaudeBackend {
    fn name(&self) -> &str {
        "claude-intercepting"
    }

    async fn is_available(&self) -> bool {
        which::which("claude").is_ok()
    }

    async fn version(&self) -> Result<String, String> {
        let output = Command::new("claude")
            .arg("--version")
            .output()
            .await
            .map_err(|e| e.to_string())?;

        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_string())
            .map_err(|e| e.to_string())
    }

    async fn start_session(
        &self,
        config: ChatSessionConfig,
    ) -> Result<ChatSessionHandle, String> {
        let session_id = Uuid::new_v4();
        let mut cmd = self.build_command(&config);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn claude: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let mut stdin = child.stdin.take().ok_or("Failed to capture stdin")?;

        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();

        // Stdin writer task - sends messages to Claude Code
        tokio::spawn(async move {
            while let Some(msg) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(msg.as_bytes()).await {
                    tracing::error!("Failed to write to stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    tracing::error!("Failed to flush stdin: {}", e);
                    break;
                }
            }
        });

        // Clone what we need for the stdout parsing task
        let sessions = self.sessions.clone();
        let sid = session_id;
        let stream_tx_clone = stream_tx.clone();
        let stdin_tx_clone = stdin_tx.clone();
        let intercept_config = self.intercept_config.clone();

        // Stdout parsing task
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            // Track Claude's session ID for tool result injection
            let mut claude_session_id = String::from("default");
            // Track pending Task tool calls to extract subagent_type when result arrives
            let mut pending_task_calls: std::collections::HashMap<String, serde_json::Value> =
                std::collections::HashMap::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    let msg_type = msg.get("type").and_then(|t| t.as_str());
                    tracing::debug!("Intercepting backend received: {:?}", msg_type);

                    match msg_type {
                        // Capture session_id from init message
                        Some("system") => {
                            if let Some(sid) = msg.get("session_id").and_then(|s| s.as_str()) {
                                claude_session_id = sid.to_string();
                                tracing::info!("Claude session ID: {}", claude_session_id);
                            }
                        }
                        Some("assistant") => {
                            if let Some(message) = msg.get("message") {
                                if let Some(content) =
                                    message.get("content").and_then(|c| c.as_array())
                                {
                                    for item in content {
                                        let item_type = item.get("type").and_then(|t| t.as_str());
                                        match item_type {
                                            Some("thinking") => {
                                                if let Some(thinking) =
                                                    item.get("thinking").and_then(|t| t.as_str())
                                                {
                                                    let _ =
                                                        stream_tx_clone.send(StreamChunk::Thinking {
                                                            content: thinking.to_string(),
                                                        });
                                                }
                                            }
                                            Some("text") => {
                                                if let Some(text) =
                                                    item.get("text").and_then(|t| t.as_str())
                                                {
                                                    let _ = stream_tx_clone.send(StreamChunk::Text {
                                                        content: text.to_string(),
                                                    });
                                                }
                                            }
                                            Some("tool_use") => {
                                                let tool_name = item
                                                    .get("name")
                                                    .and_then(|n| n.as_str())
                                                    .unwrap_or("unknown");
                                                let tool_id = item
                                                    .get("id")
                                                    .and_then(|i| i.as_str())
                                                    .unwrap_or("");
                                                let input = item
                                                    .get("input")
                                                    .cloned()
                                                    .unwrap_or(json!({}));

                                                // Track Task tool calls for subagent_type extraction
                                                if tool_name == "Task" {
                                                    pending_task_calls.insert(
                                                        tool_id.to_string(),
                                                        input.clone(),
                                                    );
                                                }

                                                // Check if we should intercept this tool
                                                let should_intercept = intercept_config
                                                    .as_ref()
                                                    .map(|c| {
                                                        c.intercept_tools
                                                            .contains(&tool_name.to_string())
                                                    })
                                                    .unwrap_or(false);

                                                // Always emit the tool use start
                                                let _ =
                                                    stream_tx_clone.send(StreamChunk::ToolUseStart {
                                                        tool_name: tool_name.to_string(),
                                                        tool_id: tool_id.to_string(),
                                                    });
                                                let _ =
                                                    stream_tx_clone.send(StreamChunk::ToolUseInput {
                                                        tool_id: tool_id.to_string(),
                                                        input: input.clone(),
                                                    });

                                                if should_intercept {
                                                    tracing::info!(
                                                        "Intercepting tool call: {} ({})",
                                                        tool_name,
                                                        tool_id
                                                    );

                                                    // Call the intercept callback
                                                    if let Some(ref config) = intercept_config {
                                                        let call = InterceptedToolCall {
                                                            tool_id: tool_id.to_string(),
                                                            tool_name: tool_name.to_string(),
                                                            input,
                                                        };

                                                        let result_rx = (config.on_intercept)(call);

                                                        // Wait for the result and inject it
                                                        let stdin_tx = stdin_tx_clone.clone();
                                                        let stream_tx = stream_tx_clone.clone();
                                                        let tid = tool_id.to_string();
                                                        let session_for_result = claude_session_id.clone();

                                                        tokio::spawn(async move {
                                                            match result_rx.await {
                                                                Ok(result) => {
                                                                    // Emit tool result to stream
                                                                    let _ = stream_tx.send(
                                                                        StreamChunk::ToolResult {
                                                                            tool_id: result
                                                                                .tool_id
                                                                                .clone(),
                                                                            result: result
                                                                                .output
                                                                                .clone(),
                                                                            is_error: result
                                                                                .is_error,
                                                                        },
                                                                    );

                                                                    // Inject result back to Claude
                                                                    let json_result =
                                                                        InterceptingClaudeBackend::format_tool_result(
                                                                            &result.tool_id,
                                                                            &result.output,
                                                                            result.is_error,
                                                                            &session_for_result,
                                                                        );
                                                                    let _ =
                                                                        stdin_tx.send(json_result);
                                                                }
                                                                Err(_) => {
                                                                    tracing::error!(
                                                                        "Intercept callback dropped for tool {}",
                                                                        tid
                                                                    );
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            let _ =
                                stream_tx_clone.send(StreamChunk::TurnComplete { turn_number: 0 });
                        }
                        Some("result") => {
                            let is_error =
                                msg.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                            if is_error {
                                let error_msg = msg
                                    .get("error")
                                    .and_then(|e| e.as_str())
                                    .unwrap_or("Unknown error");
                                let _ = stream_tx_clone.send(StreamChunk::Error {
                                    message: error_msg.to_string(),
                                });
                            }
                        }
                        // Detect Task tool completions (sub-agent spawns)
                        // Format: {"type":"user","message":...,"tool_use_result":{"agentId":"...",...}}
                        Some("user") => {
                            if let Some(tool_result) = msg.get("tool_use_result") {
                                if let Some(agent_id) = tool_result.get("agentId").and_then(|a| a.as_str()) {
                                    // This is a Task tool completion - a sub-agent was spawned
                                    let prompt = tool_result.get("prompt")
                                        .and_then(|p| p.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    // Get parent tool_use_id from the message content
                                    let parent_tool_id = msg.get("message")
                                        .and_then(|m| m.get("content"))
                                        .and_then(|c| c.as_array())
                                        .and_then(|arr| arr.first())
                                        .and_then(|item| item.get("tool_use_id"))
                                        .and_then(|id| id.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    // Extract subagent_type from the tracked Task tool call
                                    let subagent_type = pending_task_calls
                                        .remove(&parent_tool_id)
                                        .and_then(|input| {
                                            input.get("subagent_type")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        });

                                    tracing::info!(
                                        "Sub-agent spawned: {} (type: {:?}, prompt: {})",
                                        agent_id,
                                        subagent_type,
                                        if prompt.len() > 50 { &prompt[..50] } else { &prompt }
                                    );

                                    let _ = stream_tx_clone.send(StreamChunk::SubAgentSpawned {
                                        agent_id: agent_id.to_string(),
                                        session_id: claude_session_id.clone(),
                                        prompt,
                                        subagent_type,
                                        parent_tool_id,
                                    });
                                }
                            }
                        }
                        // Note: Some("system") is handled above to capture session_id
                        Some("content_block_delta") => {
                            if let Some(delta) = msg.get("delta") {
                                if let Some(thinking) =
                                    delta.get("thinking").and_then(|t| t.as_str())
                                {
                                    let _ = stream_tx_clone.send(StreamChunk::Thinking {
                                        content: thinking.to_string(),
                                    });
                                } else if let Some(text) = delta.get("text").and_then(|t| t.as_str())
                                {
                                    let _ = stream_tx_clone.send(StreamChunk::Text {
                                        content: text.to_string(),
                                    });
                                }
                            }
                        }
                        Some("message_stop") => {
                            let _ =
                                stream_tx_clone.send(StreamChunk::TurnComplete { turn_number: 0 });
                        }
                        _ => {}
                    }
                }
            }

            let _ = stream_tx_clone.send(StreamChunk::Complete { exit_code: 0 });
            sessions.remove(&sid);
        });

        // Store session
        self.sessions.insert(
            session_id,
            ActiveSession {
                child,
                stdin_tx: stdin_tx.clone(),
            },
        );

        Ok(ChatSessionHandle {
            session_id,
            stream_rx,
        })
    }

    async fn send_prompt(&self, session_id: Uuid, prompt: String) -> Result<(), String> {
        let session = self.sessions.get(&session_id).ok_or("Session not found")?;

        // Format as user message in stream-json format
        let msg = json!({
            "type": "user",
            "content": prompt
        });

        session
            .stdin_tx
            .send(format!("{}\n", msg))
            .map_err(|e| format!("Failed to send prompt: {}", e))
    }

    async fn stop_session(&self, session_id: Uuid) -> Result<(), String> {
        if let Some((_, mut session)) = self.sessions.remove(&session_id) {
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                if let Some(pid) = session.child.id() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                }
            }
            let _ = session.child.wait().await;
        }
        Ok(())
    }

    async fn kill_session(&self, session_id: Uuid) -> Result<(), String> {
        if let Some((_, mut session)) = self.sessions.remove(&session_id) {
            let _ = session.child.kill().await;
        }
        Ok(())
    }

    fn is_session_active(&self, session_id: Uuid) -> bool {
        self.sessions.contains_key(&session_id)
    }
}

/// Create a spawn_agent tool description for the system prompt
pub fn spawn_agent_tool_description() -> String {
    r#"
You have access to a special tool called `spawn_agent` that allows you to delegate tasks to sub-agents.
These sub-agents run as separate processes managed by Descartes and their progress is visible in the agent graph.

To use this tool, output a tool_use block like this:
```
<tool_use>
<name>spawn_agent</name>
<input>
{
  "task": "The task description for the sub-agent",
  "tool_level": "minimal|researcher|planner",
  "model": "optional model override",
  "context": "optional additional context"
}
</input>
</tool_use>
```

The sub-agent will execute the task and return its result, which you can then use to continue your work.
Use sub-agents for:
- Research tasks that need focused investigation
- Planning subtasks that need detailed analysis
- Any task that would benefit from parallel execution

Do NOT spawn sub-agents for simple tasks you can do directly.
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_formatting() {
        let result = InterceptingClaudeBackend::format_tool_result(
            "tool_123",
            "Task completed successfully",
            false,
            "test-session-id",
        );
        let parsed: serde_json::Value = serde_json::from_str(&result.trim()).unwrap();
        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["session_id"], "test-session-id");
        let content = &parsed["message"]["content"][0];
        assert_eq!(content["type"], "tool_result");
        assert_eq!(content["tool_use_id"], "tool_123");
        assert_eq!(content["content"], "Task completed successfully");
        assert_eq!(content["is_error"], false);
    }

    #[test]
    fn test_spawn_agent_description() {
        let desc = spawn_agent_tool_description();
        assert!(desc.contains("spawn_agent"));
        assert!(desc.contains("tool_level"));
        assert!(desc.contains("sub-agents"));
    }

    #[tokio::test]
    async fn test_intercepting_backend_availability() {
        let backend = InterceptingClaudeBackend::new();
        // Just verify it compiles and can check availability
        let _ = backend.is_available().await;
    }

    #[test]
    fn test_subagent_spawned_stream_chunk() {
        // Test that SubAgentSpawned chunk is properly constructed
        let chunk = StreamChunk::SubAgentSpawned {
            agent_id: "a9a57a7".to_string(),
            session_id: "7d487776-214f-47e8-83a1-94b65d05821b".to_string(),
            prompt: "What is 2+2?".to_string(),
            subagent_type: Some("general-purpose".to_string()),
            parent_tool_id: "toolu_014bmYNjTN754JKMTVXd9ijG".to_string(),
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"type\":\"sub_agent_spawned\""));
        assert!(json.contains("\"agent_id\":\"a9a57a7\""));
        assert!(json.contains("\"session_id\":\"7d487776-214f-47e8-83a1-94b65d05821b\""));
        assert!(json.contains("\"subagent_type\":\"general-purpose\""));

        // Verify deserialization
        let deserialized: StreamChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(chunk, deserialized);
    }

    #[test]
    fn test_subagent_detection_from_stream_json() {
        // Simulate the stream-json output format from Claude Code's Task tool
        let stream_json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": [{
                    "tool_use_id": "toolu_014bmYNjTN754JKMTVXd9ijG",
                    "type": "tool_result",
                    "content": ["2 + 2 = 4"]
                }]
            },
            "tool_use_result": {
                "status": "completed",
                "prompt": "What is 2+2?",
                "agentId": "a9a57a7",
                "totalDurationMs": 2710,
                "totalTokens": 40348
            },
            "session_id": "7d487776-214f-47e8-83a1-94b65d05821b"
        }"#;

        let msg: serde_json::Value = serde_json::from_str(stream_json).unwrap();

        // Verify we can extract the relevant fields
        let msg_type = msg.get("type").and_then(|t| t.as_str());
        assert_eq!(msg_type, Some("user"));

        let tool_result = msg.get("tool_use_result").unwrap();
        let agent_id = tool_result.get("agentId").and_then(|a| a.as_str());
        assert_eq!(agent_id, Some("a9a57a7"));

        let prompt = tool_result.get("prompt").and_then(|p| p.as_str());
        assert_eq!(prompt, Some("What is 2+2?"));

        let parent_tool_id = msg.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("tool_use_id"))
            .and_then(|id| id.as_str());
        assert_eq!(parent_tool_id, Some("toolu_014bmYNjTN754JKMTVXd9ijG"));
    }
}
