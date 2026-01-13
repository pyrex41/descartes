//! Transcript recording and playback
//!
//! All agent executions produce transcripts in SCG format for:
//! - Full visibility into what every agent/subagent did
//! - Replay and debugging
//! - Token-efficient storage

mod scg;

pub use scg::{parse_scg, ScgParser};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::harness::ResponseChunk;
use crate::{Config, Error, Result};

/// A complete transcript of an agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Unique transcript ID
    id: String,
    /// Harness that was used
    harness: String,
    /// Model that was used
    model: String,
    /// When the session started
    started: DateTime<Utc>,
    /// When the session ended
    ended: Option<DateTime<Utc>>,
    /// Parent transcript ID (for subagents)
    parent: Option<String>,
    /// Agent category
    category: Option<String>,
    /// All entries in the transcript
    entries: Vec<TranscriptEntry>,
    /// Subagent references
    subagents: Vec<SubagentRef>,
    /// Metrics
    metrics: TranscriptMetrics,
}

impl Transcript {
    /// Create a new transcript
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            harness: String::new(),
            model: String::new(),
            started: Utc::now(),
            ended: None,
            parent: None,
            category: None,
            entries: Vec::new(),
            subagents: Vec::new(),
            metrics: TranscriptMetrics::default(),
        }
    }

    /// Set the harness name
    pub fn with_harness(mut self, harness: &str) -> Self {
        self.harness = harness.to_string();
        self
    }

    /// Set the parent transcript ID
    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent = Some(parent.to_string());
        self
    }

    /// Set the agent category
    pub fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Get the transcript ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Record a user message
    pub fn record_user_message(&mut self, message: &str) {
        self.entries.push(TranscriptEntry::User(message.to_string()));
    }

    /// Record an assistant message
    pub fn record_assistant_message(&mut self, message: &str) {
        self.entries
            .push(TranscriptEntry::Assistant(message.to_string()));
    }

    /// Record a response chunk
    pub fn record_chunk(&mut self, chunk: &ResponseChunk) {
        match chunk {
            ResponseChunk::Text(text) => {
                // Append to last assistant entry or create new one
                if let Some(TranscriptEntry::Assistant(last)) = self.entries.last_mut() {
                    last.push_str(text);
                } else {
                    self.entries
                        .push(TranscriptEntry::Assistant(text.clone()));
                }
            }
            ResponseChunk::ToolCall(tool) => {
                self.entries.push(TranscriptEntry::ToolCall {
                    name: tool.name.clone(),
                    arguments: tool.arguments.clone(),
                    id: tool.id.clone(),
                });
                self.metrics.tools_called += 1;
            }
            ResponseChunk::ToolResult(result) => {
                self.entries.push(TranscriptEntry::ToolResult {
                    tool_call_id: result.tool_call_id.clone(),
                    content: result.content.clone(),
                    success: result.success,
                });
            }
            ResponseChunk::SubagentSpawn(req) => {
                self.entries.push(TranscriptEntry::SubagentSpawn {
                    category: req.category.clone(),
                    prompt: req.prompt.clone(),
                });
            }
            ResponseChunk::Error(e) => {
                self.entries.push(TranscriptEntry::Error(e.clone()));
            }
            ResponseChunk::Done => {
                // Finalize
            }
        }
    }

    /// Record a subagent execution
    pub fn record_subagent(&mut self, session_id: &str, category: &str, prompt: &str) {
        self.subagents.push(SubagentRef {
            session_id: session_id.to_string(),
            category: category.to_string(),
            prompt: prompt.to_string(),
            completed: false,
        });
    }

    /// Record subagent completion
    pub fn record_subagent_completion(&mut self, session_id: &str, success: bool) {
        if let Some(sub) = self
            .subagents
            .iter_mut()
            .find(|s| s.session_id == session_id)
        {
            sub.completed = success;
        }
    }

    /// Finalize the transcript
    pub fn finalize(&mut self) {
        self.ended = Some(Utc::now());
        if let (Some(end), start) = (self.ended, self.started) {
            self.metrics.duration_ms = (end - start).num_milliseconds() as u64;
        }
    }

    /// Convert to SCG format
    pub fn to_scg(&self) -> String {
        scg::to_scg(self)
    }

    /// Save transcript to file in SCG format
    pub fn save_scg(&self, path: &Path) -> Result<()> {
        let content = self.to_scg();
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}

