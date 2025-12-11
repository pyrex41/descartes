//! Claude Code CLI backend implementation
//!
//! Spawns and manages Claude CLI processes with stream-json output format,
//! parsing the NDJSON stream into StreamChunk messages.

use crate::cli_backend::{ChatSessionConfig, ChatSessionHandle, CliBackend, StreamChunk};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Claude Code stream-json message types
/// These match the NDJSON format output by `claude --output-format stream-json`
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum ClaudeStreamMessage {
    #[serde(rename = "system")]
    System { subtype: String },
    #[serde(rename = "assistant")]
    Assistant { message: AssistantMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_start")]
    MessageStart { message: MessageInfo },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaInfo },
    #[serde(rename = "message_stop")]
    MessageStop {},
    #[serde(rename = "result")]
    Result { subtype: String, result: ResultInfo },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AssistantMessage {
    content: Option<Vec<ContentItem>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentItem {
    #[serde(rename = "type")]
    item_type: String,
    text: Option<String>,
    thinking: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
    thinking: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessageInfo {
    id: Option<String>,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessageDeltaInfo {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResultInfo {
    #[serde(default)]
    cost_usd: Option<f64>,
    #[serde(default)]
    duration_ms: Option<u64>,
}

/// Active chat session
struct ActiveSession {
    #[allow(dead_code)]
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
}

/// Claude Code backend
pub struct ClaudeBackend {
    sessions: Arc<DashMap<Uuid, ActiveSession>>,
}

impl ClaudeBackend {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    fn build_command(&self, config: &ChatSessionConfig) -> Command {
        let mut cmd = Command::new("claude");

        // Print mode is required for --output-format to work
        cmd.arg("-p");

        // Streaming JSON output
        cmd.arg("--output-format").arg("stream-json");

        // Thinking via trigger words in prompt prefix is more reliable than --betas
        // The prompt will include thinking instructions based on thinking_level

        // Max turns
        if config.max_turns > 0 {
            cmd.arg("--max-turns").arg(config.max_turns.to_string());
        }

        // Extra flags
        for flag in &config.extra_flags {
            cmd.arg(flag);
        }

        // Add initial prompt as command-line argument
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

        // I/O setup - only need stdout for streaming output
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd
    }

    fn thinking_prefix(level: &str) -> &'static str {
        match level {
            "hard" => "think hard: ",
            "harder" => "think harder: ",
            "ultra" => "ultrathink: ",
            _ => "", // "normal" - no special prefix, thinking happens naturally
        }
    }
}

impl Default for ClaudeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CliBackend for ClaudeBackend {
    fn name(&self) -> &str {
        "claude"
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
        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        // Placeholder channel for API compatibility (single-turn mode)
        let (stdin_tx, _stdin_rx) = mpsc::unbounded_channel::<String>();

        // Stdout parsing task - reads stream-json output from claude
        // Claude CLI outputs NDJSON with these message types:
        // - system: init info with tools, session_id, etc.
        // - assistant: the response with message.content[].text
        // - result: completion status with cost, duration, etc.
        let sessions = self.sessions.clone();
        let sid = session_id;
        let stream_tx_clone = stream_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                // Try to parse as stream-json
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    let msg_type = msg.get("type").and_then(|t| t.as_str());
                    tracing::debug!("Parsed Claude CLI message type: {:?}", msg_type);

                    match msg_type {
                        // Claude CLI assistant message - contains the full response
                        Some("assistant") => {
                            if let Some(message) = msg.get("message") {
                                if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                                    for item in content {
                                        let item_type = item.get("type").and_then(|t| t.as_str());
                                        match item_type {
                                            Some("thinking") => {
                                                if let Some(thinking) = item.get("thinking").and_then(|t| t.as_str()) {
                                                    let _ = stream_tx_clone.send(StreamChunk::Thinking {
                                                        content: thinking.to_string(),
                                                    });
                                                }
                                            }
                                            Some("text") => {
                                                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                                    let _ = stream_tx_clone.send(StreamChunk::Text {
                                                        content: text.to_string(),
                                                    });
                                                }
                                            }
                                            Some("tool_use") => {
                                                let tool_name = item.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                                                let tool_id = item.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                                let _ = stream_tx_clone.send(StreamChunk::ToolUseStart {
                                                    tool_name: tool_name.to_string(),
                                                    tool_id: tool_id.to_string(),
                                                });
                                                if let Some(input) = item.get("input") {
                                                    let _ = stream_tx_clone.send(StreamChunk::ToolUseInput {
                                                        tool_id: tool_id.to_string(),
                                                        input: input.clone(),
                                                    });
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            // Mark turn complete after processing assistant message
                            let _ = stream_tx_clone.send(StreamChunk::TurnComplete { turn_number: 0 });
                        }
                        // Result message indicates end of session
                        Some("result") => {
                            let is_error = msg.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                            if is_error {
                                let error_msg = msg.get("error").and_then(|e| e.as_str()).unwrap_or("Unknown error");
                                let _ = stream_tx_clone.send(StreamChunk::Error {
                                    message: error_msg.to_string(),
                                });
                            }
                        }
                        // System message - just for init, ignore
                        Some("system") => {}
                        // API-style streaming (if claude ever outputs this format)
                        Some("content_block_delta") => {
                            if let Some(delta) = msg.get("delta") {
                                if let Some(thinking) = delta.get("thinking").and_then(|t| t.as_str()) {
                                    let _ = stream_tx_clone.send(StreamChunk::Thinking {
                                        content: thinking.to_string(),
                                    });
                                } else if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                    let _ = stream_tx_clone.send(StreamChunk::Text {
                                        content: text.to_string(),
                                    });
                                }
                            }
                        }
                        Some("message_stop") => {
                            let _ = stream_tx_clone.send(StreamChunk::TurnComplete { turn_number: 0 });
                        }
                        _ => {}
                    }
                }
            }

            // Process exited
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

        session
            .stdin_tx
            .send(prompt)
            .map_err(|e| format!("Failed to send prompt: {}", e))
    }

    async fn stop_session(&self, session_id: Uuid) -> Result<(), String> {
        if let Some((_, mut session)) = self.sessions.remove(&session_id) {
            // Send SIGTERM
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinking_prefix() {
        assert_eq!(ClaudeBackend::thinking_prefix("normal"), "");
        assert_eq!(ClaudeBackend::thinking_prefix("hard"), "think hard: ");
        assert_eq!(ClaudeBackend::thinking_prefix("harder"), "think harder: ");
        assert_eq!(ClaudeBackend::thinking_prefix("ultra"), "ultrathink: ");
    }

    #[tokio::test]
    async fn test_claude_backend_availability() {
        let backend = ClaudeBackend::new();
        // This will return true if `claude` is in PATH, false otherwise
        // We don't assert the value since it depends on the test environment
        let _ = backend.is_available().await;
    }

    #[test]
    fn test_stream_message_parsing() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.get("type").and_then(|t| t.as_str()),
            Some("content_block_delta")
        );
    }
}