/// An entry in a transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptEntry {
    /// User message
    User(String),
    /// Assistant message
    Assistant(String),
    /// Tool call
    ToolCall {
        name: String,
        arguments: serde_json::Value,
        id: String,
    },
    /// Tool result
    ToolResult {
        tool_call_id: String,
        content: String,
        success: bool,
    },
    /// Subagent spawn attempt
    SubagentSpawn { category: String, prompt: String },
    /// Error
    Error(String),
}

/// Reference to a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentRef {
    /// Session ID of the subagent
    pub session_id: String,
    /// Category of the subagent
    pub category: String,
    /// Prompt given to the subagent
    pub prompt: String,
    /// Whether the subagent completed successfully
    pub completed: bool,
}

/// Metrics about a transcript
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptMetrics {
    /// Tokens sent to model
    pub tokens_in: usize,
    /// Tokens received from model
    pub tokens_out: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of tool calls made
    pub tools_called: usize,
}

/// List transcripts in the transcript directory
pub fn list_transcripts(
    config: &Config,
    today_only: bool,
    session_filter: Option<String>,
) -> Result<Vec<String>> {
    let dir = config.transcript_dir();

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let today = Utc::now().format("%Y-%m-%d").to_string();

    let mut transcripts = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "scg").unwrap_or(false) {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // Apply filters
            if today_only && !name.contains(&today) {
                continue;
            }

            if let Some(ref filter) = session_filter {
                if !name.contains(filter) {
                    continue;
                }
            }

            transcripts.push(name);
        }
    }

    transcripts.sort();
    Ok(transcripts)
}

/// Load a transcript by ID
pub fn load(config: &Config, session_id: &str) -> Result<Transcript> {
    let path = config.transcript_dir().join(format!("{}.scg", session_id));

    if !path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Transcript not found: {}", session_id),
        )));
    }

    let content = std::fs::read_to_string(&path)?;
    parse_scg(&content)
}

/// Replay a transcript with timing
pub async fn replay(transcript: &Transcript, speed: f32) -> Result<()> {
    use tokio::time::{sleep, Duration};

    println!("=== Replaying transcript {} ===", transcript.id);
    println!("Started: {}", transcript.started);
    println!("Harness: {}", transcript.harness);
    println!();

    for entry in &transcript.entries {
        match entry {
            TranscriptEntry::User(msg) => {
                println!("\x1b[34m[USER]\x1b[0m {}", msg);
            }
            TranscriptEntry::Assistant(msg) => {
                // Simulate typing
                for c in msg.chars() {
                    print!("{}", c);
                    let delay = (10.0 / speed) as u64;
                    sleep(Duration::from_millis(delay)).await;
                }
                println!();
            }
            TranscriptEntry::ToolCall { name, id, .. } => {
                println!("\x1b[33m[TOOL]\x1b[0m {} ({})", name, id);
            }
            TranscriptEntry::ToolResult { content, success, .. } => {
                let status = if *success { "OK" } else { "FAIL" };
                println!("\x1b[32m[{}]\x1b[0m {}", status, truncate(content, 100));
            }
            TranscriptEntry::SubagentSpawn { category, prompt } => {
                println!(
                    "\x1b[35m[SUBAGENT]\x1b[0m {} - {}",
                    category,
                    truncate(prompt, 50)
                );
            }
            TranscriptEntry::Error(e) => {
                println!("\x1b[31m[ERROR]\x1b[0m {}", e);
            }
        }

        // Pause between entries
        let delay = (500.0 / speed) as u64;
        sleep(Duration::from_millis(delay)).await;
    }

    println!();
    println!("=== Replay complete ===");
    println!("Duration: {}ms", transcript.metrics.duration_ms);
    println!("Tool calls: {}", transcript.metrics.tools_called);

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
